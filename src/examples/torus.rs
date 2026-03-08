use std::f32::consts::FRAC_PI_2;

use macroquad::prelude::*;

use crate::animation::timeline::Timeline;
use crate::animation::track::{Keyframe, Track};
use crate::camera::Camera;
use crate::scene::objects::Torus;
use crate::scene::value::AnimValue;
use crate::scene::Scene;

pub fn build() -> (Scene, Timeline, Camera) {
    let mut scene = Scene::new();
    let mut timeline = Timeline::new();

    let torus = Torus::new(Vec3::ZERO, 2.0, 0.5, BLUE);
    let torus_id = scene.add(torus);

    // Tumble the torus around Y so the rotation is clearly visible.
    // Chain quarter-turn keyframes (slerp takes shortest path, so ≤ 180° each).
    let mut rot_track = Track::new(torus_id, "rotation");
    for i in 0..=4 {
        let angle = FRAC_PI_2 * i as f32;
        rot_track.add_keyframe(Keyframe::new(
            i as f32,
            AnimValue::Mat4(Mat4::from_rotation_y(angle)),
        ));
    }
    timeline.add_track(rot_track);

    let camera = Camera::new(vec3(0.0, 0.0, 8.0), Vec3::ZERO);

    (scene, timeline, camera)
}
