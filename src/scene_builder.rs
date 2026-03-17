use std::mem;

use crate::animation::easing::EasingFn;
use crate::animation::timeline::Timeline;
use crate::animation::track::{Keyframe, Track};
use crate::camera::Camera;
use crate::scene::traits::Animatable;
use crate::scene::value::AnimValue;
use crate::scene::{ObjectId, Scene, SceneNode};

/// Builder for constructing scenes with validated property names and types.
///
/// Catches property name typos and type mismatches at scene construction time
/// rather than failing silently at runtime.
pub struct SceneBuilder {
    scene: Scene,
    timeline: Timeline,
    camera: Camera,
}

impl SceneBuilder {
    pub fn new() -> Self {
        Self {
            scene: Scene::new(),
            timeline: Timeline::new(),
            camera: Camera::default(),
        }
    }

    /// Add a scene object and get back an `ObjRef` for creating animations.
    pub fn add(&mut self, object: impl SceneNode + 'static) -> ObjRef {
        let id = self.scene.add(object);
        ObjRef { id }
    }

    /// Set the camera for the scene.
    pub fn camera(&mut self, camera: Camera) -> &mut Self {
        self.camera = camera;
        self
    }

    /// Animate a property on a scene object. Validates that the property exists
    /// and that keyframe values match the property's type.
    ///
    /// # Panics
    /// Panics if the property name doesn't exist on the object, or if a keyframe
    /// value has the wrong AnimValue variant.
    pub fn animate(
        &mut self,
        obj: &ObjRef,
        property: &str,
        build: impl FnOnce(TrackBuilder) -> TrackBuilder,
    ) -> &mut Self {
        let scene_obj = self.scene.get(obj.id).unwrap_or_else(|| {
            panic!(
                "SceneBuilder::animate: object {:?} not found in scene",
                obj.id
            )
        });

        // Validate property name
        let names = scene_obj.property_names();
        assert!(
            names.contains(&property),
            "SceneBuilder::animate: property \"{}\" not found on object {:?}. Valid properties: {:?}",
            property,
            obj.id,
            names,
        );

        // Get expected variant for type checking
        let expected = scene_obj.get(property).unwrap();

        let tb = TrackBuilder::new(&expected);
        let tb = build(tb);
        let keyframes = tb.finish();

        let mut track = Track::new(obj.id, property);
        for kf in keyframes {
            track.add_keyframe(kf);
        }
        self.timeline.add_track(track);
        self
    }

    /// Animate a camera property. Validates that the property exists on Camera.
    ///
    /// # Panics
    /// Panics if the property name doesn't exist on Camera, or if a keyframe
    /// value has the wrong AnimValue variant.
    pub fn animate_camera(
        &mut self,
        property: &str,
        build: impl FnOnce(TrackBuilder) -> TrackBuilder,
    ) -> &mut Self {
        let names = self.camera.property_names();
        assert!(
            names.contains(&property),
            "SceneBuilder::animate_camera: property \"{}\" not found on Camera. Valid properties: {:?}",
            property,
            names,
        );

        let expected = self.camera.get(property).unwrap();

        let tb = TrackBuilder::new(&expected);
        let tb = build(tb);
        let keyframes = tb.finish();

        let mut track = Track::camera(property);
        for kf in keyframes {
            track.add_keyframe(kf);
        }
        self.timeline.add_track(track);
        self
    }

    /// Consume the builder and return the scene, timeline, and camera.
    pub fn build(mut self) -> (Scene, Timeline, Camera) {
        let scene = mem::take(&mut self.scene);
        let timeline = mem::take(&mut self.timeline);
        (scene, timeline, self.camera)
    }
}

impl Default for SceneBuilder {
    fn default() -> Self {
        Self::new()
    }
}

/// Reference to an object added via SceneBuilder.
#[derive(Debug, Clone, Copy)]
pub struct ObjRef {
    pub id: ObjectId,
}

/// Builder for constructing keyframes within a track, with type validation.
pub struct TrackBuilder {
    keyframes: Vec<Keyframe>,
    expected_variant: &'static str,
}

impl TrackBuilder {
    fn new(expected: &AnimValue) -> Self {
        Self {
            keyframes: Vec::new(),
            expected_variant: variant_name(expected),
        }
    }

    /// Add a keyframe with linear easing.
    pub fn keyframe(mut self, time: f32, value: AnimValue) -> Self {
        self.validate_type(&value);
        self.keyframes.push(Keyframe::new(time, value));
        self
    }

    /// Add a keyframe with a custom easing function.
    pub fn keyframe_with_easing(mut self, time: f32, value: AnimValue, easing: EasingFn) -> Self {
        self.validate_type(&value);
        self.keyframes
            .push(Keyframe::with_easing(time, value, easing));
        self
    }

