//! Single registry of every buildable scene, consumed by all binaries.
//!
//! One entry type covers examples, videos, and the scratch scene, so the
//! viewer, snapshot, export, and picker binaries all resolve scene names
//! against the same list.

use std::fmt;

use crate::animation::timeline::Timeline;
use crate::camera::Camera;
use crate::scene::Scene;
use crate::{examples, my_scene, videos};

/// Scene builders must be called inside the macroquad window (GL context):
/// scenes that load textures require it.
pub type BuildFn = fn() -> (Scene, Timeline, Camera);

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SceneKind {
    /// Small demo of specific engine features.
    Example,
    /// Full production intended for MP4 export.
    Video,
    /// The active scratch scene (`src/my_scene.rs`), default for viewer and snapshot.
    Scratch,
}

#[derive(Clone)]
pub struct SceneEntry {
    pub name: &'static str,
    pub description: &'static str,
    pub kind: SceneKind,
    pub build: BuildFn,
    /// Default audio track offered on export (None = silent).
    pub audio: Option<&'static str>,
}

impl fmt::Display for SceneEntry {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{} — {}", self.name, self.description)
    }
}

pub const SCENES: &[SceneEntry] = &[
    SceneEntry {
        name: "my_scene",
        description: "Scratch scene (src/my_scene.rs)",
        kind: SceneKind::Scratch,
        build: my_scene::build,
        audio: None,
    },
    SceneEntry {
        name: "bouncing_ball",
        description: "Bouncing ball with easing, rectangle pulse, hexagon rotation",
        kind: SceneKind::Example,
        build: examples::bouncing_ball::build,
        audio: None,
    },
    SceneEntry {
        name: "l_system",
        description: "Gradient-colored L-system with write-on progress and animated theta",
        kind: SceneKind::Example,
        build: examples::l_system::build,
        audio: None,
    },
    SceneEntry {
        name: "sequential_api",
        description: "Demonstrates animate_seq, animate_for, parallel, wait, and mixing with absolute-time animate",
        kind: SceneKind::Example,
        build: examples::sequential_api::build,
        audio: None,
    },
    SceneEntry {
        name: "spiral",
        description: "Sunflower spiral with animated delta_theta and camera pan",
        kind: SceneKind::Example,
        build: examples::spiral::build,
        audio: None,
    },
    SceneEntry {
        name: "torus",
        description: "Rotating torus with animated Mat4 orientation",
        kind: SceneKind::Example,
        build: examples::torus::build,
        audio: None,
    },
    SceneEntry {
        name: "tube",
        description: "Helix, trefoil knot, and L-bend tubes with animated radius and color",
        kind: SceneKind::Example,
        build: examples::tube::build,
        audio: None,
    },
    SceneEntry {
        name: "vector_text",
        description: "Write-on animation with bezier-based vector text",
        kind: SceneKind::Example,
        build: examples::vector_text::build,
        audio: None,
    },
    SceneEntry {
        name: "bouncing_ball_long",
        description: "20-second bouncing ball production with animated camera orbit",
        kind: SceneKind::Video,
        build: videos::bouncing_ball::build,
        audio: None,
    },
    SceneEntry {
        name: "torus_knot",
        description: "Rainbow torus knot with dolly zoom and ring sweep animations",
        kind: SceneKind::Video,
        build: videos::torus_knot::build,
        audio: None,
    },
    SceneEntry {
        name: "basic_l_system",
        description: "A basic l system that animates theta and progress changing",
        kind: SceneKind::Video,
        build: videos::basic_l_system::build,
        audio: None,
    },
    SceneEntry {
        name: "turtle_intro",
        description: "An intro to drawing with turtle graphics and the L-System rules",
        kind: SceneKind::Video,
        build: videos::turtle_intro::build,
        audio: None,
    },
];

pub fn find(name: &str) -> Option<&'static SceneEntry> {
    SCENES.iter().find(|e| e.name == name)
}

pub fn names() -> Vec<&'static str> {
    SCENES.iter().map(|e| e.name).collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn find_known_scene() {
        assert!(find("my_scene").is_some());
        assert!(find("torus_knot").is_some());
    }

    #[test]
    fn find_unknown_scene_returns_none() {
        assert!(find("nope").is_none());
    }

    #[test]
    fn names_are_unique() {
        let mut names = names();
        let total = names.len();
        names.sort_unstable();
        names.dedup();
        assert_eq!(names.len(), total, "duplicate scene names in registry");
    }
}
