use crate::scene::ObjectId;
use crate::scene::value::AnimValue;

use super::easing::Easing;

pub struct Keyframe {
    pub time: f32,
    pub value: AnimValue,
    pub easing: Easing,
}

impl Keyframe {
    pub fn new(time: f32, value: AnimValue) -> Self {
        Self {
            time,
            value,
            easing: Easing::Linear,
        }
    }

    pub fn with_easing(time: f32, value: AnimValue, easing: Easing) -> Self {
        Self { time, value, easing }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TrackTarget {
    Object(ObjectId),
    Camera,
}

pub struct Track {
    pub target: TrackTarget,
    pub property_name: String,
    keyframes: Vec<Keyframe>,
}

impl Track {
    pub fn new(object_id: ObjectId, property_name: impl Into<String>) -> Self {
        Self {
            target: TrackTarget::Object(object_id),
            property_name: property_name.into(),
            keyframes: Vec::new(),
        }
    }

    pub fn camera(property_name: impl Into<String>) -> Self {
        Self {
            target: TrackTarget::Camera,
            property_name: property_name.into(),
            keyframes: Vec::new(),
        }
    }

    pub fn add_keyframe(&mut self, keyframe: Keyframe) {
        let pos = self
            .keyframes
            .binary_search_by(|k| k.time.partial_cmp(&keyframe.time).unwrap())
            .unwrap_or_else(|i| i);
        self.keyframes.insert(pos, keyframe);
    }

    pub fn max_time(&self) -> Option<f32> {
        self.keyframes.last().map(|k| k.time)
    }

    pub fn keyframe_times(&self) -> impl Iterator<Item = f32> + '_ {
        self.keyframes.iter().map(|k| k.time)
    }

    pub fn evaluate(&self, time: f32) -> Option<AnimValue> {
        if self.keyframes.is_empty() {
            return None;
        }

        let first = &self.keyframes[0];
        if time <= first.time {
            return Some(first.value.clone());
        }

        let last = &self.keyframes[self.keyframes.len() - 1];
        if time >= last.time {
            return Some(last.value.clone());
        }

        // Find the segment: keyframes[i] and keyframes[i+1] where time is between them
        let i = self.keyframes.iter().rposition(|k| k.time <= time).unwrap();

        let k0 = &self.keyframes[i];
        let k1 = &self.keyframes[i + 1];

        let segment_duration = k1.time - k0.time;
        let t = (time - k0.time) / segment_duration;
        let eased_t = k0.easing.eval(t);

        Some(AnimValue::lerp(&k0.value, &k1.value, eased_t))
    }
}

#[cfg(test)]
mod tests {
    use macroquad::prelude::*;

    use super::*;
    use crate::animation::easing::Easing;
    use crate::scene::ObjectId;

    fn dummy_id() -> ObjectId {
        ObjectId::test_id(0)
    }

    #[test]
    fn empty_track_returns_none() {
        let track = Track::new(dummy_id(), "position");
        assert!(track.evaluate(0.0).is_none());
        assert!(track.evaluate(1.0).is_none());
    }

    #[test]
    fn single_keyframe_returns_value_at_any_time() {
        let mut track = Track::new(dummy_id(), "radius");
        track.add_keyframe(Keyframe::new(1.0, AnimValue::Float(42.0)));
        assert_eq!(track.evaluate(0.0), Some(AnimValue::Float(42.0)));
        assert_eq!(track.evaluate(1.0), Some(AnimValue::Float(42.0)));
        assert_eq!(track.evaluate(5.0), Some(AnimValue::Float(42.0)));
    }

    #[test]
    fn clamps_before_first_keyframe() {
        let mut track = Track::new(dummy_id(), "radius");
        track.add_keyframe(Keyframe::new(1.0, AnimValue::Float(10.0)));
        track.add_keyframe(Keyframe::new(3.0, AnimValue::Float(30.0)));
        assert_eq!(track.evaluate(0.0), Some(AnimValue::Float(10.0)));
    }

    #[test]
    fn clamps_after_last_keyframe() {
        let mut track = Track::new(dummy_id(), "radius");
        track.add_keyframe(Keyframe::new(1.0, AnimValue::Float(10.0)));
        track.add_keyframe(Keyframe::new(3.0, AnimValue::Float(30.0)));
        assert_eq!(track.evaluate(5.0), Some(AnimValue::Float(30.0)));
    }

    #[test]
    fn interpolates_midpoint_with_linear_easing() {
        let mut track = Track::new(dummy_id(), "radius");
        track.add_keyframe(Keyframe::new(0.0, AnimValue::Float(0.0)));
        track.add_keyframe(Keyframe::new(2.0, AnimValue::Float(100.0)));
        assert_eq!(track.evaluate(1.0), Some(AnimValue::Float(50.0)));
    }

    #[test]
    fn easing_produces_different_result_than_linear() {
        let mut linear_track = Track::new(dummy_id(), "radius");
        linear_track.add_keyframe(Keyframe::new(0.0, AnimValue::Float(0.0)));
        linear_track.add_keyframe(Keyframe::new(2.0, AnimValue::Float(100.0)));

        let mut eased_track = Track::new(dummy_id(), "radius");
        eased_track.add_keyframe(Keyframe::with_easing(0.0, AnimValue::Float(0.0), Easing::QuadIn));
        eased_track.add_keyframe(Keyframe::new(2.0, AnimValue::Float(100.0)));

        let linear_val = linear_track.evaluate(1.0);
        let eased_val = eased_track.evaluate(1.0);
        assert_ne!(linear_val, eased_val);
    }

    #[test]
    fn hold_segment_returns_same_value() {
        let mut track = Track::new(dummy_id(), "radius");
        track.add_keyframe(Keyframe::new(0.0, AnimValue::Float(50.0)));
        track.add_keyframe(Keyframe::new(2.0, AnimValue::Float(50.0)));
        assert_eq!(track.evaluate(1.0), Some(AnimValue::Float(50.0)));
    }

    #[test]
    fn vec3_interpolation() {
        let mut track = Track::new(dummy_id(), "position");
        track.add_keyframe(Keyframe::new(0.0, AnimValue::Vec3(vec3(0.0, 0.0, 0.0))));
        track.add_keyframe(Keyframe::new(1.0, AnimValue::Vec3(vec3(10.0, 20.0, 30.0))));
        let result = track.evaluate(0.5);
        assert_eq!(result, Some(AnimValue::Vec3(vec3(5.0, 10.0, 15.0))));
    }

    #[test]
    fn three_keyframes_picks_correct_segment() {
        let mut track = Track::new(dummy_id(), "radius");
        track.add_keyframe(Keyframe::new(0.0, AnimValue::Float(0.0)));
        track.add_keyframe(Keyframe::new(1.0, AnimValue::Float(100.0)));
        track.add_keyframe(Keyframe::new(2.0, AnimValue::Float(0.0)));

        assert_eq!(track.evaluate(0.5), Some(AnimValue::Float(50.0)));
        assert_eq!(track.evaluate(1.5), Some(AnimValue::Float(50.0)));
    }

    #[test]
    fn max_time_empty() {
        let track = Track::new(dummy_id(), "x");
        assert_eq!(track.max_time(), None);
    }

    #[test]
    fn max_time_returns_last_keyframe_time() {
        let mut track = Track::new(dummy_id(), "x");
        track.add_keyframe(Keyframe::new(1.0, AnimValue::Float(0.0)));
        track.add_keyframe(Keyframe::new(5.0, AnimValue::Float(0.0)));
        assert_eq!(track.max_time(), Some(5.0));
    }
}