    fn validate_type(&self, value: &AnimValue) {
        let actual = variant_name(value);
        assert_eq!(
            actual, self.expected_variant,
            "SceneBuilder: keyframe value type mismatch — expected {}, got {}",
            self.expected_variant, actual,
        );
    }

    fn finish(self) -> Vec<Keyframe> {
        self.keyframes
    }
}

fn variant_name(v: &AnimValue) -> &'static str {
    match v {
        AnimValue::Float(_) => "Float",
        AnimValue::Vec2(_) => "Vec2",
        AnimValue::Vec3(_) => "Vec3",
        AnimValue::Vec4(_) => "Vec4",
        AnimValue::Bool(_) => "Bool",
        AnimValue::Transform2D(_) => "Transform2D",
        AnimValue::Mat4(_) => "Mat4",
    }
}

#[cfg(test)]
mod tests {
    use macroquad::prelude::{Mat4, RED, Vec3, WHITE, vec2, vec3, vec4};

    use super::*;
    use crate::animation::easing::quad_out;
    use crate::camera::{Camera, ProjectionMode};
    use crate::scene::objects::{
        Arc, Arrow, Disk, Line, Polygon, Rectangle, Spiral, Text, Torus, Tube, VectorText,
    };

    #[test]
    fn build_scene_with_object() {
        let mut sb = SceneBuilder::new();
        let _circle = sb.add(Disk::new(Vec3::ZERO, 1.0, WHITE));
        let (scene, timeline, _camera) = sb.build();
        assert_eq!(scene.len(), 1);
        assert_eq!(timeline.duration(), 0.0);
    }

    #[test]
    fn animate_valid_property() {
        let mut sb = SceneBuilder::new();
        let circle = sb.add(Disk::new(Vec3::ZERO, 1.0, WHITE));
        sb.animate(&circle, "radius", |tb| {
            tb.keyframe(0.0, AnimValue::Float(1.0))
                .keyframe(2.0, AnimValue::Float(5.0))
        });
        let (_scene, timeline, _camera) = sb.build();
        assert!((timeline.duration() - 2.0).abs() < f32::EPSILON);
    }

    #[test]
    #[should_panic(expected = "property \"raidus\" not found")]
    fn animate_invalid_property_name_panics() {
        let mut sb = SceneBuilder::new();
        let circle = sb.add(Disk::new(Vec3::ZERO, 1.0, WHITE));
        sb.animate(&circle, "raidus", |tb| {
            tb.keyframe(0.0, AnimValue::Float(1.0))
        });
    }

    #[test]
    #[should_panic(expected = "keyframe value type mismatch")]
    fn animate_wrong_type_panics() {
        let mut sb = SceneBuilder::new();
        let circle = sb.add(Disk::new(Vec3::ZERO, 1.0, WHITE));
        sb.animate(&circle, "radius", |tb| {
            // radius is Float, passing Vec3
            tb.keyframe(0.0, AnimValue::Vec3(Vec3::ZERO))
        });
    }

    #[test]
    fn animate_camera_valid_property() {
        let mut sb = SceneBuilder::new();
        sb.camera(Camera::new(vec3(0.0, 0.0, 10.0), Vec3::ZERO));
        sb.animate_camera("position", |tb| {
            tb.keyframe(0.0, AnimValue::Vec3(vec3(0.0, 0.0, 10.0)))
                .keyframe(5.0, AnimValue::Vec3(vec3(5.0, 0.0, 10.0)))
        });
        let (_scene, timeline, _camera) = sb.build();
        assert!((timeline.duration() - 5.0).abs() < f32::EPSILON);
    }

    #[test]
    #[should_panic(expected = "property \"pos\" not found on Camera")]
    fn animate_camera_invalid_property_panics() {
        let mut sb = SceneBuilder::new();
        sb.animate_camera("pos", |tb| tb.keyframe(0.0, AnimValue::Vec3(Vec3::ZERO)));
    }

    #[test]
    #[should_panic(expected = "keyframe value type mismatch")]
    fn animate_camera_wrong_type_panics() {
        let mut sb = SceneBuilder::new();
        sb.animate_camera("fov", |tb| {
            // fov is Float, passing Vec3
            tb.keyframe(0.0, AnimValue::Vec3(Vec3::ZERO))
        });
    }

