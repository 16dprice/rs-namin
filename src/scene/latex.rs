use std::collections::HashMap;

use macroquad::prelude::*;

use super::bezier::{BezierContour, CubicBezier, GlyphOutline};

/// TeX default font size in points. Used to normalize SVG coordinates
/// so that 1 em ≈ 1 world unit, matching font-based VectorText.
const PT_PER_EM: f32 = 10.0;

/// Compile a LaTeX expression and return glyph outlines ready for VectorText.
///
/// Shells out to `latex` and `dvisvgm` (must be installed).
/// The returned glyphs are normalized so 1 em ≈ 1 world unit.
pub fn latex_to_glyphs(latex: &str) -> Result<Vec<GlyphOutline>, String> {
    let svg = compile_to_svg(latex)?;
    Ok(parse_dvisvgm_svg(&svg))
}

// ── LaTeX compilation ────────────────────────────────────────────────

fn compile_to_svg(latex: &str) -> Result<String, String> {
    use std::sync::atomic::{AtomicU64, Ordering};
    static COUNTER: AtomicU64 = AtomicU64::new(0);
    let id = COUNTER.fetch_add(1, Ordering::Relaxed);
    let tmp_dir = std::env::temp_dir().join(format!("rs_namin_latex_{}_{id}", std::process::id()));
    std::fs::create_dir_all(&tmp_dir).map_err(|e| format!("failed to create temp dir: {e}"))?;

    let result = compile_to_svg_inner(latex, &tmp_dir);
    let _ = std::fs::remove_dir_all(&tmp_dir);
    result
}

fn compile_to_svg_inner(latex: &str, tmp_dir: &std::path::Path) -> Result<String, String> {
    let tex_path = tmp_dir.join("input.tex");
    let dvi_path = tmp_dir.join("input.dvi");
    let svg_path = tmp_dir.join("input.svg");

    let tex_content = format!(
        "\\documentclass[preview]{{standalone}}\n\
         \\usepackage{{amsmath}}\n\
         \\usepackage{{amssymb}}\n\
         \\begin{{document}}\n\
         {latex}\n\
         \\end{{document}}\n"
    );
    std::fs::write(&tex_path, tex_content).map_err(|e| format!("failed to write .tex: {e}"))?;

    let output = std::process::Command::new("latex")
        .args(["-interaction=nonstopmode", "-output-directory"])
        .arg(tmp_dir)
        .arg(&tex_path)
        .output()
        .map_err(|e| format!("failed to run latex: {e}"))?;

    if !output.status.success() {
        return Err(format!(
            "latex compilation failed:\n{}",
            String::from_utf8_lossy(&output.stdout)
        ));
    }

    let output = std::process::Command::new("dvisvgm")
        .args(["--no-fonts", "--exact", "-o"])
        .arg(&svg_path)
        .arg(&dvi_path)
        .output()
        .map_err(|e| format!("failed to run dvisvgm: {e}"))?;

    if !output.status.success() {
        return Err(format!(
            "dvisvgm failed:\n{}",
            String::from_utf8_lossy(&output.stderr)
        ));
    }

    std::fs::read_to_string(&svg_path).map_err(|e| format!("failed to read SVG: {e}"))
}

// ── SVG parsing ──────────────────────────────────────────────────────

/// Parse dvisvgm SVG output into glyph outlines.
fn parse_dvisvgm_svg(svg: &str) -> Vec<GlyphOutline> {
    let viewbox_min_x = parse_viewbox_min_x(svg);
    let path_defs = parse_path_defs(svg);
    let uses = parse_use_elements(svg);
    let rects = parse_rect_elements(svg);

    let mut glyphs = Vec::new();

    for use_el in &uses {
        if let Some(contours) = path_defs.get(&use_el.href) {
            let transformed: Vec<BezierContour> = contours
                .iter()
                .map(|c| offset_and_transform_contour(c, use_el.x, use_el.y, viewbox_min_x))
                .collect();
            if !transformed.is_empty() {
                glyphs.push(GlyphOutline {
                    contours: transformed,
                    advance_x: 0.0,
                });
            }
        }
    }

    for rect in &rects {
        let contour = rect_to_contour(rect, viewbox_min_x);
        glyphs.push(GlyphOutline {
            contours: vec![contour],
            advance_x: 0.0,
        });
    }

    glyphs
}

