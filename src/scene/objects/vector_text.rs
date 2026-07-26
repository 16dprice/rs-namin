use std::collections::HashMap;

use lyon::math::point;
use lyon::path::Path;
use lyon::tessellation::geometry_builder::VertexBuffers;
use lyon::tessellation::{BuffersBuilder, FillOptions, FillTessellator, FillVertex, StrokeOptions, StrokeTessellator, StrokeVertex};
use macroquad::prelude::*;

use crate::scene::bezier::{BezierContour, GlyphOutline};
use crate::scene::font;
use crate::scene::latex;
use crate::scene::traits::{BoundingBox, SceneObject, animatable};

const MAX_VERTICES: usize = 10_000;
const MAX_INDICES: usize = 5_000;

/// Lyon tessellation tolerance. Default (0.1) is far too coarse for glyph-scale
/// curves (~1 world unit per em), producing visible polygon edges on letters
/// like "e", "o", "n". 0.005 gives smooth curves.
const TESSELLATION_TOLERANCE: f32 = 0.005;

/// A batch of contours to render with a given alpha.
struct RenderBatch {
    contours: Vec<BezierContour>,
    alpha: f32,
}

pub struct VectorText {
    pub glyphs: Vec<GlyphOutline>,
    pub position: Vec3,
    pub color: Vec4,
    pub progress: f32,
    pub fill_opacity: f32,
    pub stroke_width: f32,
    pub scale: f32,
    /// Per-character stagger: 0.0 = all glyphs animate simultaneously,
    /// 1.0 = fully sequential (each finishes before the next starts).
    pub stagger: f32,
}

impl VectorText {
    /// Create from text string and font data.
    /// `scale` controls the display size (1.0 = 1 em per world unit).
    pub fn new(text: &str, font_data: &[u8], scale: f32, color: Color) -> Self {
        let glyphs = font::extract_glyphs(font_data, text, 1.0);
        Self {
            glyphs,
            position: Vec3::ZERO,
            color: vec4(color.r, color.g, color.b, color.a),
            progress: 1.0,
            fill_opacity: 1.0,
            stroke_width: 0.02,
            scale,
            stagger: 1.0,
        }
    }

    /// Create from a LaTeX expression. Shells out to `latex` + `dvisvgm`.
    ///
    /// Panics if compilation fails (e.g. missing `latex`/`dvisvgm` or invalid LaTeX).
    pub fn from_latex(latex: &str, color: Color) -> Self {
        let glyphs = latex::latex_to_glyphs(latex).unwrap_or_else(|e| panic!("LaTeX compilation failed: {e}"));
        Self::from_glyphs(glyphs, color)
    }

    /// Create from pre-built glyph outlines (pipeline-agnostic).
    pub fn from_glyphs(glyphs: Vec<GlyphOutline>, color: Color) -> Self {
        Self {
            glyphs,
            position: Vec3::ZERO,
            color: vec4(color.r, color.g, color.b, color.a),
            progress: 1.0,
            fill_opacity: 1.0,
            stroke_width: 0.02,
            scale: 1.0,
            stagger: 1.0,
        }
    }

    /// Compute per-glyph local progress based on global progress and stagger.
    fn glyph_local_progress(&self, glyph_index: usize) -> f32 {
        let progress = self.progress.clamp(0.0, 1.0);
        let n = self.glyphs.len();
        if n == 0 {
            return 0.0;
        }

        let stagger = self.stagger.clamp(0.0, 1.0);
        // duration = 1 - (n-1)*stagger/n
        // At stagger=1: 1/n (sequential). At stagger=0: 1 (simultaneous).
        let glyph_duration = if n > 1 { 1.0 - (n as f32 - 1.0) * stagger / n as f32 } else { 1.0 };

        let start = glyph_index as f32 * stagger / n as f32;
        if glyph_duration > 0.0 {
            ((progress - start) / glyph_duration).clamp(0.0, 1.0)
        } else if progress >= start {
            1.0
        } else {
            0.0
        }
    }

