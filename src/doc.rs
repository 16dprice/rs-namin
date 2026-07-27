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
/// non-data constructor inputs (custom fonts, textures, LaTeX, L-system
/// configs) are not yet representable.
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
}

/// Locks `target.property` to `source.source_property` every frame. Either
/// end may be `"camera"`. A bound property cannot also have a track, and
/// bindings may chain but not cycle — both validated at build.
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
                track.add_keyframe(Keyframe::with_easing(kf.time, kf.value.clone(), kf.easing));
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

            if self
                .bindings
                .iter()
                .filter(|other| other.target == b.target && other.property == b.property)
                .count()
                > 1
            {
                return Err(format!("duplicate binding for property {:?} on {:?}", b.property, b.target));
            }
            if self.tracks.iter().any(|t| t.object == b.target && t.property == b.property) {
                return Err(format!(
                    "property {:?} on {:?} is both keyframed and bound — remove the track or the binding",
                    b.property, b.target
                ));
            }

            timeline.add_binding(Binding {
                target,
                target_property: b.property.clone(),
                source,
                source_property: b.source_property.clone(),
                offset: b.offset.clone(),
            });
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
                    },
                    KeyframeDoc {
                        time: 2.0,
                        value: AnimValue::Float(3.0),
                        easing: Easing::Linear,
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
    fn binding_on_tracked_property_errors() {
        let mut doc = bound_doc();
        // ball.radius already has a track in minimal_doc.
        doc.bindings[0].target = "ball".to_string();
        doc.bindings[0].source = "shadow".to_string();
        assert!(build_err(&doc).contains("both keyframed and bound"));
    }

    #[test]
    fn duplicate_binding_errors() {
        let mut doc = bound_doc();
        let dup = doc.bindings[0].clone();
        doc.bindings.push(dup);
        assert!(build_err(&doc).contains("duplicate binding"));
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
        });
        let (mut scene, timeline, mut camera) = doc.build().unwrap();
        timeline.apply(2.0, &mut scene, &mut camera);
        assert!((camera.fov - 3.0).abs() < 1e-5);
    }
}