fn offset_and_transform_contour(
    contour: &BezierContour,
    use_x: f32,
    use_y: f32,
    viewbox_min_x: f32,
) -> BezierContour {
    BezierContour {
        segments: contour
            .segments
            .iter()
            .map(|seg| {
                CubicBezier::new(
                    transform_point(seg.p0.x + use_x, seg.p0.y + use_y, viewbox_min_x),
                    transform_point(seg.p1.x + use_x, seg.p1.y + use_y, viewbox_min_x),
                    transform_point(seg.p2.x + use_x, seg.p2.y + use_y, viewbox_min_x),
                    transform_point(seg.p3.x + use_x, seg.p3.y + use_y, viewbox_min_x),
                )
            })
            .collect(),
        closed: contour.closed,
    }
}

fn transform_point(svg_x: f32, svg_y: f32, viewbox_min_x: f32) -> Vec2 {
    vec2((svg_x - viewbox_min_x) / PT_PER_EM, -svg_y / PT_PER_EM)
}

struct UseElement {
    x: f32,
    y: f32,
    href: String,
}

struct RectElement {
    x: f32,
    y: f32,
    width: f32,
    height: f32,
}

fn rect_to_contour(rect: &RectElement, viewbox_min_x: f32) -> BezierContour {
    let x0 = rect.x;
    let y0 = rect.y;
    let x1 = rect.x + rect.width;
    let y1 = rect.y + rect.height;

    let p0 = transform_point(x0, y0, viewbox_min_x);
    let p1 = transform_point(x1, y0, viewbox_min_x);
    let p2 = transform_point(x1, y1, viewbox_min_x);
    let p3 = transform_point(x0, y1, viewbox_min_x);

    fn line_seg(a: Vec2, b: Vec2) -> CubicBezier {
        let c1 = a.lerp(b, 1.0 / 3.0);
        let c2 = a.lerp(b, 2.0 / 3.0);
        CubicBezier::new(a, c1, c2, b)
    }

    BezierContour {
        segments: vec![
            line_seg(p0, p1),
            line_seg(p1, p2),
            line_seg(p2, p3),
            line_seg(p3, p0),
        ],
        closed: true,
    }
}

// ── Simple XML attribute extraction ──────────────────────────────────

fn parse_viewbox_min_x(svg: &str) -> f32 {
    if let Some(vb) = extract_attr(svg, "viewBox") {
        let nums: Vec<f32> = vb
            .split_whitespace()
            .filter_map(|s| s.parse().ok())
            .collect();
        if !nums.is_empty() {
            return nums[0];
        }
    }
    0.0
}

fn parse_path_defs(svg: &str) -> HashMap<String, Vec<BezierContour>> {
    let mut defs = HashMap::new();
    let mut search_from = 0;

    while let Some(start) = svg[search_from..].find("<path ") {
        let abs_start = search_from + start;
        let end = match svg[abs_start..].find("/>") {
            Some(e) => abs_start + e + 2,
            None => break,
        };
        let tag = &svg[abs_start..end];

        if let (Some(id), Some(d)) = (extract_attr(tag, "id"), extract_attr(tag, "d")) {
            let contours = parse_svg_path(&d);
            defs.insert(id, contours);
        }
        search_from = end;
    }

    defs
}

fn parse_use_elements(svg: &str) -> Vec<UseElement> {
    let mut uses = Vec::new();
    let mut search_from = 0;

    while let Some(start) = svg[search_from..].find("<use ") {
        let abs_start = search_from + start;
        let end = match svg[abs_start..].find("/>") {
            Some(e) => abs_start + e + 2,
            None => break,
        };
        let tag = &svg[abs_start..end];

        let x = extract_attr(tag, "x")
            .and_then(|s| s.parse().ok())
            .unwrap_or(0.0);
        let y = extract_attr(tag, "y")
            .and_then(|s| s.parse().ok())
            .unwrap_or(0.0);

        // xlink:href='#g0-1' → strip the '#'
        if let Some(href) = extract_attr(tag, "xlink:href") {
            let href = href.strip_prefix('#').unwrap_or(&href).to_string();
            uses.push(UseElement { x, y, href });
        }

        search_from = end;
    }

    uses
}