    /// Compute stroke and fill batches with per-glyph alpha (DrawBorderThenFill).
    ///
    /// Each glyph's animation has two phases:
    /// - First half: stroke draws progressively (full alpha), no fill
    /// - Second half: fill fades in, stroke fades out (they cross-fade)
    ///
    /// At completion: fill at full alpha, no stroke — solid filled text.
    fn compute_visibility(&self) -> (Vec<RenderBatch>, Vec<RenderBatch>) {
        let progress = self.progress.clamp(0.0, 1.0);
        if progress <= 0.0 {
            return (vec![], vec![]);
        }

        let mut stroke_batches: Vec<RenderBatch> = Vec::new();
        let mut fill_batches: Vec<RenderBatch> = Vec::new();

        for (i, glyph) in self.glyphs.iter().enumerate() {
            let local_progress = self.glyph_local_progress(i);

            if local_progress <= 0.0 {
                continue;
            }

            // Fill alpha: 0 during first half, ramps 0→1 during second half
            let fill_alpha = if local_progress <= 0.5 {
                0.0
            } else {
                ((local_progress - 0.5) * 2.0).clamp(0.0, 1.0)
            };

            // Stroke alpha: full during first half, fades as fill takes over
            let stroke_alpha = 1.0 - fill_alpha;

            // Stroke: progressive contour reveal
            if stroke_alpha > 0.0 {
                let mut glyph_stroke_contours = Vec::new();

                if local_progress >= 1.0 {
                    glyph_stroke_contours.extend(glyph.contours.iter().cloned());
                } else {
                    let glyph_total: usize = glyph.contours.iter().map(|c| c.segments.len()).sum();
                    let visible_segments = local_progress * glyph_total as f32;
                    let mut remaining = visible_segments;

                    for contour in &glyph.contours {
                        if remaining <= 0.0 {
                            break;
                        }
                        let seg_count = contour.segments.len() as f32;
                        if remaining >= seg_count {
                            glyph_stroke_contours.push(contour.clone());
                            remaining -= seg_count;
                        } else {
                            let contour_progress = remaining / seg_count;
                            glyph_stroke_contours.push(contour.truncate(contour_progress));
                            remaining = 0.0;
                        }
                    }
                }

                if !glyph_stroke_contours.is_empty() {
                    if let Some(last) = stroke_batches.last_mut()
                        && (last.alpha - stroke_alpha).abs() < 1e-6
                    {
                        last.contours.extend(glyph_stroke_contours);
                    } else {
                        stroke_batches.push(RenderBatch {
                            contours: glyph_stroke_contours,
                            alpha: stroke_alpha,
                        });
                    }
                }
            }

            // Fill: uses FULL original contours (correct winding for holes)
            if fill_alpha > 0.0 && !glyph.contours.is_empty() {
                if let Some(last) = fill_batches.last_mut()
                    && (last.alpha - fill_alpha).abs() < 1e-6
                {
                    last.contours.extend(glyph.contours.iter().cloned());
                } else {
                    fill_batches.push(RenderBatch {
                        contours: glyph.contours.clone(),
                        alpha: fill_alpha,
                    });
                }
            }
        }

        (stroke_batches, fill_batches)
    }

    /// Convenience: all visible stroke contours.
    #[cfg(test)]
    fn visible_contours(&self) -> Vec<BezierContour> {
        self.compute_visibility().0.into_iter().flat_map(|b| b.contours).collect()
    }

    /// Build lyon path from contours, scaling coordinates.
    fn contours_to_path(contours: &[BezierContour], scale: f32) -> Path {
        let mut builder = Path::builder();
        for contour in contours {
            if contour.segments.is_empty() {
                continue;
            }
            let first = &contour.segments[0];
            builder.begin(point(first.p0.x * scale, first.p0.y * scale));
            for seg in &contour.segments {
                builder.cubic_bezier_to(
                    point(seg.p1.x * scale, seg.p1.y * scale),
                    point(seg.p2.x * scale, seg.p2.y * scale),
                    point(seg.p3.x * scale, seg.p3.y * scale),
                );
            }
            builder.end(contour.closed);
        }
        builder.build()
    }

