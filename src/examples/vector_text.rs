use macroquad::prelude::*;

use crate::animation::timeline::Timeline;
use crate::animation::track::{Keyframe, Track};
use crate::camera::Camera;
use crate::scene::Scene;
use crate::scene::font;
use crate::scene::objects::VectorText;
use crate::scene::value::AnimValue;

pub fn build() -> (Scene, Timeline, Camera) {
    let mut scene = Scene::new();
    let mut timeline = Timeline::new();

    let mut text = VectorText::new("Hello", font::default_font(), 1.0, WHITE);
    text.progress = 0.0;
    text.stagger = 0.5; // letters overlap — next starts before previous finishes
    let text_id = scene.add(text);

    let mut progress_track = Track::new(text_id, "progress");
    progress_track.add_keyframe(Keyframe::new(0.0, AnimValue::Float(0.0)));
    progress_track.add_keyframe(Keyframe::new(3.0, AnimValue::Float(1.0)));
    timeline.add_track(progress_track);

    let camera = Camera::new(vec3(0.3, 0.3, 3.0), vec3(0.3, 0.3, 0.0));

    (scene, timeline, camera)
}