fn parse_rect_elements(svg: &str) -> Vec<RectElement> {
    let mut rects = Vec::new();
    let mut search_from = 0;

    while let Some(start) = svg[search_from..].find("<rect ") {
        let abs_start = search_from + start;
        let end = match svg[abs_start..].find("/>") {
            Some(e) => abs_start + e + 2,
            None => break,
        };
        let tag = &svg[abs_start..end];

        let x = extract_attr(tag, "x")
            .and_then(|s| s.parse().ok())
            .unwrap_or(0.0);
        let y = extract_attr(tag, "y")
            .and_then(|s| s.parse().ok())
            .unwrap_or(0.0);
        let width = extract_attr(tag, "width")
            .and_then(|s| s.parse().ok())
            .unwrap_or(0.0);
        let height = extract_attr(tag, "height")
            .and_then(|s| s.parse().ok())
            .unwrap_or(0.0);

        rects.push(RectElement {
            x,
            y,
            width,
            height,
        });
        search_from = end;
    }

    rects
}

/// Extract an attribute value from an XML tag string.
/// Handles both single and double quotes. Only matches complete attribute
/// names (preceded by a space or `<`), so `d='...'` won't match inside `id='...'`.
fn extract_attr(tag: &str, name: &str) -> Option<String> {
    for quote in ['\'', '"'] {
        let pattern = format!("{name}={quote}");
        let mut search_from = 0;
        while let Some(pos) = tag[search_from..].find(&pattern) {
            let abs_pos = search_from + pos;
            let at_boundary = abs_pos == 0 || matches!(tag.as_bytes()[abs_pos - 1], b' ' | b'<');
            if at_boundary {
                let val_start = abs_pos + pattern.len();
                if let Some(end) = tag[val_start..].find(quote) {
                    return Some(tag[val_start..val_start + end].to_string());
                }
            }
            search_from = abs_pos + 1;
        }
    }
    None
}

// ── SVG path data parser ─────────────────────────────────────────────

