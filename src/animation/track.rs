use crate::scene::ObjectId;
use crate::scene::value::AnimValue;

use super::easing::Easing;

pub struct Keyframe {
    pub time: f32,
    pub value: AnimValue,
    /// How motion eases *into* this keyframe: the segment from the previous
    /// keyframe to this one uses this curve. The first keyframe's easing is
    /// unused (there is no incoming segment).
    pub easing: Easing,
    /// Arrive in this many equal sub-steps, each shaped by `easing` — e.g. a
    /// progress keyframe with `steps = segment count` reveals an L-system
    /// one eased segment at a time without per-segment keyframes. 0/1 =
    /// plain single-segment interpolation.
    pub steps: u32,
}

impl Keyframe {
    pub fn new(time: f32, value: AnimValue) -> Self {
        Self {
            time,
            value,
            easing: Easing::Linear,
            steps: 1,
        }
    }

    pub fn with_easing(time: f32, value: AnimValue, easing: Easing) -> Self {
        Self {
            time,
            value,
            easing,
            steps: 1,
        }
    }

    pub fn with_steps(mut self, steps: u32) -> Self {
        self.steps = steps;
        self
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
            .binary_search_by(|k| k.time.total_cmp(&keyframe.time))
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

        // Find the segment: keyframes[i] and keyframes[i+1] where time is between them.
        // A NaN time fails every comparison (including the clamps above), so
        // fall back to the first keyframe instead of panicking mid-frame.
        let Some(i) = self.keyframes.iter().rposition(|k| k.time <= time) else {
            return Some(first.value.clone());
        };

        let k0 = &self.keyframes[i];
        let k1 = &self.keyframes[i + 1];

        let segment_duration = k1.time - k0.time;
        let t = (time - k0.time) / segment_duration;
        // Easing belongs to the arrival keyframe: k1's curve shapes k0 -> k1.
        // With `steps > 1` the segment is a staircase of N equal sub-steps,
        // each eased by k1's curve.
        let eased_t = if k1.steps > 1 {
            let n = k1.steps as f32;
            let x = (t * n).clamp(0.0, n);
            let i = x.floor().min(n - 1.0);
            (i + k1.easing.eval(x - i)) / n
        } else {
            k1.easing.eval(t)
        };

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
        eased_track.add_keyframe(Keyframe::new(0.0, AnimValue::Float(0.0)));
        eased_track.add_keyframe(Keyframe::with_easing(2.0, AnimValue::Float(100.0), Easing::QuadIn));

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
    fn stepped_keyframe_staircases_the_segment() {
        let mut track = Track::new(dummy_id(), "progress");
        track.add_keyframe(Keyframe::new(0.0, AnimValue::Float(0.0)));
        track.add_keyframe(Keyframe::with_easing(4.0, AnimValue::Float(1.0), Easing::Linear).with_steps(4));

        // With linear easing per step, stepping is invisible (each sub-step
        // linearly covers its share): value == t/4 throughout.
        for t in [0.5, 1.0, 2.0, 3.7] {
            let AnimValue::Float(v) = track.evaluate(t).unwrap() else {
                panic!("expected Float")
            };
            assert!((v - t / 4.0).abs() < 1e-5, "t={t}: {v}");
        }
    }

    #[test]
    fn stepped_keyframe_eases_within_each_substep() {
        let mut track = Track::new(dummy_id(), "progress");
        track.add_keyframe(Keyframe::new(0.0, AnimValue::Float(0.0)));
        track.add_keyframe(Keyframe::with_easing(4.0, AnimValue::Float(1.0), Easing::QuadIn).with_steps(4));

        // Sub-step boundaries land exactly on the uniform ramp…
        for (t, expected) in [(1.0, 0.25), (2.0, 0.5), (3.0, 0.75), (4.0, 1.0)] {
            let AnimValue::Float(v) = track.evaluate(t).unwrap() else {
                panic!("expected Float")
            };
            assert!((v - expected).abs() < 1e-5, "t={t}: {v}");
        }
        // …and mid-sub-step values follow the easing inside the step:
        // halfway through step 3 (t=2.5), quad_in(0.5)=0.25, so
        // value = (2 + 0.25) / 4.
        let AnimValue::Float(v) = track.evaluate(2.5).unwrap() else {
            panic!("expected Float")
        };
        assert!((v - 0.5625).abs() < 1e-5, "got {v}");
    }

    #[test]
    fn stepped_matches_equivalent_generated_keyframes() {
        // The turtle_intro pattern: N generated keyframes with uniform
        // time/value increments and SineInOut each == one stepped keyframe.
        let n = 17;
        let duration = 5.0;
        let mut generated = Track::new(dummy_id(), "progress");
        generated.add_keyframe(Keyframe::new(0.0, AnimValue::Float(0.0)));
        for i in 0..n {
            generated.add_keyframe(Keyframe::with_easing(
                (i + 1) as f32 / n as f32 * duration,
                AnimValue::Float((i + 1) as f32 / n as f32),
                Easing::SineInOut,
            ));
        }
        let mut stepped = Track::new(dummy_id(), "progress");
        stepped.add_keyframe(Keyframe::new(0.0, AnimValue::Float(0.0)));
        stepped.add_keyframe(Keyframe::with_easing(duration, AnimValue::Float(1.0), Easing::SineInOut).with_steps(n));

        for i in 0..=100 {
            let t = i as f32 / 100.0 * duration;
            let (AnimValue::Float(a), AnimValue::Float(b)) = (generated.evaluate(t).unwrap(), stepped.evaluate(t).unwrap()) else {
                panic!("expected Floats")
            };
            assert!((a - b).abs() < 1e-4, "t={t}: generated {a} vs stepped {b}");
        }
    }

    #[test]
    fn steps_of_zero_or_one_behave_like_plain_easing() {
        for steps in [0, 1] {
            let mut track = Track::new(dummy_id(), "radius");
            track.add_keyframe(Keyframe::new(0.0, AnimValue::Float(0.0)));
            track.add_keyframe(Keyframe::with_easing(2.0, AnimValue::Float(100.0), Easing::QuadOut).with_steps(steps));
            let AnimValue::Float(v) = track.evaluate(1.0).unwrap() else {
                panic!("expected Float")
            };
            assert!((v - 75.0).abs() < 1e-4, "steps={steps}: {v}");
        }
    }

    #[test]
    fn evaluate_nan_time_returns_first_value_instead_of_panicking() {
        // Regression: a zero-duration looping clock used to produce NaN time
        // (x % 0.0), and evaluate() panicked on the segment search.
        let mut track = Track::new(dummy_id(), "radius");
        track.add_keyframe(Keyframe::new(0.0, AnimValue::Float(7.0)));
        assert_eq!(track.evaluate(f32::NAN), Some(AnimValue::Float(7.0)));
        track.add_keyframe(Keyframe::new(2.0, AnimValue::Float(9.0)));
        assert_eq!(track.evaluate(f32::NAN), Some(AnimValue::Float(7.0)));
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