    /// Convert tessellated buffers into chunked macroquad Meshes.
    fn buffers_to_meshes(buffers: &VertexBuffers<[f32; 2], u16>, position: Vec3, color_bytes: [u8; 4]) -> Vec<Mesh> {
        if buffers.vertices.is_empty() || buffers.indices.is_empty() {
            return vec![];
        }

        let normal = vec4(0.0, 0.0, 1.0, 0.0);
        let to_vertex = |p: &[f32; 2]| Vertex {
            position: vec3(position.x + p[0], position.y + p[1], position.z),
            uv: vec2(0.0, 0.0),
            color: color_bytes,
            normal,
        };

        // Fast path: fits in a single mesh
        if buffers.vertices.len() <= MAX_VERTICES && buffers.indices.len() <= MAX_INDICES {
            return vec![Mesh {
                vertices: buffers.vertices.iter().map(to_vertex).collect(),
                indices: buffers.indices.clone(),
                texture: None,
            }];
        }

        // Chunk by triangles with vertex remapping
        let all_vertices = &buffers.vertices;
        let max_indices_per_chunk = (MAX_INDICES / 3) * 3;
        let mut meshes = Vec::new();
        let mut vertex_map: HashMap<u16, u16> = HashMap::new();
        let mut chunk_vertices: Vec<Vertex> = Vec::new();
        let mut chunk_indices: Vec<u16> = Vec::new();

        for tri in buffers.indices.chunks(3) {
            if tri.len() < 3 {
                break;
            }

            let new_verts = tri.iter().filter(|i| !vertex_map.contains_key(i)).count();

            if chunk_vertices.len() + new_verts > MAX_VERTICES || chunk_indices.len() + 3 > max_indices_per_chunk {
                meshes.push(Mesh {
                    vertices: std::mem::take(&mut chunk_vertices),
                    indices: std::mem::take(&mut chunk_indices),
                    texture: None,
                });
                vertex_map.clear();
            }

            for &idx in tri {
                let mapped = *vertex_map.entry(idx).or_insert_with(|| {
                    let n = chunk_vertices.len() as u16;
                    chunk_vertices.push(to_vertex(&all_vertices[idx as usize]));
                    n
                });
                chunk_indices.push(mapped);
            }
        }

        if !chunk_indices.is_empty() {
            meshes.push(Mesh {
                vertices: chunk_vertices,
                indices: chunk_indices,
                texture: None,
            });
        }

        meshes
    }

    /// Tessellate visible contours and return chunked meshes (fill behind stroke).
    fn build_meshes(&self) -> Vec<Mesh> {
        let (stroke_batches, fill_batches) = self.compute_visibility();
        if stroke_batches.is_empty() && fill_batches.is_empty() {
            return vec![];
        }

        let mut meshes = Vec::new();

        // Fill pass (rendered first, behind stroke)
        if self.fill_opacity > 0.0 {
            let mut fill_opts = FillOptions::default();
            fill_opts.fill_rule = lyon::tessellation::FillRule::NonZero;
            fill_opts.tolerance = TESSELLATION_TOLERANCE;

            for batch in &fill_batches {
                let fill_path = Self::contours_to_path(&batch.contours, self.scale);
                let fill_alpha = (self.color.w * self.fill_opacity * batch.alpha).clamp(0.0, 1.0);
                let fill_color: [u8; 4] = Color::new(self.color.x, self.color.y, self.color.z, fill_alpha).into();

                let mut tessellator = FillTessellator::new();
                let mut buffers: VertexBuffers<[f32; 2], u16> = VertexBuffers::new();

                let result = tessellator.tessellate_path(
                    &fill_path,
                    &fill_opts,
                    &mut BuffersBuilder::new(&mut buffers, |vertex: FillVertex| {
                        let p = vertex.position();
                        [p.x, p.y]
                    }),
                );

                if result.is_ok() {
                    meshes.extend(Self::buffers_to_meshes(&buffers, self.position, fill_color));
                }
            }
        }

        // Stroke pass (rendered on top of fill, fades out as fill takes over)
        if self.stroke_width > 0.0 {
            let stroke_opts = StrokeOptions::default()
                .with_line_width(self.stroke_width)
                .with_tolerance(TESSELLATION_TOLERANCE);

            for batch in &stroke_batches {
                let stroke_path = Self::contours_to_path(&batch.contours, self.scale);
                let stroke_alpha = (self.color.w * batch.alpha).clamp(0.0, 1.0);
                let stroke_color: [u8; 4] = Color::new(self.color.x, self.color.y, self.color.z, stroke_alpha).into();

                let mut tessellator = StrokeTessellator::new();
                let mut buffers: VertexBuffers<[f32; 2], u16> = VertexBuffers::new();

                let result = tessellator.tessellate_path(
                    &stroke_path,
                    &stroke_opts,
                    &mut BuffersBuilder::new(&mut buffers, |vertex: StrokeVertex| {
                        let p = vertex.position();
                        [p.x, p.y]
                    }),
                );

                if result.is_ok() {
                    meshes.extend(Self::buffers_to_meshes(&buffers, self.position, stroke_color));
                }
            }
        }

        meshes
    }
}