/// Parse an SVG path `d` attribute into bezier contours.
fn parse_svg_path(d: &str) -> Vec<BezierContour> {
    let tokens = tokenize_path(d);
    let mut contours = Vec::new();
    let mut current_segments: Vec<CubicBezier> = Vec::new();
    let mut pos = Vec2::ZERO;
    let mut contour_start = Vec2::ZERO;
    let mut last_control: Option<Vec2> = None; // for S/s commands
    let mut i = 0;

    while i < tokens.len() {
        match tokens[i] {
            PathToken::Command(cmd) => {
                i += 1;
                match cmd {
                    'M' | 'm' => {
                        // Flush current contour
                        if !current_segments.is_empty() {
                            contours.push(BezierContour {
                                segments: std::mem::take(&mut current_segments),
                                closed: false,
                            });
                        }
                        let (x, y) = consume_pair(&tokens, &mut i);
                        if cmd == 'M' {
                            pos = vec2(x, y);
                        } else {
                            pos += vec2(x, y);
                        }
                        contour_start = pos;
                        last_control = None;

                        // Implicit LineTo for subsequent coordinate pairs
                        while i < tokens.len() && matches!(tokens[i], PathToken::Number(_)) {
                            let (x, y) = consume_pair(&tokens, &mut i);
                            let end = if cmd == 'M' {
                                vec2(x, y)
                            } else {
                                pos + vec2(x, y)
                            };
                            current_segments.push(line_as_cubic(pos, end));
                            pos = end;
                            last_control = None;
                        }
                    }
                    'L' | 'l' => {
                        while i < tokens.len() && matches!(tokens[i], PathToken::Number(_)) {
                            let (x, y) = consume_pair(&tokens, &mut i);
                            let end = if cmd == 'L' {
                                vec2(x, y)
                            } else {
                                pos + vec2(x, y)
                            };
                            current_segments.push(line_as_cubic(pos, end));
                            pos = end;
                            last_control = None;
                        }
                    }
                    'H' | 'h' => {
                        while i < tokens.len() && matches!(tokens[i], PathToken::Number(_)) {
                            let x = consume_number(&tokens, &mut i);
                            let end = if cmd == 'H' {
                                vec2(x, pos.y)
                            } else {
                                vec2(pos.x + x, pos.y)
                            };
                            current_segments.push(line_as_cubic(pos, end));
                            pos = end;
                            last_control = None;
                        }
                    }
                    'V' | 'v' => {
                        while i < tokens.len() && matches!(tokens[i], PathToken::Number(_)) {
                            let y = consume_number(&tokens, &mut i);
                            let end = if cmd == 'V' {
                                vec2(pos.x, y)
                            } else {
                                vec2(pos.x, pos.y + y)
                            };
                            current_segments.push(line_as_cubic(pos, end));
                            pos = end;
                            last_control = None;
                        }
                    }
                    'C' | 'c' => {
                        while i < tokens.len() && matches!(tokens[i], PathToken::Number(_)) {
                            let (x1, y1) = consume_pair(&tokens, &mut i);
                            let (x2, y2) = consume_pair(&tokens, &mut i);
                            let (x, y) = consume_pair(&tokens, &mut i);
                            let (cp1, cp2, end) = if cmd == 'C' {
                                (vec2(x1, y1), vec2(x2, y2), vec2(x, y))
                            } else {
                                (pos + vec2(x1, y1), pos + vec2(x2, y2), pos + vec2(x, y))
                            };
                            current_segments.push(CubicBezier::new(pos, cp1, cp2, end));
                            last_control = Some(cp2);
                            pos = end;
                        }
                    }
                    'S' | 's' => {
                        while i < tokens.len() && matches!(tokens[i], PathToken::Number(_)) {
                            let (x2, y2) = consume_pair(&tokens, &mut i);
                            let (x, y) = consume_pair(&tokens, &mut i);
                            // Reflect previous control point
                            let cp1 = match last_control {
                                Some(lc) => vec2(2.0 * pos.x - lc.x, 2.0 * pos.y - lc.y),
                                None => pos,
                            };
                            let (cp2, end) = if cmd == 'S' {
                                (vec2(x2, y2), vec2(x, y))
                            } else {
                                (pos + vec2(x2, y2), pos + vec2(x, y))
                            };
                            current_segments.push(CubicBezier::new(pos, cp1, cp2, end));
                            last_control = Some(cp2);
                            pos = end;
                        }
                    }
                    'Q' | 'q' => {
                        while i < tokens.len() && matches!(tokens[i], PathToken::Number(_)) {
                            let (x1, y1) = consume_pair(&tokens, &mut i);
                            let (x, y) = consume_pair(&tokens, &mut i);
                            let (cp, end) = if cmd == 'Q' {
                                (vec2(x1, y1), vec2(x, y))
                            } else {
                                (pos + vec2(x1, y1), pos + vec2(x, y))
                            };
                            current_segments.push(CubicBezier::from_quadratic(pos, cp, end));
                            last_control = Some(cp);
                            pos = end;
                        }
                    }
                    'Z' | 'z' => {
                        // Close the contour
                        if (pos - contour_start).length() > 1e-6 {
                            current_segments.push(line_as_cubic(pos, contour_start));
                        }
                        if !current_segments.is_empty() {
                            contours.push(BezierContour {
                                segments: std::mem::take(&mut current_segments),
                                closed: true,
                            });
                        }
                        pos = contour_start;
                        last_control = None;
                    }
                    _ => {} // skip unknown commands
                }
            }
            PathToken::Number(_) => {
                i += 1; // skip stray numbers
            }
        }
    }

    // Flush any remaining open contour
    if !current_segments.is_empty() {
        contours.push(BezierContour {
            segments: current_segments,
            closed: false,
        });
    }

    contours
}

fn line_as_cubic(from: Vec2, to: Vec2) -> CubicBezier {
    let c1 = from.lerp(to, 1.0 / 3.0);
    let c2 = from.lerp(to, 2.0 / 3.0);
    CubicBezier::new(from, c1, c2, to)
}

fn consume_number(tokens: &[PathToken], i: &mut usize) -> f32 {
    match tokens[*i] {
        PathToken::Number(n) => {
            *i += 1;
            n
        }
        _ => panic!("expected number at token {i}"),
    }
}

fn consume_pair(tokens: &[PathToken], i: &mut usize) -> (f32, f32) {
    let x = consume_number(tokens, i);
    let y = consume_number(tokens, i);
    (x, y)
}

// ── SVG path tokenizer ───────────────────────────────────────────────

#[derive(Debug, Clone)]
enum PathToken {
    Command(char),
    Number(f32),
}

/// Tokenize SVG path data into commands and numbers.
///
/// Handles the compact format dvisvgm produces where negative signs
/// and decimal points serve as implicit separators:
/// `M2.168867-2.531507C...` → [Command('M'), Number(2.168867), Number(-2.531507), Command('C'), ...]
fn tokenize_path(d: &str) -> Vec<PathToken> {
    let mut tokens = Vec::new();
    let chars: Vec<char> = d.chars().collect();
    let mut i = 0;

    while i < chars.len() {
        let ch = chars[i];

        if ch.is_ascii_alphabetic() {
            tokens.push(PathToken::Command(ch));
            i += 1;
        } else if ch == '-' || ch == '+' || ch == '.' || ch.is_ascii_digit() {
            let (num, consumed) = parse_number_at(&chars, i);
            tokens.push(PathToken::Number(num));
            i += consumed;
        } else {
            // whitespace, comma, etc.
            i += 1;
        }
    }

    tokens
}

