use macroquad::prelude::*;

use crate::animation::timeline::Timeline;
use crate::animation::track::{Keyframe, Track};
use crate::camera::Camera;
use crate::scene::Scene;
use crate::scene::l_system as lsys;
use crate::scene::objects::LSystem;
use crate::scene::value::AnimValue;

pub fn build() -> (Scene, Timeline, Camera) {
    let mut scene = Scene::new();
    let mut timeline = Timeline::new();

    let (config, theta) = lsys::my3();
    let mut l_system = LSystem::new(config, theta, GREEN).with_colors(vec![RED, YELLOW, GREEN, BLUE, PURPLE]);
    l_system.iterations = 5.0;
    l_system.progress = 0.0;
    l_system.line_width = 0.1;
    l_system.scale = 0.5;
    let l_system_id = scene.add(l_system);

    // Then animate progress 0 → 1 over 3 seconds (held at 0 until t=4)
    let mut progress_track = Track::new(l_system_id, "progress");
    progress_track.add_keyframe(Keyframe::new(0.0, AnimValue::Float(0.0)));
    progress_track.add_keyframe(Keyframe::new(7.0, AnimValue::Float(1.0)));

    let mut theta_track = Track::new(l_system_id, "theta");
    theta_track.add_keyframe(Keyframe::new(8.0, AnimValue::Float(std::f32::consts::FRAC_PI_2)));
    theta_track.add_keyframe(Keyframe::new(28.0, AnimValue::Float(3.0 * std::f32::consts::FRAC_PI_2)));

    timeline.add_track(progress_track);
    timeline.add_track(theta_track);

    let camera = Camera::new(vec3(0.0, 0.0, 15.0), vec3(0.0, 0.0, 0.0));

    (scene, timeline, camera)
}
