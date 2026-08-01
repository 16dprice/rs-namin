//! Scene documents: data-driven scenes loaded from RON files.
//!
//! A `SceneDoc` is the serializable counterpart of a hand-written builder
//! function — objects, initial property overrides, keyframe tracks, and a
//! camera — and builds into the same `(Scene, Timeline, Camera)` triple.
//! Documents in `scenes/*.ron` are discovered by the registry and appear in
//! the library/snapshot/export like any built-in scene. This is the designer's
//! document model (docs/gui_plan.md, Phase 2).

use macroquad::prelude::*;
use serde::{Deserialize, Serialize};

use crate::animation::binding::Binding;
use crate::animation::easing::Easing;
use crate::animation::timeline::Timeline;
use crate::animation::track::{Keyframe, Track, TrackTarget};
use crate::camera::Camera;
use crate::scene::traits::Animatable;
use crate::scene::value::AnimValue;
use crate::scene::{Scene, SceneNode};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SceneDoc {
    #[serde(default)]
    pub description: String,
    #[serde(default)]
    pub camera: CameraDoc,
    pub objects: Vec<ObjectDoc>,
    #[serde(default)]
    pub tracks: Vec<TrackDoc>,
    /// Property bindings: each frame, after tracks apply, the target property
    /// is overwritten with the source property's value (+ optional offset).
    #[serde(default)]
    pub bindings: Vec<BindingDoc>,
    /// Per-document export defaults, pre-filling the export form and the
    /// non-interactive CLI. All fields optional; absent = app defaults.
    #[serde(default)]
    pub export: ExportDefaults,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ExportDefaults {
    #[serde(default)]
    pub resolution: Option<String>,
    #[serde(default)]
    pub fps: Option<u32>,
    #[serde(default)]
    pub output: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CameraDoc {
    pub position: Vec3,
    pub target: Vec3,
    #[serde(default = "default_fov")]
    pub fov: f32,
    /// Initial overrides for any other camera property (up, near, far,
    /// rotation_x/y/z...), applied after construction like `ObjectDoc.set`.
    #[serde(default)]
    pub set: Vec<(String, AnimValue)>,
}

fn default_fov() -> f32 {
    60.0
}

impl Default for CameraDoc {
    fn default() -> Self {
        Self {
            position: vec3(0.0, 5.0, 10.0),
            target: Vec3::ZERO,
            fov: 60.0,
            set: Vec::new(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ObjectDoc {
    /// Document-local identifier referenced by tracks.
    pub id: String,
    pub object: ObjectSpec,
    /// Initial property overrides applied after construction, e.g.
    /// `[("progress", Float(0.0))]`.
    #[serde(default)]
    pub set: Vec<(String, AnimValue)>,
}

/// Constructible object types and their parameters. Colors are RGBA in 0-1.
/// `VectorText` always uses the built-in default font; objects with other
/// non-data constructor inputs (custom fonts, textures, LaTeX) are not yet
/// representable.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ObjectSpec {
    Disk {
        position: Vec3,
        radius: f32,
        color: Vec4,
    },
    Ring {
        position: Vec3,
        radius: f32,
        color: Vec4,
        progress: f32,
    },
    Rectangle {
        position: Vec3,
        size: Vec2,
        color: Vec4,
    },
    Polygon {
        position: Vec3,
        radius: f32,
        sides: u32,
        color: Vec4,
    },
    Line {
        start: Vec3,
        end: Vec3,
        color: Vec4,
    },
    Arc {
        position: Vec3,
        inner_radius: f32,
        outer_radius: f32,
        start_angle: f32,
        sweep_angle: f32,
        color: Vec4,
    },
    Arrow {
        start: Vec3,
        end: Vec3,
        color: Vec4,
    },
    Spiral {
        position: Vec3,
        delta_radius: f32,
        delta_theta: f32,
        color: Vec4,
        num_points: usize,
        dot_radius: f32,
    },
    Torus {
        position: Vec3,
        major_radius: f32,
        minor_radius: f32,
        color: Vec4,
    },
    Tube {
        points: Vec<Vec3>,
        radius: f32,
        color: Vec4,
        #[serde(default)]
        colors: Vec<Vec4>,
        #[serde(default)]
        closed: bool,
    },
    Text {
        content: String,
        position: Vec2,
        font_size: f32,
        color: Vec4,
    },
    /// Bezier-outline text with the write-on animation (world-space, default
    /// font). Reveal via `progress`; `stagger`, `stroke_width`, and
    /// `fill_opacity` are animatable properties (use `set`/tracks).
    VectorText {
        content: String,
        position: Vec3,
        /// Display size: 1.0 = one em per world unit.
        scale: f32,
        color: Vec4,
    },
    /// An L-system: `axiom` rewritten by `rules` for `iterations` steps,
    /// then drawn as turtle graphics (`F`/`G` draw forward, `+`/`-` turn by
    /// `theta` radians, `[`/`]` push/pop, other letters are silent
    /// variables). Each rule is `(symbol, replacement)`; only the symbol's
    /// first char is used. `progress` reveals the path; the `pen_position`
    /// output is the drawing tip. Optional `colors` is a segment gradient.
    LSystem {
        axiom: String,
        rules: Vec<(String, String)>,
        /// Turn angle in radians.
        theta: f32,
        iterations: f32,
        position: Vec3,
        scale: f32,
        color: Vec4,
        #[serde(default)]
        colors: Vec<Vec4>,
    },
    /// A function plot: `y = f(x)` with axes, drawn into a `size` rectangle
    /// centered on `position`. `expression` is a math string in `x` (see
    /// `scene::expr`); one that doesn't parse plots axes only. Bounds are
    /// animatable Vec2s (keyframe/bind them to zoom or pan the window);
    /// `progress` reveals the curve; the `pen_position` output is the tip of
    /// the revealed curve for bindings.
    Plot {
        expression: String,
        position: Vec3,
        size: Vec2,
        x_bounds: Vec2,
        y_bounds: Vec2,
        color: Vec4,
        /// Curve sample count across `x_bounds` (structural, not animatable).
        samples: usize,
    },
}

fn color(v: Vec4) -> Color {
    Color::new(v.x, v.y, v.z, v.w)
}

impl ObjectSpec {
    pub(crate) fn spawn(&self) -> Box<dyn SceneNode> {
        use crate::scene::objects::*;
        match self.clone() {
            ObjectSpec::Disk {
                position,
                radius,
                color: c,
            } => Box::new(Disk::new(position, radius, color(c))),
            ObjectSpec::Ring {
                position,
                radius,
                color: c,
                progress,
            } => Box::new(Ring::new(position, radius, color(c), progress)),
            ObjectSpec::Rectangle { position, size, color: c } => Box::new(Rectangle::new(position, size, color(c))),
            ObjectSpec::Polygon {
                position,
                radius,
                sides,
                color: c,
            } => Box::new(Polygon::new(position, radius, sides, color(c))),
            ObjectSpec::Line { start, end, color: c } => Box::new(Line::new(start, end, color(c))),
            ObjectSpec::Arc {
                position,
                inner_radius,
                outer_radius,
                start_angle,
                sweep_angle,
                color: c,
            } => Box::new(Arc::new(position, inner_radius, outer_radius, start_angle, sweep_angle, color(c))),
            ObjectSpec::Arrow { start, end, color: c } => Box::new(Arrow::new(start, end, color(c))),
            ObjectSpec::Spiral {
                position,
                delta_radius,
                delta_theta,
                color: c,
                num_points,
                dot_radius,
            } => Box::new(Spiral::new(position, delta_radius, delta_theta, color(c), num_points, dot_radius)),
            ObjectSpec::Torus {
                position,
                major_radius,
                minor_radius,
                color: c,
            } => Box::new(Torus::new(position, major_radius, minor_radius, color(c))),
            ObjectSpec::Tube {
                points,
                radius,
                color: c,
                colors,
                closed,
            } => {
                let mut tube = Tube::new(points, radius, color(c));
                if !colors.is_empty() {
                    tube = tube.with_colors(colors.into_iter().map(color).collect());
                }
                tube.closed = closed;
                Box::new(tube)
            }
            ObjectSpec::Text {
                content,
                position,
                font_size,
                color: c,
            } => Box::new(Text::new(content, position, font_size, color(c))),
            ObjectSpec::VectorText {
                content,
                position,
                scale,
                color: c,
            } => {
                let mut vt = VectorText::new(&content, crate::scene::font::default_font(), scale, color(c));
                vt.position = position;
                Box::new(vt)
            }
            ObjectSpec::LSystem {
                axiom,
                rules,
                theta,
                iterations,
                position,
                scale,
                color: c,
                colors,
            } => {
                let config = crate::scene::l_system::LSystemConfig {
                    axiom,
                    // Sanitize silently (spawn is infallible): empty symbols
                    // are dropped, longer ones use their first char.
                    rules: rules
                        .iter()
                        .filter_map(|(from, to)| {
                            Some(crate::scene::l_system::ReplacementRule {
                                from: from.chars().next()?,
                                to: to.clone(),
                            })
                        })
                        .collect(),
                };
                let mut ls = LSystem::new(config, theta, color(c)).with_colors(colors.iter().map(|v| color(*v)).collect());
                ls.iterations = iterations;
                ls.position = position;
                ls.scale = scale;
                Box::new(ls)
            }
            ObjectSpec::Plot {
                expression,
                position,
                size,
                x_bounds,
                y_bounds,
                color: c,
                samples,
            } => {
                let mut plot = Plot::new(&expression, position, size, x_bounds, y_bounds, color(c));
                plot.samples = samples;
                Box::new(plot)
            }
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TrackDoc {
    /// An `ObjectDoc.id`, or `"camera"` to target the camera.
    pub object: String,
    pub property: String,
    pub keyframes: Vec<KeyframeDoc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct KeyframeDoc {
    pub time: f32,
    pub value: AnimValue,
    #[serde(default)]
    pub easing: Easing,
    /// Arrive in this many equal sub-steps, each shaped by `easing` (see
    /// `Keyframe::steps`). Absent/1 = plain interpolation.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub steps: Option<u32>,
}

/// Locks `target.property` to `source.source_property` every frame. Either
/// end may be `"camera"`. Bindings may chain but not cycle (validated at
/// build).
///
/// An optional `start`/`end` window limits when the binding drives the
/// property. A windowed binding may coexist with a keyframe track on the
/// same property (the binding wins inside the window, the track outside),
/// and several bindings may share a property when their windows don't
/// overlap. An unwindowed binding owns the property outright — combining it
/// with a track is a build error.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct BindingDoc {
    /// An `ObjectDoc.id`, or `"camera"`.
    pub target: String,
    pub property: String,
    /// An `ObjectDoc.id`, or `"camera"`. Must differ from `target`.
    pub source: String,
    pub source_property: String,
    /// Added to the source value each frame (component-wise;
    /// Float/Vec2/Vec3/Vec4 properties only).
    #[serde(default)]
    pub offset: Option<AnimValue>,
    /// Window start in seconds (inclusive); `None` = from the beginning.
    #[serde(default)]
    pub start: Option<f32>,
    /// Window end in seconds (exclusive); `None` = forever.
    #[serde(default)]
    pub end: Option<f32>,
}

impl BindingDoc {
    /// Whether the binding drives its target at `time` (mirrors
    /// `Binding::active_at`).
    pub fn active_at(&self, time: f32) -> bool {
        self.start.is_none_or(|s| time >= s) && self.end.is_none_or(|e| time < e)
    }

    pub fn is_windowed(&self) -> bool {
        self.start.is_some() || self.end.is_some()
    }
}

/// Create a new empty scene document under `scenes/`, returning its
/// registry name (the file stem).
pub fn create_untitled() -> Result<String, String> {
    std::fs::create_dir_all("scenes").map_err(|e| format!("cannot create scenes/: {e}"))?;
    let mut n = 1;
    loop {
        let name = format!("untitled_{n}");
        let path = format!("scenes/{name}.ron");
        if !std::path::Path::new(&path).exists() {
            let doc = SceneDoc {
                description: "New scene".to_string(),
                camera: CameraDoc::default(),
                objects: Vec::new(),
                tracks: Vec::new(),
                bindings: Vec::new(),
                export: ExportDefaults::default(),
            };
            std::fs::write(&path, doc.to_ron_string()?).map_err(|e| format!("cannot write {path}: {e}"))?;
            return Ok(name);
        }
        n += 1;
    }
}

impl SceneDoc {
    pub fn from_ron_str(source: &str) -> Result<Self, String> {
        ron::from_str(source).map_err(|e| format!("RON parse error: {e}"))
    }

    pub fn load(path: &str) -> Result<Self, String> {
        let source = std::fs::read_to_string(path).map_err(|e| format!("cannot read {path}: {e}"))?;
        Self::from_ron_str(&source).map_err(|e| format!("{path}: {e}"))
    }

    pub fn to_ron_string(&self) -> Result<String, String> {
        ron::ser::to_string_pretty(self, ron::ser::PrettyConfig::default()).map_err(|e| format!("RON serialize error: {e}"))
    }

    /// Build the runtime scene. Validates object ids, property names, and
    /// value types, returning a descriptive error instead of panicking.
    pub fn build(&self) -> Result<(Scene, Timeline, Camera), String> {
        let mut scene = Scene::new();
        let mut ids = std::collections::HashMap::new();

        for obj_doc in &self.objects {
            if ids.contains_key(obj_doc.id.as_str()) {
                return Err(format!("duplicate object id {:?}", obj_doc.id));
            }
            if obj_doc.id == "camera" {
                return Err("object id \"camera\" is reserved for camera tracks".to_string());
            }
            let id = scene.add_boxed(obj_doc.object.spawn());
            ids.insert(obj_doc.id.as_str(), id);

            for (property, value) in &obj_doc.set {
                let object = scene.get_mut(id).unwrap();
                validate_property(object, &obj_doc.id, property, value)?;
                object.set(property, value.clone());
            }
        }

        let mut timeline = Timeline::new();
        let mut camera = Camera::new(self.camera.position, self.camera.target);
        camera.fov = self.camera.fov;
        for (property, value) in &self.camera.set {
            validate_property(&camera, "camera", property, value)?;
            camera.set(property, value.clone());
        }

        for track_doc in &self.tracks {
            let mut track = if track_doc.object == "camera" {
                for kf in &track_doc.keyframes {
                    validate_property(&camera, "camera", &track_doc.property, &kf.value)?;
                }
                Track::camera(track_doc.property.clone())
            } else {
                let id = *ids
                    .get(track_doc.object.as_str())
                    .ok_or_else(|| format!("track references unknown object id {:?}", track_doc.object))?;
                let object = scene.get(id).unwrap();
                for kf in &track_doc.keyframes {
                    validate_property(object, &track_doc.object, &track_doc.property, &kf.value)?;
                }
                Track::new(id, track_doc.property.clone())
            };
            for kf in &track_doc.keyframes {
                track.add_keyframe(Keyframe::with_easing(kf.time, kf.value.clone(), kf.easing).with_steps(kf.steps.unwrap_or(1)));
            }
            timeline.add_track(track);
        }

        for b in &self.bindings {
            let resolve = |name: &str| -> Result<TrackTarget, String> {
                if name == "camera" {
                    Ok(TrackTarget::Camera)
                } else {
                    ids.get(name)
                        .map(|id| TrackTarget::Object(*id))
                        .ok_or_else(|| format!("binding references unknown object id {:?}", name))
                }
            };
            let target = resolve(&b.target)?;
            let source = resolve(&b.source)?;
            if target == source {
                return Err(format!("binding on {:?} cannot source its own target object", b.target));
            }

            // The target property must be settable; the source only readable
            // (which will include derived output properties).
            let expected = match target {
                TrackTarget::Object(id) => {
                    let object = scene.get(id).unwrap();
                    if !object.property_names().contains(&b.property.as_str()) {
                        return Err(format!(
                            "object {:?} has no settable property {:?} (valid: {:?})",
                            b.target,
                            b.property,
                            object.property_names()
                        ));
                    }
                    object.get(&b.property).unwrap()
                }
                TrackTarget::Camera => {
                    if !camera.property_names().contains(&b.property.as_str()) {
                        return Err(format!(
                            "camera has no settable property {:?} (valid: {:?})",
                            b.property,
                            camera.property_names()
                        ));
                    }
                    camera.get(&b.property).unwrap()
                }
            };
            let source_value = match source {
                TrackTarget::Object(id) => scene.get(id).unwrap().get(&b.source_property),
                TrackTarget::Camera => camera.get(&b.source_property),
            }
            .ok_or_else(|| format!("object {:?} has no readable property {:?}", b.source, b.source_property))?;

            if std::mem::discriminant(&expected) != std::mem::discriminant(&source_value) {
                return Err(format!(
                    "binding type mismatch: {:?}.{:?} expects {:?}-like values, but {:?}.{:?} is {:?}",
                    b.target, b.property, expected, b.source, b.source_property, source_value
                ));
            }
            if let Some(offset) = &b.offset {
                if std::mem::discriminant(&expected) != std::mem::discriminant(offset) {
                    return Err(format!(
                        "binding offset type mismatch: {:?}.{:?} expects {:?}-like values, got offset {:?}",
                        b.target, b.property, expected, offset
                    ));
                }
                if !expected.supports_offset() {
                    return Err(format!(
                        "binding offset on {:?}.{:?}: {:?}-like values do not support offsets",
                        b.target, b.property, expected
                    ));
                }
            }

            if let (Some(start), Some(end)) = (b.start, b.end)
                && start >= end
            {
                return Err(format!(
                    "binding window on {:?}.{:?}: start {start} must be before end {end}",
                    b.target, b.property
                ));
            }

            let binding = Binding {
                target,
                target_property: b.property.clone(),
                source,
                source_property: b.source_property.clone(),
                offset: b.offset.clone(),
                start: b.start,
                end: b.end,
            };

            // Same-property bindings must not be active at the same time…
            if timeline.bindings.iter().any(|other| {
                other.target == binding.target && other.target_property == binding.target_property && other.window_overlaps(&binding)
            }) {
                return Err(format!(
                    "overlapping bindings for property {:?} on {:?} — adjust their windows",
                    b.property, b.target
                ));
            }
            // …and only a *windowed* binding may share its property with a
            // track (the track drives it outside the window).
            if !binding.is_windowed() && self.tracks.iter().any(|t| t.object == b.target && t.property == b.property) {
                return Err(format!(
                    "property {:?} on {:?} is both keyframed and always-bound — window the binding or remove the track",
                    b.property, b.target
                ));
            }

            timeline.add_binding(binding);
        }

        if let Err(cycle) = timeline.sort_bindings() {
            let names: Vec<String> = cycle
                .iter()
                .map(|t| match t {
                    TrackTarget::Camera => "camera".to_string(),
                    TrackTarget::Object(id) => ids
                        .iter()
                        .find(|(_, v)| *v == id)
                        .map(|(k, _)| k.to_string())
                        .unwrap_or_else(|| format!("{id:?}")),
                })
                .collect();
            return Err(format!("binding cycle between objects: {}", names.join(" -> ")));
        }

        Ok((scene, timeline, camera))
    }
}

fn validate_property(object: &(impl Animatable + ?Sized), object_id: &str, property: &str, value: &AnimValue) -> Result<(), String> {
    let Some(current) = object.get(property) else {
        return Err(format!(
            "object {:?} has no property {:?} (valid: {:?})",
            object_id,
            property,
            object.property_names()
        ));
    };
    if std::mem::discriminant(&current) != std::mem::discriminant(value) {
        return Err(format!(
            "property {:?} on object {:?} expects {:?}-like values, got {:?}",
            property, object_id, current, value
        ));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn build_err(doc: &SceneDoc) -> String {
        match doc.build() {
            Err(e) => e,
            Ok(_) => panic!("expected build error"),
        }
    }

    fn minimal_doc() -> SceneDoc {
        SceneDoc {
            description: "test".to_string(),
            camera: CameraDoc::default(),
            export: ExportDefaults::default(),
            bindings: Vec::new(),
            objects: vec![ObjectDoc {
                id: "ball".to_string(),
                object: ObjectSpec::Disk {
                    position: Vec3::ZERO,
                    radius: 1.0,
                    color: vec4(1.0, 0.0, 0.0, 1.0),
                },
                set: vec![],
            }],
            tracks: vec![TrackDoc {
                object: "ball".to_string(),
                property: "radius".to_string(),
                keyframes: vec![
                    KeyframeDoc {
                        time: 0.0,
                        value: AnimValue::Float(1.0),
                        easing: Easing::QuadOut,
                        steps: None,
                    },
                    KeyframeDoc {
                        time: 2.0,
                        value: AnimValue::Float(3.0),
                        easing: Easing::Linear,
                        steps: None,
                    },
                ],
            }],
        }
    }

    #[test]
    fn build_minimal_doc() {
        let (scene, timeline, _camera) = minimal_doc().build().unwrap();
        assert_eq!(scene.len(), 1);
        assert!((timeline.duration() - 2.0).abs() < f32::EPSILON);
    }

    #[test]
    fn ron_round_trip() {
        let doc = minimal_doc();
        let ron_str = doc.to_ron_string().unwrap();
        let parsed = SceneDoc::from_ron_str(&ron_str).unwrap();
        assert_eq!(parsed.objects.len(), 1);
        assert_eq!(parsed.tracks[0].keyframes[0].easing, Easing::QuadOut);
        parsed.build().unwrap();
    }

    #[test]
    fn set_override_applies() {
        let mut doc = minimal_doc();
        doc.objects[0].set = vec![("radius".to_string(), AnimValue::Float(9.0))];
        let (scene, _, _) = doc.build().unwrap();
        let (_, obj) = scene.iter().next().unwrap();
        assert_eq!(obj.get("radius"), Some(AnimValue::Float(9.0)));
    }

    #[test]
    fn unknown_property_errors() {
        let mut doc = minimal_doc();
        doc.tracks[0].property = "raidus".to_string();
        let err = build_err(&doc);
        assert!(err.contains("raidus"), "unexpected error: {err}");
    }

    #[test]
    fn wrong_value_type_errors() {
        let mut doc = minimal_doc();
        doc.tracks[0].keyframes[0].value = AnimValue::Vec3(Vec3::ZERO);
        let err = build_err(&doc);
        assert!(err.contains("radius"), "unexpected error: {err}");
    }

    #[test]
    fn unknown_track_object_errors() {
        let mut doc = minimal_doc();
        doc.tracks[0].object = "ghost".to_string();
        let err = build_err(&doc);
        assert!(err.contains("ghost"), "unexpected error: {err}");
    }

    #[test]
    fn duplicate_object_id_errors() {
        let mut doc = minimal_doc();
        doc.tracks.clear();
        let dup = doc.objects[0].clone();
        doc.objects.push(dup);
        assert!(build_err(&doc).contains("duplicate"));
    }

    #[test]
    fn camera_track_builds() {
        let mut doc = minimal_doc();
        doc.tracks = vec![TrackDoc {
            object: "camera".to_string(),
            property: "fov".to_string(),
            keyframes: vec![KeyframeDoc {
                time: 0.0,
                value: AnimValue::Float(60.0),
                easing: Easing::Linear,
                steps: None,
            }],
        }];
        doc.build().unwrap();
    }

    #[test]
    fn vector_text_spec_builds_and_round_trips() {
        let mut doc = minimal_doc();
        doc.objects.push(ObjectDoc {
            id: "title".to_string(),
            object: ObjectSpec::VectorText {
                content: "hi".to_string(),
                position: vec3(-1.0, 2.0, 0.0),
                scale: 1.5,
                color: vec4(1.0, 1.0, 1.0, 1.0),
            },
            set: vec![("stagger".to_string(), AnimValue::Float(0.5))],
        });
        doc.tracks.push(TrackDoc {
            object: "title".to_string(),
            property: "progress".to_string(),
            keyframes: vec![KeyframeDoc {
                time: 1.0,
                value: AnimValue::Float(1.0),
                easing: Easing::Linear,
                steps: None,
            }],
        });

        let ron_str = doc.to_ron_string().unwrap();
        let parsed = SceneDoc::from_ron_str(&ron_str).unwrap();
        let (scene, _, _) = parsed.build().unwrap();
        let (_, vt) = scene.iter().nth(1).unwrap();
        assert_eq!(vt.get("position"), Some(AnimValue::Vec3(vec3(-1.0, 2.0, 0.0))));
        assert_eq!(vt.get("scale"), Some(AnimValue::Float(1.5)));
        assert_eq!(vt.get("stagger"), Some(AnimValue::Float(0.5)));
        // Glyph outlines were actually extracted from the default font.
        let bb = vt.bounding_box();
        assert!(bb.max.x > bb.min.x);
    }

    /// The spec-spawned LSystem must be geometry-identical to one built
    /// directly from the dragon_curve() preset with the same parameters.
    #[test]
    fn lsystem_spec_matches_reference_construction() {
        use crate::scene::objects::LSystem;
        use crate::scene::traits::{Animatable, SceneObject};
        let (config, theta) = crate::scene::l_system::dragon_curve();
        let mut reference = LSystem::new(config, theta, macroquad::prelude::WHITE);
        reference.iterations = 10.0;
        reference.scale = 0.15;
        reference.position = macroquad::prelude::vec3(1.0, -2.0, 0.0);

        let spec = ObjectSpec::LSystem {
            axiom: "F".to_string(),
            rules: vec![("F".to_string(), "F+G".to_string()), ("G".to_string(), "F-G".to_string())],
            theta: std::f32::consts::FRAC_PI_2,
            iterations: 10.0,
            position: vec3(1.0, -2.0, 0.0),
            scale: 0.15,
            color: vec4(1.0, 1.0, 1.0, 1.0),
            colors: vec![],
        };
        let spawned = spec.spawn();

        let rb = reference.bounding_box();
        let sb = spawned.bounding_box();
        assert!((rb.min - sb.min).length() < 1e-4 && (rb.max - sb.max).length() < 1e-4);
        assert_eq!(reference.get("pen_position"), spawned.get("pen_position"));
    }

    #[test]
    fn lsystem_spec_builds_and_round_trips() {
        let mut doc = minimal_doc();
        doc.objects.push(ObjectDoc {
            id: "dragon".to_string(),
            object: ObjectSpec::LSystem {
                axiom: "F".to_string(),
                rules: vec![("F".to_string(), "F+G".to_string()), ("G".to_string(), "F-G".to_string())],
                theta: std::f32::consts::FRAC_PI_2,
                iterations: 4.0,
                position: vec3(1.0, 2.0, 0.0),
                scale: 0.5,
                color: vec4(0.2, 0.5, 1.0, 1.0),
                colors: vec![vec4(1.0, 0.0, 0.0, 1.0), vec4(0.0, 0.0, 1.0, 1.0)],
            },
            set: vec![],
        });

        let ron_str = doc.to_ron_string().unwrap();
        let parsed = SceneDoc::from_ron_str(&ron_str).unwrap();
        let (scene, _, _) = parsed.build().unwrap();
        let (_, ls) = scene.iter().nth(1).unwrap();
        assert_eq!(ls.get("iterations"), Some(AnimValue::Float(4.0)));
        assert_eq!(ls.get("scale"), Some(AnimValue::Float(0.5)));
        // The rules were actually applied: 4 dragon iterations = 16 segments,
        // giving a non-degenerate bounding box away from the origin.
        let bb = ls.bounding_box();
        assert!(bb.max.x > bb.min.x && bb.max.y > bb.min.y);
        // pen_position output is live (binding source).
        assert!(matches!(ls.get("pen_position"), Some(AnimValue::Vec3(_))));
    }

    #[test]
    fn lsystem_spec_sanitizes_bad_rules_instead_of_failing() {
        let mut doc = minimal_doc();
        doc.objects.push(ObjectDoc {
            id: "weird".to_string(),
            object: ObjectSpec::LSystem {
                // Empty rule symbol (dropped), multi-char symbol (first char
                // used), unmatched bracket in the axiom (ignored at draw).
                axiom: "]F[".to_string(),
                rules: vec![("".to_string(), "F".to_string()), ("Fx".to_string(), "F+F".to_string())],
                theta: 1.0,
                iterations: 2.0,
                position: Vec3::ZERO,
                scale: 1.0,
                color: vec4(1.0, 1.0, 1.0, 1.0),
                colors: vec![],
            },
            set: vec![],
        });
        let (scene, _, _) = doc.build().unwrap();
        let (_, ls) = scene.iter().nth(1).unwrap();
        // F -> F+F applied twice: 4 drawing chars.
        let bb = ls.bounding_box();
        assert!(bb.max != bb.min, "expected some drawn segments");
    }

    #[test]
    fn plot_spec_builds_and_round_trips() {
        let mut doc = minimal_doc();
        doc.objects.push(ObjectDoc {
            id: "graph".to_string(),
            object: ObjectSpec::Plot {
                expression: "x^2 - 1".to_string(),
                position: vec3(0.0, 1.0, 0.0),
                size: vec2(6.0, 4.0),
                x_bounds: vec2(-2.0, 2.0),
                y_bounds: vec2(-1.5, 3.5),
                color: vec4(0.3, 0.8, 1.0, 1.0),
                samples: 64,
            },
            set: vec![("progress".to_string(), AnimValue::Float(0.5))],
        });

        let ron_str = doc.to_ron_string().unwrap();
        let parsed = SceneDoc::from_ron_str(&ron_str).unwrap();
        let (scene, _, _) = parsed.build().unwrap();
        let (_, plot) = scene.iter().nth(1).unwrap();
        assert_eq!(plot.get("x_bounds"), Some(AnimValue::Vec2(vec2(-2.0, 2.0))));
        assert_eq!(plot.get("progress"), Some(AnimValue::Float(0.5)));
        // The pen output is live and sits inside the plot rect.
        let Some(AnimValue::Vec3(pen)) = plot.get("pen_position") else {
            panic!("expected pen_position output");
        };
        assert!(pen.x.abs() <= 3.0 && (pen.y - 1.0).abs() <= 2.0);
    }

    #[test]
    fn plot_spec_with_bad_expression_still_builds() {
        let mut doc = minimal_doc();
        doc.objects.push(ObjectDoc {
            id: "graph".to_string(),
            object: ObjectSpec::Plot {
                expression: "wat(".to_string(),
                position: Vec3::ZERO,
                size: vec2(4.0, 2.0),
                x_bounds: vec2(-1.0, 1.0),
                y_bounds: vec2(-1.0, 1.0),
                color: vec4(1.0, 1.0, 1.0, 1.0),
                samples: 32,
            },
            set: vec![],
        });
        // Axes-only fallback, never a build error (spawn is infallible).
        doc.build().unwrap();
    }

    #[test]
    fn keyframe_steps_round_trip_and_drive_evaluation() {
        let mut doc = minimal_doc();
        doc.tracks[0].keyframes[1].steps = Some(4);

        let ron_str = doc.to_ron_string().unwrap();
        assert!(ron_str.contains("steps"), "steps should serialize when set");
        let parsed = SceneDoc::from_ron_str(&ron_str).unwrap();
        assert_eq!(parsed.tracks[0].keyframes[1].steps, Some(4));

        // radius 1 -> 3 over 2s in 4 steps: at t=1.0 (two full steps) the
        // staircase sits exactly halfway.
        let (mut scene, timeline, mut camera) = parsed.build().unwrap();
        timeline.apply(1.0, &mut scene, &mut camera);
        let (_, ball) = scene.iter().next().unwrap();
        assert_eq!(ball.get("radius"), Some(AnimValue::Float(2.0)));

        // Unset steps stays out of the file entirely.
        doc.tracks[0].keyframes[1].steps = None;
        assert!(!doc.to_ron_string().unwrap().contains("steps"));
    }

    #[test]
    fn custom_easing_does_not_serialize() {
        let mut doc = minimal_doc();
        doc.tracks[0].keyframes[0].easing = Easing::Custom(crate::animation::easing::linear);
        assert!(doc.to_ron_string().is_err());
    }

    /// minimal_doc plus a second disk whose radius is bound to ball.radius
    /// (tracked 1.0 → 3.0 over 2s) with a +0.5 offset.
    fn bound_doc() -> SceneDoc {
        let mut doc = minimal_doc();
        doc.objects.push(ObjectDoc {
            id: "shadow".to_string(),
            object: ObjectSpec::Disk {
                position: Vec3::ZERO,
                radius: 1.0,
                color: vec4(0.5, 0.5, 0.5, 1.0),
            },
            set: vec![],
        });
        doc.bindings.push(BindingDoc {
            target: "shadow".to_string(),
            property: "radius".to_string(),
            source: "ball".to_string(),
            source_property: "radius".to_string(),
            offset: Some(AnimValue::Float(0.5)),
            start: None,
            end: None,
        });
        doc
    }

    #[test]
    fn binding_builds_and_applies() {
        let (mut scene, timeline, mut camera) = bound_doc().build().unwrap();
        timeline.apply(2.0, &mut scene, &mut camera);
        let (_, shadow) = scene.iter().nth(1).unwrap();
        assert_eq!(shadow.get("radius"), Some(AnimValue::Float(3.5)));
    }

    #[test]
    fn binding_round_trips_through_ron() {
        let doc = bound_doc();
        let ron_str = doc.to_ron_string().unwrap();
        let parsed = SceneDoc::from_ron_str(&ron_str).unwrap();
        assert_eq!(parsed.bindings, doc.bindings);
        parsed.build().unwrap();
    }

    #[test]
    fn docs_without_bindings_section_still_parse() {
        let ron_str = "(objects: [], tracks: [])";
        let parsed = SceneDoc::from_ron_str(ron_str).unwrap();
        assert!(parsed.bindings.is_empty());
    }

    #[test]
    fn binding_unknown_object_errors() {
        let mut doc = bound_doc();
        doc.bindings[0].source = "ghost".to_string();
        assert!(build_err(&doc).contains("ghost"));
    }

    #[test]
    fn binding_self_source_errors() {
        let mut doc = bound_doc();
        doc.bindings[0].source = "shadow".to_string();
        assert!(build_err(&doc).contains("its own target object"));
    }

    #[test]
    fn binding_unknown_target_property_errors() {
        let mut doc = bound_doc();
        doc.bindings[0].property = "raidus".to_string();
        assert!(build_err(&doc).contains("raidus"));
    }

    #[test]
    fn binding_type_mismatch_errors() {
        let mut doc = bound_doc();
        doc.bindings[0].property = "position".to_string();
        assert!(build_err(&doc).contains("type mismatch"));
    }

    #[test]
    fn binding_offset_type_mismatch_errors() {
        let mut doc = bound_doc();
        doc.bindings[0].offset = Some(AnimValue::Vec3(Vec3::ZERO));
        assert!(build_err(&doc).contains("offset type mismatch"));
    }

    #[test]
    fn unwindowed_binding_on_tracked_property_errors() {
        let mut doc = bound_doc();
        // ball.radius already has a track in minimal_doc.
        doc.bindings[0].target = "ball".to_string();
        doc.bindings[0].source = "shadow".to_string();
        assert!(build_err(&doc).contains("keyframed and always-bound"));
        // Windowing the binding resolves the conflict.
        doc.bindings[0].end = Some(1.0);
        doc.build().unwrap();
    }

    #[test]
    fn overlapping_bindings_error_disjoint_windows_build() {
        let mut doc = bound_doc();
        // Same property twice, both unwindowed: overlap.
        let mut dup = doc.bindings[0].clone();
        dup.source_property = "radius".to_string();
        doc.bindings.push(dup);
        assert!(build_err(&doc).contains("overlapping bindings"));

        // Disjoint windows on the same property are fine.
        doc.bindings[0].end = Some(2.0);
        doc.bindings[1].start = Some(2.0);
        doc.build().unwrap();

        // Touch again and they overlap.
        doc.bindings[1].start = Some(1.5);
        assert!(build_err(&doc).contains("overlapping bindings"));
    }

    #[test]
    fn inverted_binding_window_errors() {
        let mut doc = bound_doc();
        doc.bindings[0].start = Some(5.0);
        doc.bindings[0].end = Some(2.0);
        assert!(build_err(&doc).contains("start"));
    }

    #[test]
    fn windowed_binding_hands_off_to_track() {
        // The requested camera flow: bound to the ball's radius until t=2,
        // keyframed on its own track afterwards.
        let mut doc = bound_doc();
        doc.bindings[0].target = "camera".to_string();
        doc.bindings[0].property = "fov".to_string();
        doc.bindings[0].offset = None;
        doc.bindings[0].end = Some(2.0);
        doc.tracks.push(TrackDoc {
            object: "camera".to_string(),
            property: "fov".to_string(),
            keyframes: vec![
                KeyframeDoc {
                    time: 2.0,
                    value: AnimValue::Float(90.0),
                    easing: Easing::Linear,
                    steps: None,
                },
                KeyframeDoc {
                    time: 4.0,
                    value: AnimValue::Float(30.0),
                    easing: Easing::Linear,
                    steps: None,
                },
            ],
        });

        let (mut scene, timeline, mut camera) = doc.build().unwrap();
        // Inside the window: fov = ball.radius (tracked 1.0 -> 3.0 over 2s),
        // overriding the fov track.
        timeline.apply(1.0, &mut scene, &mut camera);
        assert!((camera.fov - 2.0).abs() < 1e-5);
        // After the window: the fov track owns it again.
        timeline.apply(3.0, &mut scene, &mut camera);
        assert!((camera.fov - 60.0).abs() < 1e-5);
    }

    #[test]
    fn binding_window_round_trips_through_ron() {
        let mut doc = bound_doc();
        doc.bindings[0].start = Some(1.0);
        doc.bindings[0].end = Some(3.5);
        let parsed = SceneDoc::from_ron_str(&doc.to_ron_string().unwrap()).unwrap();
        assert_eq!(parsed.bindings, doc.bindings);
    }

    #[test]
    fn camera_set_overrides_apply_and_validate() {
        let mut doc = minimal_doc();
        doc.camera.set = vec![("rotation_z".to_string(), AnimValue::Float(0.5))];
        let (_, _, camera) = doc.build().unwrap();
        assert_eq!(camera.get("rotation_z"), Some(AnimValue::Float(0.5)));

        doc.camera.set = vec![("rotation_zz".to_string(), AnimValue::Float(0.5))];
        assert!(build_err(&doc).contains("rotation_zz"));
    }

    #[test]
    fn binding_cycle_errors_with_object_names() {
        let mut doc = bound_doc();
        doc.tracks.clear();
        doc.bindings.push(BindingDoc {
            target: "ball".to_string(),
            property: "position".to_string(),
            source: "shadow".to_string(),
            source_property: "position".to_string(),
            offset: None,
            start: None,
            end: None,
        });
        let err = build_err(&doc);
        assert!(err.contains("binding cycle"), "unexpected error: {err}");
        assert!(err.contains("ball") && err.contains("shadow"), "unexpected error: {err}");
    }

    #[test]
    fn binding_may_target_camera() {
        let mut doc = bound_doc();
        doc.bindings.push(BindingDoc {
            target: "camera".to_string(),
            property: "fov".to_string(),
            source: "ball".to_string(),
            source_property: "radius".to_string(),
            offset: None,
            start: None,
            end: None,
        });
        let (mut scene, timeline, mut camera) = doc.build().unwrap();
        timeline.apply(2.0, &mut scene, &mut camera);
        assert!((camera.fov - 3.0).abs() < 1e-5);
    }
}