impl SceneObject for VectorText {
    fn draw(&self) {
        if self.progress <= 0.0 {
            return;
        }
        for mesh in self.build_meshes() {
            draw_mesh(&mesh);
        }
    }

    fn bounding_box(&self) -> BoundingBox {
        let mut min_x = f32::MAX;
        let mut min_y = f32::MAX;
        let mut max_x = f32::MIN;
        let mut max_y = f32::MIN;

        for glyph in &self.glyphs {
            for contour in &glyph.contours {
                for seg in &contour.segments {
                    for p in [seg.p0, seg.p1, seg.p2, seg.p3] {
                        let x = self.position.x + p.x * self.scale;
                        let y = self.position.y + p.y * self.scale;
                        min_x = min_x.min(x);
                        min_y = min_y.min(y);
                        max_x = max_x.max(x);
                        max_y = max_y.max(y);
                    }
                }
            }
        }

        if min_x > max_x {
            // No contours
            BoundingBox {
                min: self.position,
                max: self.position,
            }
        } else {
            BoundingBox {
                min: vec3(min_x, min_y, self.position.z),
                max: vec3(max_x, max_y, self.position.z),
            }
        }
    }
}

animatable!(VectorText {
    position: Vec3,
    color: Vec4,
    progress: Float,
    fill_opacity: Float,
    stroke_width: Float,
    scale: Float,
    stagger: Float,
});

#[cfg(test)]
mod tests {
    use super::*;
    use crate::scene::bezier::{BezierContour, CubicBezier, GlyphOutline};
    use crate::scene::traits::test_support::assert_property_roundtrip;

    fn make_test_glyph() -> GlyphOutline {
        let seg = CubicBezier::new(vec2(0.0, 0.0), vec2(0.5, 1.0), vec2(1.0, 1.0), vec2(1.0, 0.0));
        GlyphOutline {
            contours: vec![BezierContour {
                segments: vec![seg],
                closed: true,
            }],
            advance_x: 1.0,
        }
    }

    fn make_vector_text() -> VectorText {
        VectorText {
            glyphs: vec![make_test_glyph()],
            position: Vec3::ZERO,
            color: vec4(1.0, 1.0, 1.0, 1.0),
            progress: 1.0,
            fill_opacity: 1.0,
            stroke_width: 0.02,
            scale: 1.0,
            stagger: 1.0,
        }
    }

    #[test]
    fn property_round_trip() {
        assert_property_roundtrip(&mut make_vector_text());
    }

    #[test]
    fn progress_clamping() {
        let mut vt = make_vector_text();
        vt.progress = -0.5;
        let contours = vt.visible_contours();
        assert!(contours.is_empty());

        // At progress=1.5 (clamped to 1.0), stroke has faded out but fill is present
        vt.progress = 1.5;
        let (_stroke, fill) = vt.compute_visibility();
        assert!(!fill.is_empty());
    }

    #[test]
    fn empty_at_zero_progress() {
        let mut vt = make_vector_text();
        vt.progress = 0.0;
        let contours = vt.visible_contours();
        assert!(contours.is_empty());
        let meshes = vt.build_meshes();
        assert!(meshes.is_empty());
    }

    #[test]
    fn non_empty_at_full_progress() {
        let vt = make_vector_text();
        // At progress=1, stroke faded out but fill is present → meshes exist
        let meshes = vt.build_meshes();
        assert!(!meshes.is_empty());
    }

    #[test]
    fn meshes_respect_vertex_limit() {
        let vt = make_vector_text();
        for mesh in vt.build_meshes() {
            assert!(mesh.vertices.len() <= MAX_VERTICES);
            assert!(mesh.indices.len() <= MAX_INDICES);
        }
    }