/// Parse a number starting at position `start` in the char slice.
/// Returns (value, chars_consumed).
fn parse_number_at(chars: &[char], start: usize) -> (f32, usize) {
    let mut i = start;
    let mut has_dot = false;

    // Optional sign
    if i < chars.len() && (chars[i] == '-' || chars[i] == '+') {
        i += 1;
    }

    // Integer part
    while i < chars.len() && chars[i].is_ascii_digit() {
        i += 1;
    }

    // Decimal point and fraction
    if i < chars.len() && chars[i] == '.' {
        has_dot = true;
        i += 1;
        while i < chars.len() && chars[i].is_ascii_digit() {
            i += 1;
        }
    }

    // Handle edge case: just a sign with no digits (shouldn't happen in valid SVG)
    if i == start || (i == start + 1 && (chars[start] == '-' || chars[start] == '+') && !has_dot) {
        return (0.0, 1);
    }

    let s: String = chars[start..i].iter().collect();
    let num = s.parse::<f32>().unwrap_or(0.0);
    (num, i - start)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tokenize_compact_path() {
        let tokens = tokenize_path("M2.5-3.7L1.0 2.0");
        // M, 2.5, -3.7, L, 1.0, 2.0
        assert_eq!(tokens.len(), 6);
        assert!(matches!(tokens[0], PathToken::Command('M')));
        assert!(matches!(tokens[1], PathToken::Number(n) if (n - 2.5).abs() < 1e-6));
        assert!(matches!(tokens[2], PathToken::Number(n) if (n - -3.7).abs() < 1e-6));
        assert!(matches!(tokens[3], PathToken::Command('L')));
    }

    #[test]
    fn tokenize_leading_dot() {
        let tokens = tokenize_path("M.5.6");
        // M, 0.5, 0.6
        assert_eq!(tokens.len(), 3);
        assert!(matches!(tokens[1], PathToken::Number(n) if (n - 0.5).abs() < 1e-6));
        assert!(matches!(tokens[2], PathToken::Number(n) if (n - 0.6).abs() < 1e-6));
    }

    #[test]
    fn parse_simple_path() {
        let contours = parse_svg_path("M0 0L10 0L10 10Z");
        assert_eq!(contours.len(), 1);
        assert!(contours[0].closed);
        // M→L, L→L, L→close = 3 segments
        assert_eq!(contours[0].segments.len(), 3);
    }

    #[test]
    fn parse_cubic_path() {
        let contours = parse_svg_path("M0 0C1 2 3 4 5 6Z");
        assert_eq!(contours.len(), 1);
        assert!(contours[0].closed);
        // C→cubic, close→line = 2 segments
        assert_eq!(contours[0].segments.len(), 2);
        let seg = &contours[0].segments[0];
        assert!((seg.p0 - vec2(0.0, 0.0)).length() < 1e-5);
        assert!((seg.p1 - vec2(1.0, 2.0)).length() < 1e-5);
        assert!((seg.p2 - vec2(3.0, 4.0)).length() < 1e-5);
        assert!((seg.p3 - vec2(5.0, 6.0)).length() < 1e-5);
    }

    #[test]
    fn parse_h_v_commands() {
        let contours = parse_svg_path("M0 0H10V5Z");
        assert_eq!(contours.len(), 1);
        let segs = &contours[0].segments;
        // H→(10,0), V→(10,5), close→(0,0)
        assert!((segs[0].p3 - vec2(10.0, 0.0)).length() < 1e-5);
        assert!((segs[1].p3 - vec2(10.0, 5.0)).length() < 1e-5);
    }

    #[test]
    fn parse_smooth_cubic() {
        // C sets last control at (3,4), then S reflects it
        let contours = parse_svg_path("M0 0C1 2 3 4 5 6S9 8 10 10Z");
        assert_eq!(contours.len(), 1);
        let segs = &contours[0].segments;
        // S: cp1 = reflect (3,4) around (5,6) = (7,8)
        assert!((segs[1].p1 - vec2(7.0, 8.0)).length() < 1e-5);
        assert!((segs[1].p2 - vec2(9.0, 8.0)).length() < 1e-5);
        assert!((segs[1].p3 - vec2(10.0, 10.0)).length() < 1e-5);
    }

    #[test]
    fn parse_dvisvgm_real_path() {
        // Subset of a real dvisvgm path (the '=' sign)
        let d = "M6.844334-3.257783C6.993773-3.257783 7.183064-3.257783 7.183064-3.457036S6.993773-3.656289 6.854296-3.656289H.886675Z";
        let contours = parse_svg_path(d);
        assert!(!contours.is_empty());
        assert!(contours[0].closed);
    }

    #[test]
    fn extract_attr_single_quotes() {
        let tag = "<path id='g0-1' d='M1 2'/>";
        assert_eq!(extract_attr(tag, "id"), Some("g0-1".to_string()));
        assert_eq!(extract_attr(tag, "d"), Some("M1 2".to_string()));
    }

    #[test]
    fn extract_attr_double_quotes() {
        let tag = r#"<use x="1.5" y="-2.3"/>"#;
        assert_eq!(extract_attr(tag, "x"), Some("1.5".to_string()));
        assert_eq!(extract_attr(tag, "y"), Some("-2.3".to_string()));
    }

    #[test]
    fn rect_to_contour_produces_closed_rect() {
        let rect = RectElement {
            x: 10.0,
            y: -5.0,
            width: 20.0,
            height: 1.0,
        };
        let contour = rect_to_contour(&rect, 0.0);
        assert!(contour.closed);
        assert_eq!(contour.segments.len(), 4);
    }

    #[test]
    fn parse_full_svg_structure() {
        let svg = r#"<?xml version='1.0'?>
<svg viewBox='0 -10 50 15'>
<defs>
<path id='g0' d='M0 0L5 0L5 5L0 5Z'/>
</defs>
<g id='page1'>
<use x='2' y='-3' xlink:href='#g0'/>
<rect x='10' y='-1' height='.5' width='8'/>
</g>
</svg>"#;
        let glyphs = parse_dvisvgm_svg(svg);
        assert_eq!(glyphs.len(), 2); // 1 use + 1 rect
        assert!(!glyphs[0].contours.is_empty());
        assert!(!glyphs[1].contours.is_empty());
    }

    #[test]
    fn coordinate_transform_flips_y() {
        let p = transform_point(10.0, -5.0, 0.0);
        assert!((p.x - 1.0).abs() < 1e-5); // 10 / 10
        assert!((p.y - 0.5).abs() < 1e-5); // -(-5) / 10
    }

    #[test]
    fn coordinate_transform_subtracts_viewbox_x() {
        let p = transform_point(10.0, 0.0, 5.0);
        assert!((p.x - 0.5).abs() < 1e-5); // (10 - 5) / 10
    }

    #[test]
    fn latex_compilation_produces_glyphs() {
        let glyphs = latex_to_glyphs("$x^2$").expect("latex compilation failed");
        assert!(
            !glyphs.is_empty(),
            "should produce at least one glyph for x^2"
        );
        // All glyphs should have contours
        for glyph in &glyphs {
            assert!(!glyph.contours.is_empty());
        }
    }

    #[test]
    fn latex_math_symbols() {
        let glyphs = latex_to_glyphs(r"$\int_{0}^{\infty} e^{-x^2} dx = \frac{\sqrt{\pi}}{2}$")
            .expect("latex compilation failed");
        assert!(
            glyphs.len() > 5,
            "complex formula should produce many glyphs, got {}",
            glyphs.len()
        );
    }

    #[test]
    fn relative_commands() {
        let contours = parse_svg_path("M0 0l10 0l0 5z");
        assert_eq!(contours.len(), 1);
        assert!(contours[0].closed);
        let segs = &contours[0].segments;
        assert!((segs[0].p3 - vec2(10.0, 0.0)).length() < 1e-5);
        assert!((segs[1].p3 - vec2(10.0, 5.0)).length() < 1e-5);
    }

    #[test]
    fn multiple_contours() {
        let contours = parse_svg_path("M0 0L10 0ZM20 20L30 20Z");
        assert_eq!(contours.len(), 2);
        assert!(contours[0].closed);
        assert!(contours[1].closed);
    }

    #[test]
    fn implicit_lineto_after_moveto() {
        // After M, subsequent coordinate pairs are implicit L
        let contours = parse_svg_path("M0 0 10 0 10 10Z");
        assert_eq!(contours.len(), 1);
        assert_eq!(contours[0].segments.len(), 3); // 2 implicit L + close
    }
}
