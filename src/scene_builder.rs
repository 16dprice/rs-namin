use std::mem;

use crate::animation::binding::Binding;
use crate::animation::easing::Easing;
use crate::animation::timeline::Timeline;
use crate::animation::track::{Keyframe, Track, TrackTarget};
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
    cursor: f32,
}

impl SceneBuilder {
    pub fn new() -> Self {
        Self {
            scene: Scene::new(),
            timeline: Timeline::new(),
            camera: Camera::default(),
            cursor: 0.0,
        }
    }

    /// Returns the current cursor position (absolute time).
    pub fn cursor(&self) -> f32 {
        self.cursor
    }

    /// Set the cursor to an absolute time.
    pub fn set_cursor(&mut self, time: f32) -> &mut Self {
        self.cursor = time;
        self
    }

    /// Advance the cursor by `duration` seconds without creating any animation.
    pub fn wait(&mut self, duration: f32) -> &mut Self {
        self.cursor += duration;
        self
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
    pub fn animate(&mut self, obj: &ObjRef, property: &str, build: impl FnOnce(TrackBuilder) -> TrackBuilder) -> &mut Self {
        let scene_obj = self
            .scene
            .get(obj.id)
            .unwrap_or_else(|| panic!("SceneBuilder::animate: object {:?} not found in scene", obj.id));

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
    pub fn animate_camera(&mut self, property: &str, build: impl FnOnce(TrackBuilder) -> TrackBuilder) -> &mut Self {
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

    /// Animate a property using **relative** keyframe times offset by the cursor.
    /// After the closure runs, the cursor advances by the max keyframe time.
    ///
    /// This lets you chain animations sequentially:
    /// ```ignore
    /// sb.set_cursor(5.0);
    /// sb.animate_seq(&obj, "progress", |tb| {
    ///     tb.keyframe(0.0, AnimValue::Float(0.0))   // absolute t = 5.0
    ///       .keyframe(3.0, AnimValue::Float(1.0))   // absolute t = 8.0
    /// });
    /// // cursor is now 8.0
    /// ```
    pub fn animate_seq(&mut self, obj: &ObjRef, property: &str, build: impl FnOnce(TrackBuilder) -> TrackBuilder) -> &mut Self {
        let scene_obj = self
            .scene
            .get(obj.id)
            .unwrap_or_else(|| panic!("SceneBuilder::animate_seq: object {:?} not found in scene", obj.id));

        let names = scene_obj.property_names();
        assert!(
            names.contains(&property),
            "SceneBuilder::animate_seq: property \"{}\" not found on object {:?}. Valid properties: {:?}",
            property,
            obj.id,
            names,
        );

        let expected = scene_obj.get(property).unwrap();
        let tb = TrackBuilder::new(&expected);
        let tb = build(tb);
        let keyframes = tb.finish();

        self.build_seq_track(keyframes, || Track::new(obj.id, property));
        self
    }

    /// Animate a camera property using **relative** keyframe times offset by the cursor.
    /// After the closure runs, the cursor advances by the max keyframe time.
    pub fn animate_camera_seq(&mut self, property: &str, build: impl FnOnce(TrackBuilder) -> TrackBuilder) -> &mut Self {
        let names = self.camera.property_names();
        assert!(
            names.contains(&property),
            "SceneBuilder::animate_camera_seq: property \"{}\" not found on Camera. Valid properties: {:?}",
            property,
            names,
        );

        let expected = self.camera.get(property).unwrap();
        let tb = TrackBuilder::new(&expected);
        let tb = build(tb);
        let keyframes = tb.finish();

        self.build_seq_track(keyframes, || Track::camera(property));
        self
    }

    /// Lock a property of `target` to a property of `source`: every frame,
    /// after keyframe tracks apply, the target property is overwritten with
    /// the source property's current value. Bindings replace keyframes for
    /// the target property (a property is tracked or bound, never both) and
    /// may chain across objects; cycles are rejected at `build()`.
    ///
    /// # Panics
    /// Panics if either property doesn't exist, the value types differ, the
    /// source is the target object itself, the property is already bound, or
    /// the property already has a keyframe track.
    pub fn bind(&mut self, target: &ObjRef, property: &str, source: &ObjRef, source_property: &str) -> &mut Self {
        self.bind_internal(target, property, source, source_property, None, None, None)
    }

    /// Like [`bind`](Self::bind), but only active inside the `[start, end)`
    /// time window (either side `None` = open). Outside the window the
    /// property falls back to its keyframe track, if any — windowed bindings
    /// may coexist with tracks, and several windowed bindings may share a
    /// property as long as their windows don't overlap.
    pub fn bind_during(
        &mut self,
        target: &ObjRef,
        property: &str,
        source: &ObjRef,
        source_property: &str,
        start: Option<f32>,
        end: Option<f32>,
    ) -> &mut Self {
        self.bind_internal(target, property, source, source_property, None, start, end)
    }

    /// Hide `obj` until `time` seconds — it skips drawing before then but
    /// stays fully animatable, so tracks and bindings can pre-position it
    /// for its entrance.
    ///
    /// # Panics
    /// Panics if `time` is negative or non-finite.
    pub fn appear_at(&mut self, obj: &ObjRef, time: f32) -> &mut Self {
        assert!(
            time.is_finite() && time >= 0.0,
            "SceneBuilder::appear_at: time must be a non-negative number, got {time}",
        );
        self.timeline.add_appearance(obj.id, time);
        self
    }

    /// Like [`bind`](Self::bind), but adds `offset` to the source value each
    /// frame (component-wise; Float/Vec2/Vec3/Vec4 properties only).
    ///
    /// # Panics
    /// Panics on the same conditions as `bind`, or if `offset`'s variant
    /// doesn't match the property type.
    pub fn bind_with_offset(
        &mut self,
        target: &ObjRef,
        property: &str,
        source: &ObjRef,
        source_property: &str,
        offset: AnimValue,
    ) -> &mut Self {
        self.bind_internal(target, property, source, source_property, Some(offset), None, None)
    }

    #[allow(clippy::too_many_arguments)]
    fn bind_internal(
        &mut self,
        target: &ObjRef,
        property: &str,
        source: &ObjRef,
        source_property: &str,
        offset: Option<AnimValue>,
        start: Option<f32>,
        end: Option<f32>,
    ) -> &mut Self {
        assert!(
            target.id != source.id,
            "SceneBuilder::bind: object {:?} cannot source its own target object",
            target.id,
        );

        let target_obj = self
            .scene
            .get(target.id)
            .unwrap_or_else(|| panic!("SceneBuilder::bind: target object {:?} not found in scene", target.id));
        let names = target_obj.property_names();
        assert!(
            names.contains(&property),
            "SceneBuilder::bind: property \"{}\" not found on target object {:?}. Valid properties: {:?}",
            property,
            target.id,
            names,
        );
        let expected = target_obj.get(property).unwrap();

        let source_obj = self
            .scene
            .get(source.id)
            .unwrap_or_else(|| panic!("SceneBuilder::bind: source object {:?} not found in scene", source.id));
        let source_value = source_obj.get(source_property).unwrap_or_else(|| {
            panic!(
                "SceneBuilder::bind: property \"{}\" not readable on source object {:?}. Valid properties: {:?}, outputs: {:?}",
                source_property,
                source.id,
                source_obj.property_names(),
                source_obj.output_names(),
            )
        });

        assert_eq!(
            variant_name(&expected),
            variant_name(&source_value),
            "SceneBuilder::bind: type mismatch — target \"{}\" is {}, source \"{}\" is {}",
            property,
            variant_name(&expected),
            source_property,
            variant_name(&source_value),
        );

        if let Some(offset) = &offset {
            assert_eq!(
                variant_name(offset),
                variant_name(&expected),
                "SceneBuilder::bind: offset type mismatch — property \"{}\" is {}, offset is {}",
                property,
                variant_name(&expected),
                variant_name(offset),
            );
            assert!(
                expected.supports_offset(),
                "SceneBuilder::bind: {} properties do not support offsets",
                variant_name(&expected),
            );
        }

        if let (Some(s), Some(e)) = (start, end) {
            assert!(
                s < e,
                "SceneBuilder::bind: window start {s} must be before end {e} on property \"{property}\"",
            );
        }

        let binding = Binding {
            target: TrackTarget::Object(target.id),
            target_property: property.to_string(),
            source: TrackTarget::Object(source.id),
            source_property: source_property.to_string(),
            offset,
            start,
            end,
        };
        assert!(
            !self
                .timeline
                .bindings
                .iter()
                .any(|b| b.target == binding.target && b.target_property == property && b.window_overlaps(&binding)),
            "SceneBuilder::bind: property \"{}\" on object {:?} is already bound during that window",
            property,
            target.id,
        );

        self.timeline.add_binding(binding);
        self
    }

    /// Run multiple animations in parallel from the current cursor position.
    /// The cursor advances by the duration of the longest animation in the group.
    ///
    /// ```ignore
    /// sb.parallel(|p| {
    ///     p.animate(&obj, "progress", |tb| {
    ///         tb.keyframe(0.0, AnimValue::Float(0.0))
    ///           .keyframe(3.0, AnimValue::Float(1.0))
    ///     });
    ///     p.animate(&obj, "scale", |tb| {
    ///         tb.keyframe(0.0, AnimValue::Float(1.0))
    ///           .keyframe(2.0, AnimValue::Float(1.5))
    ///     });
    /// }); // cursor advances by max(3.0, 2.0) = 3.0
    /// ```
    pub fn parallel(&mut self, build: impl FnOnce(&mut ParallelGroup)) -> &mut Self {
        let start = self.cursor;
        let max_end;
        {
            let mut group = ParallelGroup {
                sb: self,
                start,
                max_end: start,
            };
            build(&mut group);
            max_end = group.max_end;
        }
        self.cursor = max_end;
        self
    }

    /// Offset keyframes by cursor, add the track, and advance cursor by duration.
    /// Returns the duration (max keyframe time before offset).
    fn build_seq_track(&mut self, keyframes: Vec<Keyframe>, make_track: impl FnOnce() -> Track) -> f32 {
        let duration = keyframes.iter().map(|kf| kf.time).fold(0.0_f32, f32::max);
        let mut track = make_track();
        for mut kf in keyframes {
            kf.time += self.cursor;
            track.add_keyframe(kf);
        }
        self.timeline.add_track(track);
        self.cursor += duration;
        duration
    }

    /// Consume the builder and return the scene, timeline, and camera.
    ///
    /// # Panics
    /// Panics if a property is both keyframed and bound, or if bindings form
    /// a cycle.
    pub fn build(mut self) -> (Scene, Timeline, Camera) {
        let scene = mem::take(&mut self.scene);
        let mut timeline = mem::take(&mut self.timeline);
        for binding in &timeline.bindings {
            // A windowed binding may coexist with a track (the track drives
            // the property outside the window); an unwindowed one shadows
            // the track at every time, which can only be a mistake.
            assert!(
                binding.is_windowed()
                    || !timeline
                        .tracks
                        .iter()
                        .any(|t| t.target == binding.target && t.property_name == binding.target_property),
                "SceneBuilder::build: property \"{}\" on {:?} is both keyframed and always-bound — window the binding or remove the track",
                binding.target_property,
                binding.target,
            );
        }
        if let Err(cycle) = timeline.sort_bindings() {
            panic!("SceneBuilder::build: binding cycle between {cycle:?}");
        }
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

/// A group of animations that all start at the same cursor position.
/// Created by [`SceneBuilder::parallel`]. The cursor advances by the longest animation.
pub struct ParallelGroup<'a> {
    sb: &'a mut SceneBuilder,
    start: f32,
    max_end: f32,
}

impl<'a> ParallelGroup<'a> {
    /// Animate a scene object property in parallel. Uses relative keyframe times
    /// offset by the group's start time.
    pub fn animate(&mut self, obj: &ObjRef, property: &str, build: impl FnOnce(TrackBuilder) -> TrackBuilder) -> &mut Self {
        self.sb.cursor = self.start;
        self.sb.animate_seq(obj, property, build);
        self.max_end = self.max_end.max(self.sb.cursor);
        self
    }

    /// Animate a camera property in parallel. Uses relative keyframe times
    /// offset by the group's start time.
    pub fn animate_camera(&mut self, property: &str, build: impl FnOnce(TrackBuilder) -> TrackBuilder) -> &mut Self {
        self.sb.cursor = self.start;
        self.sb.animate_camera_seq(property, build);
        self.max_end = self.max_end.max(self.sb.cursor);
        self
    }
}

/// Builder for constructing keyframes within a track, with type validation.
pub struct TrackBuilder {
    keyframes: Vec<Keyframe>,
    expected_variant: &'static str,
    local_time: f32,
}

impl TrackBuilder {
    fn new(expected: &AnimValue) -> Self {
        Self {
            keyframes: Vec::new(),
            expected_variant: variant_name(expected),
            local_time: 0.0,
        }
    }

    /// Add a keyframe with linear easing.
    pub fn keyframe(mut self, time: f32, value: AnimValue) -> Self {
        self.validate_type(&value);
        self.keyframes.push(Keyframe::new(time, value));
        self.local_time = time;
        self
    }

    /// Add a keyframe with an easing that shapes the motion *into* it (the
    /// segment from the previous keyframe to this one).
    pub fn keyframe_with_easing(mut self, time: f32, value: AnimValue, easing: Easing) -> Self {
        self.validate_type(&value);
        self.keyframes.push(Keyframe::with_easing(time, value, easing));
        self.local_time = time;
        self
    }

    /// Animate to `value` over `duration` seconds from the current local time,
    /// using `easing` on the transition. Advances the local time by `duration`.
    ///
    /// Requires at least one prior keyframe (via `keyframe()` or a previous
    /// `animate_for()`) to establish a starting value.
    ///
    /// ```ignore
    /// tb.keyframe(0.0, AnimValue::Float(1.0))
    ///   .animate_for(1.0, AnimValue::Float(5.0), Easing::SineOut)
    ///   .animate_for(1.0, AnimValue::Float(1.0), Easing::SineIn)
    /// ```
    pub fn animate_for(mut self, duration: f32, value: AnimValue, easing: Easing) -> Self {
        self.validate_type(&value);
        assert!(
            !self.keyframes.is_empty(),
            "TrackBuilder::animate_for: requires a starting keyframe"
        );
        self.local_time += duration;
        self.keyframes.push(Keyframe::with_easing(self.local_time, value, easing));
        self
    }

    /// Subdivide the arrival into the most recent keyframe into `steps`
    /// equal sub-steps, each shaped by that keyframe's easing — e.g. reveal
    /// an L-system one eased segment at a time:
    /// ```ignore
    /// tb.keyframe(0.0, AnimValue::Float(0.0))
    ///   .animate_for(30.0, AnimValue::Float(1.0), Easing::SineInOut)
    ///   .in_steps(segment_count)
    /// ```
    pub fn in_steps(mut self, steps: u32) -> Self {
        let keyframe = self
            .keyframes
            .last_mut()
            .expect("TrackBuilder::in_steps: requires a keyframe to apply steps to");
        keyframe.steps = steps;
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
    use crate::animation::easing::Easing;
    use crate::camera::{Camera, ProjectionMode};
    use crate::scene::l_system as lsys;
    use crate::scene::objects::{Arc, Arrow, Disk, LSystem, Line, Plot, Polygon, Rectangle, Spiral, Text, Torus, Tube, VectorText};

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
            tb.keyframe(0.0, AnimValue::Float(1.0)).keyframe(2.0, AnimValue::Float(5.0))
        });
        let (_scene, timeline, _camera) = sb.build();
        assert!((timeline.duration() - 2.0).abs() < f32::EPSILON);
    }

    #[test]
    #[should_panic(expected = "property \"raidus\" not found")]
    fn animate_invalid_property_name_panics() {
        let mut sb = SceneBuilder::new();
        let circle = sb.add(Disk::new(Vec3::ZERO, 1.0, WHITE));
        sb.animate(&circle, "raidus", |tb| tb.keyframe(0.0, AnimValue::Float(1.0)));
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
            tb.keyframe(0.0, AnimValue::Float(1.0)).keyframe(3.0, AnimValue::Float(5.0))
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
            // Easing is on the arrival keyframe: it shapes the segment into it.
            tb.keyframe(0.0, AnimValue::Float(1.0))
                .keyframe_with_easing(2.0, AnimValue::Float(5.0), Easing::QuadOut)
        });
        let (_scene, timeline, _camera) = sb.build();
        // Evaluate at midpoint — quad_out easing into k1
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
            tb.keyframe(0.0, AnimValue::Float(1.0)).keyframe(2.0, AnimValue::Float(5.0))
        });
        sb.animate_camera("fov", |tb| {
            tb.keyframe(0.0, AnimValue::Float(60.0)).keyframe(2.0, AnimValue::Float(90.0))
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
        let _line = sb.add(Line::new(Vec3::ZERO, Vec3::X, WHITE));
        let _rect = sb.add(Rectangle::new(Vec3::ZERO, vec2(1.0, 1.0), WHITE));
        let _poly = sb.add(Polygon::new(Vec3::ZERO, 1.0, 6, WHITE));
        let _arc = sb.add(Arc::new(Vec3::ZERO, 0.5, 1.0, 0.0, std::f32::consts::PI, WHITE));
        let _arrow = sb.add(Arrow::new(Vec3::ZERO, Vec3::X, WHITE));
        let _spiral = sb.add(Spiral::new(Vec3::ZERO, 0.001, 0.1, WHITE, 100, 0.01));
        let _text = sb.add(Text::new("hello", vec2(10.0, 20.0), 16.0, WHITE));
        let _torus = sb.add(Torus::new(Vec3::ZERO, 2.0, 0.5, WHITE));
        let _tube = sb.add(Tube::new(vec![Vec3::ZERO, Vec3::X], 0.5, WHITE));
        let _vector_text = sb.add(VectorText::new("test", crate::scene::font::default_font(), 1.0, WHITE));
        let (dragon_config, dragon_theta) = lsys::dragon_curve();
        let _lsystem = sb.add(LSystem::new(dragon_config, dragon_theta, WHITE));
        let _plot = sb.add(Plot::new(
            "sin(x)",
            Vec3::ZERO,
            vec2(4.0, 2.0),
            vec2(-3.0, 3.0),
            vec2(-1.0, 1.0),
            WHITE,
        ));

        let (scene, _timeline, _camera) = sb.build();
        assert_eq!(scene.len(), 13);
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
            tb.keyframe(0.0, AnimValue::Float(0.0)).keyframe(2.0, AnimValue::Float(0.8))
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
    fn animate_torus_orientation() {
        let mut sb = SceneBuilder::new();
        let torus = sb.add(Torus::new(Vec3::ZERO, 2.0, 0.5, WHITE));
        sb.animate(&torus, "orientation", |tb| {
            tb.keyframe(0.0, AnimValue::Mat4(Mat4::IDENTITY))
                .keyframe(2.0, AnimValue::Mat4(Mat4::from_rotation_z(std::f32::consts::FRAC_PI_2)))
        });
        let (mut scene, timeline, mut camera) = sb.build();
        timeline.apply(1.0, &mut scene, &mut camera);

        let obj = scene.get(torus.id).unwrap();
        let AnimValue::Mat4(rot) = obj.get("orientation").unwrap() else {
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
    fn animate_torus_orientation_wrong_type_panics() {
        let mut sb = SceneBuilder::new();
        let torus = sb.add(Torus::new(Vec3::ZERO, 2.0, 0.5, WHITE));
        sb.animate(&torus, "orientation", |tb| tb.keyframe(0.0, AnimValue::Float(0.0)));
    }

    #[test]
    fn animate_tube_specific_properties() {
        let mut sb = SceneBuilder::new();
        let tube = sb.add(Tube::new(vec![Vec3::ZERO, Vec3::X, vec3(2.0, 1.0, 0.0)], 0.5, WHITE));
        sb.animate(&tube, "radius", |tb| {
            tb.keyframe(0.0, AnimValue::Float(0.5)).keyframe(2.0, AnimValue::Float(2.0))
        });
        sb.animate(&tube, "closed", |tb| {
            tb.keyframe(0.0, AnimValue::Bool(false)).keyframe(1.0, AnimValue::Bool(true))
        });
        let (_scene, timeline, _camera) = sb.build();
        assert_eq!(timeline.tracks.len(), 2);
    }

    #[test]
    fn bind_locks_property_to_source() {
        let mut sb = SceneBuilder::new();
        let leader = sb.add(Disk::new(Vec3::ZERO, 1.0, WHITE));
        let follower = sb.add(Disk::new(Vec3::ZERO, 1.0, WHITE));
        sb.animate(&leader, "position", |tb| {
            tb.keyframe(0.0, AnimValue::Vec3(Vec3::ZERO))
                .keyframe(2.0, AnimValue::Vec3(vec3(8.0, 0.0, 0.0)))
        });
        sb.bind(&follower, "position", &leader, "position");

        let (mut scene, timeline, mut camera) = sb.build();
        timeline.apply(1.0, &mut scene, &mut camera);
        let AnimValue::Vec3(p) = scene.get(follower.id).unwrap().get("position").unwrap() else {
            panic!("expected Vec3");
        };
        assert!((p - vec3(4.0, 0.0, 0.0)).length() < 1e-5);
    }

    #[test]
    fn bind_with_offset_adds_offset() {
        let mut sb = SceneBuilder::new();
        let leader = sb.add(Disk::new(Vec3::ZERO, 1.0, WHITE));
        let follower = sb.add(Disk::new(Vec3::ZERO, 1.0, WHITE));
        sb.bind_with_offset(&follower, "position", &leader, "position", AnimValue::Vec3(vec3(0.0, 1.5, 0.0)));

        let (mut scene, timeline, mut camera) = sb.build();
        timeline.apply(0.0, &mut scene, &mut camera);
        let AnimValue::Vec3(p) = scene.get(follower.id).unwrap().get("position").unwrap() else {
            panic!("expected Vec3");
        };
        assert!((p - vec3(0.0, 1.5, 0.0)).length() < 1e-5);
    }

    #[test]
    fn bind_cross_property_same_type_works() {
        let mut sb = SceneBuilder::new();
        let disk = sb.add(Disk::new(Vec3::ZERO, 3.0, WHITE));
        let ring = sb.add(crate::scene::objects::Ring::new(Vec3::ZERO, 1.0, WHITE, 1.0));
        // Different property names, same Float type.
        sb.bind(&ring, "progress", &disk, "radius");
        let (mut scene, timeline, mut camera) = sb.build();
        timeline.apply(0.0, &mut scene, &mut camera);
        let AnimValue::Float(p) = scene.get(ring.id).unwrap().get("progress").unwrap() else {
            panic!("expected Float");
        };
        assert!((p - 3.0).abs() < 1e-5);
    }

    #[test]
    fn bind_can_source_a_derived_output_property() {
        let mut sb = SceneBuilder::new();
        let (config, theta) = lsys::dragon_curve();
        let lsystem = sb.add(LSystem::new(config, theta, WHITE));
        let marker = sb.add(Disk::new(Vec3::ZERO, 0.2, WHITE));
        sb.animate(&lsystem, "progress", |tb| {
            tb.keyframe(0.0, AnimValue::Float(0.0)).keyframe(1.0, AnimValue::Float(1.0))
        });
        sb.bind(&marker, "position", &lsystem, "pen_position");

        let (mut scene, timeline, mut camera) = sb.build();
        timeline.apply(0.75, &mut scene, &mut camera);
        let pen = scene.get(lsystem.id).unwrap().get("pen_position").unwrap();
        let marker_pos = scene.get(marker.id).unwrap().get("position").unwrap();
        assert_eq!(marker_pos, pen);
        // The pen has actually moved away from the origin by then.
        let AnimValue::Vec3(p) = marker_pos else { panic!("expected Vec3") };
        assert!(p.length() > 1e-3);
    }

    #[test]
    #[should_panic(expected = "not found on target object")]
    fn bind_cannot_target_an_output_property() {
        let mut sb = SceneBuilder::new();
        let (config, theta) = lsys::dragon_curve();
        let lsystem = sb.add(LSystem::new(config, theta, WHITE));
        let marker = sb.add(Disk::new(Vec3::ZERO, 0.2, WHITE));
        sb.bind(&lsystem, "pen_position", &marker, "position");
    }

    #[test]
    #[should_panic(expected = "not found on target object")]
    fn bind_invalid_target_property_panics() {
        let mut sb = SceneBuilder::new();
        let a = sb.add(Disk::new(Vec3::ZERO, 1.0, WHITE));
        let b = sb.add(Disk::new(Vec3::ZERO, 1.0, WHITE));
        sb.bind(&a, "raidus", &b, "radius");
    }

    #[test]
    #[should_panic(expected = "type mismatch")]
    fn bind_type_mismatch_panics() {
        let mut sb = SceneBuilder::new();
        let a = sb.add(Disk::new(Vec3::ZERO, 1.0, WHITE));
        let b = sb.add(Disk::new(Vec3::ZERO, 1.0, WHITE));
        sb.bind(&a, "position", &b, "radius");
    }

    #[test]
    #[should_panic(expected = "cannot source its own target object")]
    fn bind_self_object_panics() {
        let mut sb = SceneBuilder::new();
        let a = sb.add(Disk::new(Vec3::ZERO, 1.0, WHITE));
        sb.bind(&a, "position", &a, "position");
    }

    #[test]
    #[should_panic(expected = "already bound")]
    fn bind_duplicate_property_panics() {
        let mut sb = SceneBuilder::new();
        let a = sb.add(Disk::new(Vec3::ZERO, 1.0, WHITE));
        let b = sb.add(Disk::new(Vec3::ZERO, 1.0, WHITE));
        sb.bind(&a, "position", &b, "position");
        sb.bind(&a, "position", &b, "position");
    }

    #[test]
    #[should_panic(expected = "offset type mismatch")]
    fn bind_offset_type_mismatch_panics() {
        let mut sb = SceneBuilder::new();
        let a = sb.add(Disk::new(Vec3::ZERO, 1.0, WHITE));
        let b = sb.add(Disk::new(Vec3::ZERO, 1.0, WHITE));
        sb.bind_with_offset(&a, "position", &b, "position", AnimValue::Float(1.0));
    }

    #[test]
    #[should_panic(expected = "keyframed and always-bound")]
    fn bound_and_keyframed_property_panics_at_build() {
        let mut sb = SceneBuilder::new();
        let a = sb.add(Disk::new(Vec3::ZERO, 1.0, WHITE));
        let b = sb.add(Disk::new(Vec3::ZERO, 1.0, WHITE));
        sb.bind(&a, "position", &b, "position");
        sb.animate(&a, "position", |tb| tb.keyframe(0.0, AnimValue::Vec3(Vec3::ZERO)));
        let _ = sb.build();
    }

    #[test]
    fn bind_during_coexists_with_a_track() {
        let mut sb = SceneBuilder::new();
        let a = sb.add(Disk::new(Vec3::ZERO, 1.0, WHITE));
        let b = sb.add(Disk::new(Vec3::ZERO, 3.0, WHITE));
        sb.animate(&a, "radius", |tb| {
            tb.keyframe(0.0, AnimValue::Float(1.0)).keyframe(4.0, AnimValue::Float(9.0))
        });
        sb.bind_during(&a, "radius", &b, "radius", None, Some(2.0));

        let (mut scene, timeline, mut camera) = sb.build();
        timeline.apply(1.0, &mut scene, &mut camera);
        let AnimValue::Float(r) = scene.get(a.id).unwrap().get("radius").unwrap() else {
            panic!("expected Float");
        };
        assert!((r - 3.0).abs() < 1e-5, "bound inside the window, got {r}");

        timeline.apply(3.0, &mut scene, &mut camera);
        let AnimValue::Float(r) = scene.get(a.id).unwrap().get("radius").unwrap() else {
            panic!("expected Float");
        };
        assert!((r - 7.0).abs() < 1e-5, "track resumes after the window, got {r}");
    }

    #[test]
    #[should_panic(expected = "already bound during that window")]
    fn overlapping_windows_panic() {
        let mut sb = SceneBuilder::new();
        let a = sb.add(Disk::new(Vec3::ZERO, 1.0, WHITE));
        let b = sb.add(Disk::new(Vec3::ZERO, 1.0, WHITE));
        sb.bind_during(&a, "radius", &b, "radius", None, Some(3.0));
        sb.bind_during(&a, "radius", &b, "radius", Some(2.0), None);
    }

    #[test]
    #[should_panic(expected = "binding cycle")]
    fn binding_cycle_panics_at_build() {
        let mut sb = SceneBuilder::new();
        let a = sb.add(Disk::new(Vec3::ZERO, 1.0, WHITE));
        let b = sb.add(Disk::new(Vec3::ZERO, 1.0, WHITE));
        sb.bind(&a, "position", &b, "position");
        sb.bind(&b, "radius", &a, "radius");
        let _ = sb.build();
    }

    #[test]
    fn obj_ref_is_copyable() {
        let mut sb = SceneBuilder::new();
        let circle = sb.add(Disk::new(Vec3::ZERO, 1.0, WHITE));
        let circle2 = circle; // Copy
        sb.animate(&circle, "radius", |tb| tb.keyframe(0.0, AnimValue::Float(1.0)));
        sb.animate(&circle2, "color", |tb| tb.keyframe(0.0, AnimValue::Vec4(vec4(1.0, 0.0, 0.0, 1.0))));
        let (_scene, timeline, _camera) = sb.build();
        assert_eq!(timeline.tracks.len(), 2);
    }

    #[test]
    fn cursor_starts_at_zero() {
        let sb = SceneBuilder::new();
        assert!((sb.cursor() - 0.0).abs() < f32::EPSILON);
    }

    #[test]
    fn set_cursor_and_wait() {
        let mut sb = SceneBuilder::new();
        sb.set_cursor(5.0);
        assert!((sb.cursor() - 5.0).abs() < f32::EPSILON);
        sb.wait(3.0);
        assert!((sb.cursor() - 8.0).abs() < f32::EPSILON);
    }

    #[test]
    fn animate_seq_offsets_keyframes_by_cursor() {
        let mut sb = SceneBuilder::new();
        let circle = sb.add(Disk::new(Vec3::ZERO, 1.0, WHITE));
        sb.set_cursor(5.0);
        sb.animate_seq(&circle, "radius", |tb| {
            tb.keyframe(0.0, AnimValue::Float(1.0)).keyframe(3.0, AnimValue::Float(5.0))
        });

        let (mut scene, timeline, mut camera) = sb.build();
        // At t=5.0 (cursor start), radius should be 1.0
        timeline.apply(5.0, &mut scene, &mut camera);
        let AnimValue::Float(r) = scene.get(circle.id).unwrap().get("radius").unwrap() else {
            panic!("expected Float");
        };
        assert!((r - 1.0).abs() < 1e-5);

        // At t=8.0 (cursor + 3.0), radius should be 5.0
        timeline.apply(8.0, &mut scene, &mut camera);
        let AnimValue::Float(r) = scene.get(circle.id).unwrap().get("radius").unwrap() else {
            panic!("expected Float");
        };
        assert!((r - 5.0).abs() < 1e-5);
    }

    #[test]
    fn animate_seq_advances_cursor() {
        let mut sb = SceneBuilder::new();
        let circle = sb.add(Disk::new(Vec3::ZERO, 1.0, WHITE));
        sb.set_cursor(2.0);
        sb.animate_seq(&circle, "radius", |tb| {
            tb.keyframe(0.0, AnimValue::Float(1.0)).keyframe(4.0, AnimValue::Float(5.0))
        });
        assert!((sb.cursor() - 6.0).abs() < f32::EPSILON);
    }

    #[test]
    fn animate_seq_chains_sequentially() {
        let mut sb = SceneBuilder::new();
        let circle = sb.add(Disk::new(Vec3::ZERO, 1.0, WHITE));
        // First animation: radius 1→5 over 2s starting at t=0
        sb.animate_seq(&circle, "radius", |tb| {
            tb.keyframe(0.0, AnimValue::Float(1.0)).keyframe(2.0, AnimValue::Float(5.0))
        });
        // Second animation: color over 3s starting at t=2 (after first)
        sb.animate_seq(&circle, "color", |tb| {
            tb.keyframe(0.0, AnimValue::Vec4(vec4(1.0, 1.0, 1.0, 1.0)))
                .keyframe(3.0, AnimValue::Vec4(vec4(1.0, 0.0, 0.0, 1.0)))
        });
        assert!((sb.cursor() - 5.0).abs() < f32::EPSILON);

        let (mut scene, timeline, mut camera) = sb.build();
        assert!((timeline.duration() - 5.0).abs() < f32::EPSILON);

        // At t=1.0: radius should be midpoint (3.0), color still white (before t=2)
        timeline.apply(1.0, &mut scene, &mut camera);
        let AnimValue::Float(r) = scene.get(circle.id).unwrap().get("radius").unwrap() else {
            panic!("expected Float");
        };
        assert!((r - 3.0).abs() < 1e-5);
    }

    #[test]
    fn animate_camera_seq_works() {
        let mut sb = SceneBuilder::new();
        sb.camera(Camera::new(vec3(0.0, 0.0, 10.0), Vec3::ZERO));
        sb.set_cursor(1.0);
        sb.animate_camera_seq("fov", |tb| {
            tb.keyframe(0.0, AnimValue::Float(60.0)).keyframe(2.0, AnimValue::Float(90.0))
        });
        assert!((sb.cursor() - 3.0).abs() < f32::EPSILON);

        let (mut scene, timeline, mut camera) = sb.build();
        timeline.apply(2.0, &mut scene, &mut camera);
        // At t=2.0 (midpoint of 1.0..3.0), fov should be 75.0
        assert!((camera.fov - 75.0).abs() < 1e-5);
    }

    #[test]
    fn parallel_advances_by_longest() {
        let mut sb = SceneBuilder::new();
        let circle = sb.add(Disk::new(Vec3::ZERO, 1.0, WHITE));
        sb.set_cursor(1.0);
        sb.parallel(|p| {
            p.animate(&circle, "radius", |tb| {
                tb.keyframe(0.0, AnimValue::Float(1.0)).keyframe(3.0, AnimValue::Float(5.0))
            });
            p.animate(&circle, "color", |tb| {
                tb.keyframe(0.0, AnimValue::Vec4(vec4(1.0, 1.0, 1.0, 1.0)))
                    .keyframe(2.0, AnimValue::Vec4(vec4(1.0, 0.0, 0.0, 1.0)))
            });
        });
        // Cursor should advance by max(3.0, 2.0) = 3.0, from 1.0 to 4.0
        assert!((sb.cursor() - 4.0).abs() < f32::EPSILON);

        let (mut scene, timeline, mut camera) = sb.build();
        // Both should start at t=1.0
        timeline.apply(1.0, &mut scene, &mut camera);
        let AnimValue::Float(r) = scene.get(circle.id).unwrap().get("radius").unwrap() else {
            panic!("expected Float");
        };
        assert!((r - 1.0).abs() < 1e-5);

        // At t=2.5 (midpoint of radius track 1.0..4.0)
        timeline.apply(2.5, &mut scene, &mut camera);
        let AnimValue::Float(r) = scene.get(circle.id).unwrap().get("radius").unwrap() else {
            panic!("expected Float");
        };
        assert!((r - 3.0).abs() < 1e-5);
    }

    #[test]
    fn parallel_camera_works() {
        let mut sb = SceneBuilder::new();
        sb.camera(Camera::new(vec3(0.0, 0.0, 10.0), Vec3::ZERO));
        sb.parallel(|p| {
            p.animate_camera("fov", |tb| {
                tb.keyframe(0.0, AnimValue::Float(60.0)).keyframe(2.0, AnimValue::Float(90.0))
            });
            p.animate_camera("position", |tb| {
                tb.keyframe(0.0, AnimValue::Vec3(vec3(0.0, 0.0, 10.0)))
                    .keyframe(3.0, AnimValue::Vec3(vec3(5.0, 0.0, 10.0)))
            });
        });
        assert!((sb.cursor() - 3.0).abs() < f32::EPSILON);
    }

    #[test]
    fn animate_for_builds_keyframes_from_duration() {
        let mut sb = SceneBuilder::new();
        let circle = sb.add(Disk::new(Vec3::ZERO, 1.0, WHITE));
        sb.animate(&circle, "radius", |tb| {
            tb.keyframe(0.0, AnimValue::Float(1.0))
                .animate_for(2.0, AnimValue::Float(5.0), Easing::QuadOut)
        });
        let (mut scene, timeline, mut camera) = sb.build();
        assert!((timeline.duration() - 2.0).abs() < f32::EPSILON);

        // quad_out(0.5) = 0.75, so at t=1.0: 1.0 + (5.0-1.0)*0.75 = 4.0
        timeline.apply(1.0, &mut scene, &mut camera);
        let AnimValue::Float(r) = scene.get(circle.id).unwrap().get("radius").unwrap() else {
            panic!("expected Float");
        };
        assert!((r - 4.0).abs() < 1e-5);
    }

    #[test]
    fn animate_for_chains_multiple_segments() {
        let mut sb = SceneBuilder::new();
        let circle = sb.add(Disk::new(Vec3::ZERO, 1.0, WHITE));
        sb.animate(&circle, "radius", |tb| {
            tb.keyframe(0.0, AnimValue::Float(1.0))
                .animate_for(1.0, AnimValue::Float(5.0), Easing::Linear)
                .animate_for(1.0, AnimValue::Float(1.0), Easing::Linear)
        });
        let (mut scene, timeline, mut camera) = sb.build();
        assert!((timeline.duration() - 2.0).abs() < f32::EPSILON);

        // At t=0.5: midpoint of 1→5 = 3.0
        timeline.apply(0.5, &mut scene, &mut camera);
        let AnimValue::Float(r) = scene.get(circle.id).unwrap().get("radius").unwrap() else {
            panic!("expected Float");
        };
        assert!((r - 3.0).abs() < 1e-5);

        // At t=1.5: midpoint of 5→1 = 3.0
        timeline.apply(1.5, &mut scene, &mut camera);
        let AnimValue::Float(r) = scene.get(circle.id).unwrap().get("radius").unwrap() else {
            panic!("expected Float");
        };
        assert!((r - 3.0).abs() < 1e-5);

        // At t=1.0: peak at 5.0
        timeline.apply(1.0, &mut scene, &mut camera);
        let AnimValue::Float(r) = scene.get(circle.id).unwrap().get("radius").unwrap() else {
            panic!("expected Float");
        };
        assert!((r - 5.0).abs() < 1e-5);
    }

    #[test]
    fn animate_for_works_with_animate_seq() {
        let mut sb = SceneBuilder::new();
        let circle = sb.add(Disk::new(Vec3::ZERO, 1.0, WHITE));
        sb.set_cursor(10.0);
        sb.animate_seq(&circle, "radius", |tb| {
            tb.keyframe(0.0, AnimValue::Float(1.0))
                .animate_for(2.0, AnimValue::Float(5.0), Easing::Linear)
                .animate_for(3.0, AnimValue::Float(1.0), Easing::Linear)
        });
        // cursor should advance by 5.0 (2+3)
        assert!((sb.cursor() - 15.0).abs() < f32::EPSILON);

        let (mut scene, timeline, mut camera) = sb.build();
        // At t=12.0 (cursor 10 + 2.0), should be 5.0
        timeline.apply(12.0, &mut scene, &mut camera);
        let AnimValue::Float(r) = scene.get(circle.id).unwrap().get("radius").unwrap() else {
            panic!("expected Float");
        };
        assert!((r - 5.0).abs() < 1e-5);
    }

    #[test]
    #[should_panic(expected = "requires a starting keyframe")]
    fn animate_for_panics_without_starting_keyframe() {
        let mut sb = SceneBuilder::new();
        let circle = sb.add(Disk::new(Vec3::ZERO, 1.0, WHITE));
        sb.animate(&circle, "radius", |tb| tb.animate_for(1.0, AnimValue::Float(5.0), Easing::Linear));
    }

    #[test]
    fn animate_seq_and_animate_coexist() {
        let mut sb = SceneBuilder::new();
        let circle = sb.add(Disk::new(Vec3::ZERO, 1.0, WHITE));

        // Absolute-time animation (doesn't touch cursor)
        sb.animate(&circle, "color", |tb| {
            tb.keyframe(0.0, AnimValue::Vec4(vec4(1.0, 1.0, 1.0, 1.0)))
                .keyframe(10.0, AnimValue::Vec4(vec4(1.0, 0.0, 0.0, 1.0)))
        });
        assert!((sb.cursor() - 0.0).abs() < f32::EPSILON);

        // Sequential animation (advances cursor)
        sb.animate_seq(&circle, "radius", |tb| {
            tb.keyframe(0.0, AnimValue::Float(1.0)).keyframe(3.0, AnimValue::Float(5.0))
        });
        assert!((sb.cursor() - 3.0).abs() < f32::EPSILON);

        let (_scene, timeline, _camera) = sb.build();
        assert_eq!(timeline.tracks.len(), 2);
        assert!((timeline.duration() - 10.0).abs() < f32::EPSILON);
    }
}
