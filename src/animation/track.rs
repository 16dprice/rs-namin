use crate::scene::value::AnimValue;
use crate::scene::ObjectId;

use super::easing::{linear, EasingFn};

pub struct Keyframe {
    pub time: f32,
    pub value: AnimValue,
    pub easing: EasingFn,
}

impl Keyframe {
    pub fn new(time: f32, value: AnimValue) -> Self {
        Self {
            time,
            value,
            easing: linear,
        }
    }

    pub fn with_easing(time: f32, value: AnimValue, easing: EasingFn) -> Self {
        Self {
            time,
            value,
            easing,
        }
    }
}

pub struct Track {
    pub object_id: ObjectId,
    pub property_name: String,
    keyframes: Vec<Keyframe>,
}

impl Track {
    pub fn new(object_id: ObjectId, property_name: impl Into<String>) -> Self {
        Self {
            object_id,
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
        let i = self
            .keyframes
            .iter()
            .rposition(|k| k.time <= time)
            .unwrap();

        let k0 = &self.keyframes[i];
        let k1 = &self.keyframes[i + 1];

        let segment_duration = k1.time - k0.time;
        let t = (time - k0.time) / segment_duration;
        let eased_t = (k0.easing)(t);

        Some(AnimValue::lerp(&k0.value, &k1.value, eased_t))
    }
}