    #[test]
    fn multiple_objects_and_tracks() {
        let mut sb = SceneBuilder::new();
        let circle = sb.add(Disk::new(Vec3::ZERO, 1.0, WHITE));
        let rect = sb.add(Rectangle::new(Vec3::ZERO, vec2(2.0, 1.0), RED));

        sb.animate(&circle, "radius", |tb| {
            tb.keyframe(0.0, AnimValue::Float(1.0))
                .keyframe(3.0, AnimValue::Float(5.0))
        });
        sb.animate(&circle, "color", |tb| {
            tb.keyframe(0.0, AnimValue::Vec4(vec4(1.0, 1.0, 1.0, 1.0)))
                .keyframe(2.0, AnimValue::Vec4(vec4(1.0, 0.0, 0.0, 1.0)))
        });
        sb.animate(&rect, "size", |tb| {
            tb.keyframe(0.0, AnimValue::Vec2(vec2(2.0, 1.0)))
                .keyframe(4.0, AnimValue::Vec2(vec2(10.0, 5.0)))
        });

        let (scene, timeline, _camera) = sb.build();
        assert_eq!(scene.len(), 2);
        assert!((timeline.duration() - 4.0).abs() < f32::EPSILON);
    }

    #[test]
    fn keyframe_with_easing() {
        let mut sb = SceneBuilder::new();
        let circle = sb.add(Disk::new(Vec3::ZERO, 1.0, WHITE));
        sb.animate(&circle, "radius", |tb| {
            // Easing is on k0 (the segment start keyframe)
            tb.keyframe_with_easing(0.0, AnimValue::Float(1.0), quad_out)
                .keyframe(2.0, AnimValue::Float(5.0))
        });
        let (_scene, timeline, _camera) = sb.build();
        // Evaluate at midpoint — quad_out easing on k0
        let val = timeline.tracks[0].evaluate(1.0);
        let AnimValue::Float(f) = val.unwrap() else {
            panic!("expected Float");
        };
        // quad_out(0.5) = 0.75, so value = 1.0 + (5.0-1.0)*0.75 = 4.0
        assert!((f - 4.0).abs() < 1e-5);
    }

    #[test]
    fn build_applies_correctly() {
        let mut sb = SceneBuilder::new();
        let circle = sb.add(Disk::new(Vec3::ZERO, 1.0, WHITE));
        sb.camera(Camera::new(vec3(0.0, 0.0, 10.0), Vec3::ZERO));
        sb.animate(&circle, "radius", |tb| {
            tb.keyframe(0.0, AnimValue::Float(1.0))
                .keyframe(2.0, AnimValue::Float(5.0))
        });
        sb.animate_camera("fov", |tb| {
            tb.keyframe(0.0, AnimValue::Float(60.0))
                .keyframe(2.0, AnimValue::Float(90.0))
        });

        let (mut scene, timeline, mut camera) = sb.build();
        timeline.apply(1.0, &mut scene, &mut camera);

        // Disk radius should be 3.0 (midpoint of 1.0 and 5.0)
        let obj = scene.get(circle.id).unwrap();
        let AnimValue::Float(radius) = obj.get("radius").unwrap() else {
            panic!("expected Float");
        };
        assert!((radius - 3.0).abs() < 1e-5);

        // Camera fov should be 75.0 (midpoint of 60.0 and 90.0)
        assert!((camera.fov - 75.0).abs() < 1e-5);
    }

    #[test]
    fn all_object_types_work() {
        let mut sb = SceneBuilder::new();
        let _circle = sb.add(Disk::new(Vec3::ZERO, 1.0, WHITE));
        let _line = sb.add(Line::new(Vec3::ZERO, Vec3::X, 1.0, WHITE));
        let _rect = sb.add(Rectangle::new(Vec3::ZERO, vec2(1.0, 1.0), WHITE));
        let _poly = sb.add(Polygon::new(Vec3::ZERO, 1.0, 6, WHITE));
        let _arc = sb.add(Arc::new(
            Vec3::ZERO,
            0.5,
            1.0,
            0.0,
            std::f32::consts::PI,
            WHITE,
        ));
        let _arrow = sb.add(Arrow::new(Vec3::ZERO, Vec3::X, WHITE));
        let _spiral = sb.add(Spiral::new(Vec3::ZERO, 0.001, 0.1, WHITE, 100, 0.01));
        let _text = sb.add(Text::new("hello", vec2(10.0, 20.0), 16.0, WHITE));
        let _torus = sb.add(Torus::new(Vec3::ZERO, 2.0, 0.5, WHITE));
        let _tube = sb.add(Tube::new(vec![Vec3::ZERO, Vec3::X], 0.5, WHITE));
        let _vector_text = sb.add(VectorText::new(
            "test",
            crate::scene::font::default_font(),
            1.0,
            WHITE,
        ));

        let (scene, _timeline, _camera) = sb.build();
        assert_eq!(scene.len(), 11);
    }