    #[test]
    fn from_glyphs_constructor() {
        let vt = VectorText::from_glyphs(vec![make_test_glyph()], WHITE);
        assert_eq!(vt.glyphs.len(), 1);
        assert_eq!(vt.scale, 1.0);
        assert_eq!(vt.progress, 1.0);
    }

    #[test]
    fn new_constructor_with_font() {
        let vt = VectorText::new("A", font::default_font(), 1.0, WHITE);
        assert!(!vt.glyphs.is_empty());
        assert_eq!(vt.scale, 1.0);
    }

    #[test]
    fn fill_fades_in_gradually() {
        let glyph1 = make_test_glyph();
        let glyph2 = make_test_glyph();
        let mut vt = VectorText {
            glyphs: vec![glyph1, glyph2],
            position: Vec3::ZERO,
            color: vec4(1.0, 1.0, 1.0, 1.0),
            progress: 0.5, // first glyph complete (stagger=1), second not started
            fill_opacity: 1.0,
            stroke_width: 0.02,
            scale: 1.0,
            stagger: 1.0,
        };
        let (_stroke, fill_batches) = vt.compute_visibility();
        // First glyph complete → fill alpha = 1.0
        assert_eq!(fill_batches.len(), 1);
        assert!((fill_batches[0].alpha - 1.0).abs() < 1e-5);

        // At 25% progress with stagger=1, glyph0 is at local 50% → fill just starting
        vt.progress = 0.25;
        let (_stroke, fill_batches) = vt.compute_visibility();
        // local_progress=0.5, fill_alpha = (0.5-0.5)*2 = 0.0 → no fill yet
        assert!(fill_batches.is_empty());

        // At 37.5% progress, glyph0 is at local 75% → fill fading in
        vt.progress = 0.375;
        let (_stroke, fill_batches) = vt.compute_visibility();
        assert_eq!(fill_batches.len(), 1);
        assert!(fill_batches[0].alpha > 0.0);
        assert!(fill_batches[0].alpha < 1.0);

        // At full progress, both glyphs have fill alpha = 1.0
        vt.progress = 1.0;
        let (_stroke, fill_batches) = vt.compute_visibility();
        // Both at alpha=1.0, batched together
        assert_eq!(fill_batches.len(), 1);
        assert!((fill_batches[0].alpha - 1.0).abs() < 1e-5);
        assert_eq!(fill_batches[0].contours.len(), 2);
    }

    #[test]
    fn stroke_fades_out_as_fill_fades_in() {
        let mut vt = VectorText {
            glyphs: vec![make_test_glyph()],
            position: Vec3::ZERO,
            color: vec4(1.0, 1.0, 1.0, 1.0),
            progress: 0.25, // local=0.25, first half → stroke full, no fill
            fill_opacity: 1.0,
            stroke_width: 0.02,
            scale: 1.0,
            stagger: 1.0,
        };
        let (stroke_batches, fill_batches) = vt.compute_visibility();
        assert_eq!(stroke_batches.len(), 1);
        assert!((stroke_batches[0].alpha - 1.0).abs() < 1e-5);
        assert!(fill_batches.is_empty());

        // At 75%, fill_alpha=0.5, stroke_alpha=0.5
        vt.progress = 0.75;
        let (stroke_batches, fill_batches) = vt.compute_visibility();
        assert_eq!(stroke_batches.len(), 1);
        assert!((stroke_batches[0].alpha - 0.5).abs() < 1e-5);
        assert_eq!(fill_batches.len(), 1);
        assert!((fill_batches[0].alpha - 0.5).abs() < 1e-5);

        // At 100%, fill=1 stroke=0 → no stroke batches
        vt.progress = 1.0;
        let (stroke_batches, fill_batches) = vt.compute_visibility();
        assert!(stroke_batches.is_empty());
        assert_eq!(fill_batches.len(), 1);
        assert!((fill_batches[0].alpha - 1.0).abs() < 1e-5);
    }

    #[test]
    fn no_fill_when_opacity_zero() {
        let mut vt = make_vector_text();
        vt.fill_opacity = 0.0;
        // At progress=1, stroke_alpha=0, fill disabled → no meshes
        // Set progress to 0.5 so stroke is still visible
        vt.progress = 0.25;
        let meshes = vt.build_meshes();
        assert!(!meshes.is_empty());
    }

