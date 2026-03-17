pub mod bouncing_ball;
pub mod l_system;
pub mod spiral;
pub mod torus;
pub mod tube;
pub mod vector_text;

use crate::animation::timeline::Timeline;
use crate::camera::Camera;
use crate::scene::Scene;

type BuildFn = fn() -> (Scene, Timeline, Camera);

pub struct Example {
    pub name: &'static str,
    pub description: &'static str,
    pub build: BuildFn,
}

pub const EXAMPLES: &[Example] = &[
    Example {
        name: "bouncing_ball",
        description: "Bouncing ball with easing, rectangle pulse, hexagon rotation",
        build: bouncing_ball::build,
    },
    Example {
        name: "l_system",
        description: "Dragon curve L-system with animated iterations and write-on",
        build: l_system::build,
    },
    Example {
        name: "spiral",
        description: "Sunflower spiral with animated delta_theta and camera pan",
        build: spiral::build,
    },
    Example {
        name: "torus",
        description: "Rotating torus with animated Mat4 rotation matrix",
        build: torus::build,
    },
    Example {
        name: "tube",
        description: "Helix, trefoil knot, and L-bend tubes with animated radius and color",
        build: tube::build,
    },
    Example {
        name: "vector_text",
        description: "Write-on animation with bezier-based vector text",
        build: vector_text::build,
    },
];

pub fn find(name: &str) -> Option<&'static Example> {
    EXAMPLES.iter().find(|e| e.name == name)
}

pub fn names() -> Vec<&'static str> {
    EXAMPLES.iter().map(|e| e.name).collect()
}