    #[test]
    fn animate_arc_specific_properties() {
        let mut sb = SceneBuilder::new();
        let arc = sb.add(Arc::new(Vec3::ZERO, 0.0, 1.0, 0.0, 0.0, WHITE));
        sb.animate(&arc, "sweep_angle", |tb| {
            tb.keyframe(0.0, AnimValue::Float(0.0))
                .keyframe(2.0, AnimValue::Float(std::f32::consts::TAU))
        });
        sb.animate(&arc, "inner_radius", |tb| {
            tb.keyframe(0.0, AnimValue::Float(0.0))
                .keyframe(2.0, AnimValue::Float(0.8))
        });
        let (_scene, timeline, _camera) = sb.build();
        assert_eq!(timeline.tracks.len(), 2);
    }

    #[test]
    fn animate_arrow_specific_properties() {
        let mut sb = SceneBuilder::new();
        let arrow = sb.add(Arrow::new(Vec3::ZERO, vec3(5.0, 0.0, 0.0), WHITE));
        sb.animate(&arrow, "end", |tb| {
            tb.keyframe(0.0, AnimValue::Vec3(vec3(5.0, 0.0, 0.0)))
                .keyframe(3.0, AnimValue::Vec3(vec3(0.0, 5.0, 0.0)))
        });
        let (_scene, timeline, _camera) = sb.build();
        assert!((timeline.duration() - 3.0).abs() < f32::EPSILON);
    }

    #[test]
    fn camera_defaults_preserved_when_not_set() {
        let sb = SceneBuilder::new();
        let (_scene, _timeline, camera) = sb.build();
        assert_eq!(camera.projection, ProjectionMode::Perspective);
    }

    #[test]
    fn animate_torus_rotation() {
        let mut sb = SceneBuilder::new();
        let torus = sb.add(Torus::new(Vec3::ZERO, 2.0, 0.5, WHITE));
        sb.animate(&torus, "rotation", |tb| {
            tb.keyframe(0.0, AnimValue::Mat4(Mat4::IDENTITY)).keyframe(
                2.0,
                AnimValue::Mat4(Mat4::from_rotation_z(std::f32::consts::FRAC_PI_2)),
            )
        });
        let (mut scene, timeline, mut camera) = sb.build();
        timeline.apply(1.0, &mut scene, &mut camera);

        let obj = scene.get(torus.id).unwrap();
        let AnimValue::Mat4(rot) = obj.get("rotation").unwrap() else {
            panic!("expected Mat4");
        };
        // At t=1.0 (midpoint), should be ~45 degrees
        let expected = Mat4::from_rotation_z(std::f32::consts::FRAC_PI_4);
        for i in 0..16 {
            assert!(
                (rot.to_cols_array()[i] - expected.to_cols_array()[i]).abs() < 1e-4,
                "rotation mismatch at index {i}"
            );
        }
    }

    #[test]
    #[should_panic(expected = "keyframe value type mismatch")]
    fn animate_torus_rotation_wrong_type_panics() {
        let mut sb = SceneBuilder::new();
        let torus = sb.add(Torus::new(Vec3::ZERO, 2.0, 0.5, WHITE));
        sb.animate(&torus, "rotation", |tb| {
            tb.keyframe(0.0, AnimValue::Float(0.0))
        });
    }

    #[test]
    fn animate_tube_specific_properties() {
        let mut sb = SceneBuilder::new();
        let tube = sb.add(Tube::new(
            vec![Vec3::ZERO, Vec3::X, vec3(2.0, 1.0, 0.0)],
            0.5,
            WHITE,
        ));
        sb.animate(&tube, "radius", |tb| {
            tb.keyframe(0.0, AnimValue::Float(0.5))
                .keyframe(2.0, AnimValue::Float(2.0))
        });
        sb.animate(&tube, "closed", |tb| {
            tb.keyframe(0.0, AnimValue::Bool(false))
                .keyframe(1.0, AnimValue::Bool(true))
        });
        let (_scene, timeline, _camera) = sb.build();
        assert_eq!(timeline.tracks.len(), 2);
    }

    #[test]
    fn obj_ref_is_copyable() {
        let mut sb = SceneBuilder::new();
        let circle = sb.add(Disk::new(Vec3::ZERO, 1.0, WHITE));
        let circle2 = circle; // Copy
        sb.animate(&circle, "radius", |tb| {
            tb.keyframe(0.0, AnimValue::Float(1.0))
        });
        sb.animate(&circle2, "color", |tb| {
            tb.keyframe(0.0, AnimValue::Vec4(vec4(1.0, 0.0, 0.0, 1.0)))
        });
        let (_scene, timeline, _camera) = sb.build();
        assert_eq!(timeline.tracks.len(), 2);
    }
}