    #[test]
    fn stagger_zero_all_simultaneous() {
        let mut vt = VectorText {
            glyphs: vec![make_test_glyph(), make_test_glyph()],
            position: Vec3::ZERO,
            color: vec4(1.0, 1.0, 1.0, 1.0),
            progress: 0.5,
            fill_opacity: 1.0,
            stroke_width: 0.02,
            scale: 1.0,
            stagger: 0.0,
        };
        let (stroke_batches, fill_batches) = vt.compute_visibility();
        // Both glyphs at local_progress=0.5 → stroke full, no fill
        let stroke_contour_count: usize = stroke_batches.iter().map(|b| b.contours.len()).sum();
        assert_eq!(stroke_contour_count, 2);
        assert!(fill_batches.is_empty());

        // At progress=0.75, local=0.75, fill_alpha=0.5, stroke_alpha=0.5
        vt.progress = 0.75;
        let (stroke_batches, fill_batches) = vt.compute_visibility();
        let stroke_contour_count: usize = stroke_batches.iter().map(|b| b.contours.len()).sum();
        assert_eq!(stroke_contour_count, 2);
        assert_eq!(fill_batches.len(), 1);
        assert!((fill_batches[0].alpha - 0.5).abs() < 1e-5);

        // At progress=1, fill=1 stroke=0 → no stroke
        vt.progress = 1.0;
        let (stroke_batches, fill_batches) = vt.compute_visibility();
        assert!(stroke_batches.is_empty());
        assert_eq!(fill_batches.len(), 1);
        assert!((fill_batches[0].alpha - 1.0).abs() < 1e-5);
    }

    #[test]
    fn stagger_one_fully_sequential() {
        let vt = VectorText {
            glyphs: vec![make_test_glyph(), make_test_glyph()],
            position: Vec3::ZERO,
            color: vec4(1.0, 1.0, 1.0, 1.0),
            progress: 0.5, // first glyph complete, second not started
            fill_opacity: 1.0,
            stroke_width: 0.02,
            scale: 1.0,
            stagger: 1.0,
        };
        let (stroke_batches, fill_batches) = vt.compute_visibility();
        // First glyph complete → fill=1, stroke=0 (no stroke for it)
        // Second glyph not started → nothing
        assert!(stroke_batches.is_empty());
        assert_eq!(fill_batches.len(), 1);
    }

    #[test]
    fn stagger_half_overlapping() {
        let vt = VectorText {
            glyphs: vec![make_test_glyph(), make_test_glyph()],
            position: Vec3::ZERO,
            color: vec4(1.0, 1.0, 1.0, 1.0),
            progress: 0.5,
            fill_opacity: 1.0,
            stroke_width: 0.02,
            scale: 1.0,
            stagger: 0.5,
        };
        let (stroke_batches, _fill) = vt.compute_visibility();
        // With stagger=0.5, n=2: duration=0.75, starts=[0.0, 0.25]
        // glyph0 local=0.67 → fill_alpha=0.33, stroke_alpha=0.67
        // glyph1 local=0.33 → fill_alpha=0, stroke_alpha=1.0
        // Both have stroke (different alphas → 2 batches)
        let stroke_contour_count: usize = stroke_batches.iter().map(|b| b.contours.len()).sum();
        assert_eq!(stroke_contour_count, 2);
    }

    #[test]
    fn bounding_box_empty_glyphs() {
        let vt = VectorText {
            glyphs: vec![],
            position: vec3(1.0, 2.0, 0.0),
            color: vec4(1.0, 1.0, 1.0, 1.0),
            progress: 1.0,
            fill_opacity: 1.0,
            stroke_width: 0.02,
            scale: 1.0,
            stagger: 1.0,
        };
        let bb = vt.bounding_box();
        assert_eq!(bb.min, vt.position);
        assert_eq!(bb.max, vt.position);
    }

    #[test]
    fn bounding_box_scales_with_scale() {
        let mut vt = make_vector_text();
        let bb1 = vt.bounding_box();
        vt.scale = 2.0;
        let bb2 = vt.bounding_box();
        let w1 = bb1.max.x - bb1.min.x;
        let w2 = bb2.max.x - bb2.min.x;
        assert!((w2 - w1 * 2.0).abs() < 1e-5);
    }
}
