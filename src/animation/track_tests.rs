#[cfg(test)]
mod tests {
    use macroquad::prelude::*;

    use crate::animation::easing::quad_in;
    use crate::animation::track::{Keyframe, Track};
    use crate::scene::value::AnimValue;
    use crate::scene::ObjectId;

    fn dummy_id() -> ObjectId {
        // ObjectId(0) — we just need any valid ID for track tests
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
        eased_track.add_keyframe(Keyframe::with_easing(
            0.0,
            AnimValue::Float(0.0),
            quad_in,
        ));
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
