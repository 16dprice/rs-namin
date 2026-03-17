# Vector Text Rendering

World-space scene object that renders text as bezier curve outlines with write-on animation. The data model is pipeline-agnostic — `VectorText` receives bezier curves and doesn't care whether they came from a font file or LaTeX.

## Architecture

### Source files

| File | Purpose |
|------|---------|
| `src/scene/bezier.rs` | `CubicBezier`, `BezierContour`, `GlyphOutline` — data model + De Casteljau math |
| `src/scene/font.rs` | `extract_glyphs()` — ttf-parser → glyph outlines; `default_font()` — embedded Roboto |
| `src/scene/objects/vector_text.rs` | `VectorText` scene object — tessellation via lyon, chunked rendering |
| `src/examples/vector_text.rs` | Example: "Hello" with progress 0→1 over 3 seconds, stagger=0.5 |
| `assets/fonts/Roboto-Regular.ttf` | Default embedded font (Apache 2.0) |

### Data model

```
CubicBezier { p0, p1, p2, p3: Vec2 }     — one cubic segment
BezierContour { segments: Vec<CubicBezier>, closed: bool }  — one path
GlyphOutline { contours: Vec<BezierContour>, advance_x: f32 }  — one character

VectorText {
    glyphs: Vec<GlyphOutline>,
    position: Vec3,
    color: Vec4,
    progress: f32,        // 0.0..1.0 write-on reveal
    fill_opacity: f32,    // 0.0..1.0 master fill multiplier
    stroke_width: f32,    // world units
    scale: f32,           // multiplier on glyph coordinates
    stagger: f32,         // 0.0 = simultaneous, 1.0 = fully sequential
}
```

### Animatable properties

`position` (Vec3), `color` (Vec4), `progress` (Float), `fill_opacity` (Float), `stroke_width` (Float), `scale` (Float), `stagger` (Float)

### How write-on works

Each frame, `progress` drives a two-pass render: stroke (write-on) and fill (fade-in).

**Per-glyph timing with stagger:**

Each glyph `i` of `n` gets its own local progress window:
- `start_i = i * stagger / n`
- `duration = 1 - (n-1) * stagger / n`
- `local_progress = (global_progress - start_i) / duration`, clamped to [0,1]

At `stagger=1.0` glyphs are fully sequential (each completes before the next starts); at `stagger=0.0` all animate identically.

**Stroke pass:** progressive contour reveal via bezier truncation. Segments before the boundary render complete; the boundary segment is split via De Casteljau (`CubicBezier::split_at`).

**Fill pass:** uses the **full original glyph contours** (not the truncated stroke contours). This is critical for correct hole rendering — letters like "O" and "A" have inner and outer contours that must both be present for the NonZero winding rule to cut the hole correctly.

Fill alpha per glyph = `max(0, (local_progress - 0.5) * 2)` — fill fades in during the second half of each glyph's animation, reaching full alpha when the glyph's stroke is complete. The final alpha multiplies `color.a * fill_opacity * batch.alpha`.

Consecutive glyphs at the same fill alpha are batched into a single tessellation call.

**Rendering order:** fill meshes first (behind), then stroke meshes on top.

Both passes tessellate via lyon and chunk output into multiple `Mesh` if needed (10k vertex / 5k index limit — same constraint as `Spiral`).

### Constructors

- `VectorText::new(text, font_data, scale, color)` — extracts glyphs at 1 em = 1 world unit, applies `scale` at render time
- `VectorText::from_glyphs(glyphs, color)` — pipeline-agnostic entry point for future LaTeX support

### Font extraction

`extract_glyphs(font_data, text, scale)` implements `ttf_parser::OutlineBuilder`:
- `move_to` / `close` → contour boundaries
- `line_to` → degenerate cubic (control points along line)
- `quad_to` → promoted to cubic via standard 2/3 formula
- `curve_to` → cubic bezier directly
- Positions offset by cumulative `advance_x` for layout

### Dependencies

| Crate | Purpose |
|-------|---------|
| `ttf-parser` 0.25 | Extract glyph outlines from .ttf files |
| `lyon` 1.0 | Tessellate bezier paths into triangle meshes |

### Relationship to Text object

`Text` (`src/scene/objects/text.rs`) is screen-space bitmap text using `draw_text`. `VectorText` is world-space mesh text for animated rendering. They coexist.

## Future work

- **LaTeX pipeline:** shell to `latex` + `dvisvgm --no-fonts`, parse SVG paths into `Vec<GlyphOutline>`, feed to `VectorText::from_glyphs`
