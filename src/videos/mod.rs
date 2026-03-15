pub mod bouncing_ball;
pub mod torus_knot;

use std::fmt;

use crate::animation::timeline::Timeline;
use crate::camera::Camera;
use crate::scene::Scene;

type BuildFn = fn() -> (Scene, Timeline, Camera);

#[derive(Clone)]
pub struct Video {
    pub name: &'static str,
    pub description: &'static str,
    pub build: BuildFn,
    pub audio: Option<&'static str>,
}

impl fmt::Display for Video {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{} — {}", self.name, self.description)
    }
}

pub const VIDEOS: &[Video] = &[
    Video {
        name: "bouncing_ball",
        description: "Bouncing ball with easing, rectangle pulse, hexagon rotation",
        build: bouncing_ball::build,
        audio: None,
    },
    Video {
        name: "torus_knot",
        description: "Rainbow torus knot with dolly zoom and ring sweep animations",
        build: torus_knot::build,
        audio: None,
    },
];

pub fn find(name: &str) -> Option<&'static Video> {
    VIDEOS.iter().find(|v| v.name == name)
}
