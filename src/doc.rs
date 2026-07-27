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

use crate::animation::easing::Easing;
use crate::animation::timeline::Timeline;
use crate::animation::track::{Keyframe, Track};
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
/// Objects with non-data constructor inputs (fonts, textures, L-system
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
    fn custom_easing_does_not_serialize() {
        let mut doc = minimal_doc();
        doc.tracks[0].keyframes[0].easing = Easing::Custom(crate::animation::easing::linear);
        assert!(doc.to_ron_string().is_err());
    }
}
