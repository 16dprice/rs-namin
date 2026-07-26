//! Single registry of every buildable scene, consumed by all binaries.
//!
//! One entry type covers built-in scenes (examples, videos, the scratch
//! scene) and scene documents discovered in `scenes/*.ron`, so the viewer,
//! snapshot, export, and library all resolve scene names against one list.

use std::fmt;
use std::sync::OnceLock;

use macroquad::prelude::*;

use crate::animation::timeline::Timeline;
use crate::camera::Camera;
use crate::scene::Scene;
use crate::scene::objects::Text;
use crate::{doc, examples, my_scene, videos};

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
    /// A scene document loaded from `scenes/*.ron`.
    Doc,
}

#[derive(Clone)]
pub enum SceneSource {
    Builtin(BuildFn),
    /// Path to a RON scene document, re-read on every build.
    Doc(&'static str),
}

#[derive(Clone)]
pub struct SceneEntry {
    pub name: &'static str,
    pub description: &'static str,
    pub kind: SceneKind,
    pub source: SceneSource,
    /// Default audio track offered on export (None = silent).
    pub audio: Option<&'static str>,
}

impl SceneEntry {
    /// Build the scene. Builtins are infallible; documents surface load and
    /// validation errors.
    pub fn build_scene(&self) -> Result<(Scene, Timeline, Camera), String> {
        match self.source {
            SceneSource::Builtin(build) => Ok(build()),
            SceneSource::Doc(path) => doc::SceneDoc::load(path)?.build(),
        }
    }

    /// Build the scene, converting any document error into a visible
    /// error-message scene (for the app, which has no failure screen).
    pub fn build_or_error_scene(&self) -> (Scene, Timeline, Camera) {
        match self.build_scene() {
            Ok(built) => built,
            Err(error) => {
                let mut scene = Scene::new();
                scene.add(Text::new(format!("Failed to load scene: {error}"), vec2(20.0, 60.0), 24.0, RED));
                (scene, Timeline::new(), Camera::default())
            }
        }
    }
}

impl fmt::Display for SceneEntry {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{} — {}", self.name, self.description)
    }
}

const BUILTIN_SCENES: &[SceneEntry] = &[
    SceneEntry {
        name: "my_scene",
        description: "Scratch scene (src/my_scene.rs)",
        kind: SceneKind::Scratch,
        source: SceneSource::Builtin(my_scene::build),
        audio: None,
    },
    SceneEntry {
        name: "bouncing_ball",
        description: "Bouncing ball with easing, rectangle pulse, hexagon rotation",
        kind: SceneKind::Example,
        source: SceneSource::Builtin(examples::bouncing_ball::build),
        audio: None,
    },
    SceneEntry {
        name: "l_system",
        description: "Gradient-colored L-system with write-on progress and animated theta",
        kind: SceneKind::Example,
        source: SceneSource::Builtin(examples::l_system::build),
        audio: None,
    },
    SceneEntry {
        name: "sequential_api",
        description: "Demonstrates animate_seq, animate_for, parallel, wait, and mixing with absolute-time animate",
        kind: SceneKind::Example,
        source: SceneSource::Builtin(examples::sequential_api::build),
        audio: None,
    },
    SceneEntry {
        name: "spiral",
        description: "Sunflower spiral with animated delta_theta and camera pan",
        kind: SceneKind::Example,
        source: SceneSource::Builtin(examples::spiral::build),
        audio: None,
    },
    SceneEntry {
        name: "torus",
        description: "Rotating torus with animated Mat4 orientation",
        kind: SceneKind::Example,
        source: SceneSource::Builtin(examples::torus::build),
        audio: None,
    },
    SceneEntry {
        name: "tube",
        description: "Helix, trefoil knot, and L-bend tubes with animated radius and color",
        kind: SceneKind::Example,
        source: SceneSource::Builtin(examples::tube::build),
        audio: None,
    },
    SceneEntry {
        name: "vector_text",
        description: "Write-on animation with bezier-based vector text",
        kind: SceneKind::Example,
        source: SceneSource::Builtin(examples::vector_text::build),
        audio: None,
    },
    SceneEntry {
        name: "bouncing_ball_long",
        description: "20-second bouncing ball production with animated camera orbit",
        kind: SceneKind::Video,
        source: SceneSource::Builtin(videos::bouncing_ball::build),
        audio: None,
    },
    SceneEntry {
        name: "torus_knot",
        description: "Rainbow torus knot with dolly zoom and ring sweep animations",
        kind: SceneKind::Video,
        source: SceneSource::Builtin(videos::torus_knot::build),
        audio: None,
    },
    SceneEntry {
        name: "basic_l_system",
        description: "A basic l system that animates theta and progress changing",
        kind: SceneKind::Video,
        source: SceneSource::Builtin(videos::basic_l_system::build),
        audio: None,
    },
    SceneEntry {
        name: "turtle_intro",
        description: "An intro to drawing with turtle graphics and the L-System rules",
        kind: SceneKind::Video,
        source: SceneSource::Builtin(videos::turtle_intro::build),
        audio: None,
    },
];

/// All scenes: builtins plus documents discovered in `scenes/*.ron`.
/// Discovered once per process; entries live for the process lifetime.
pub fn scenes() -> &'static [SceneEntry] {
    static ALL: OnceLock<Vec<SceneEntry>> = OnceLock::new();
    ALL.get_or_init(|| {
        let mut all = BUILTIN_SCENES.to_vec();
        all.extend(discover_docs());
        all
    })
}

fn discover_docs() -> Vec<SceneEntry> {
    let Ok(read_dir) = std::fs::read_dir("scenes") else {
        return Vec::new();
    };
    let mut paths: Vec<_> = read_dir
        .filter_map(|e| e.ok())
        .map(|e| e.path())
        .filter(|p| p.extension().is_some_and(|ext| ext == "ron"))
        .collect();
    paths.sort();

    paths
        .into_iter()
        .filter_map(|path| {
            let name = path.file_stem()?.to_str()?.to_string();
            let path_str = path.to_str()?.to_string();
            // Parse now so the library can show the description (or the error).
            let description = match doc::SceneDoc::load(&path_str) {
                Ok(doc) if doc.description.is_empty() => format!("Scene document ({path_str})"),
                Ok(doc) => doc.description,
                Err(error) => format!("LOAD ERROR: {error}"),
            };
            Some(SceneEntry {
                name: leak(name),
                description: leak(description),
                kind: SceneKind::Doc,
                source: SceneSource::Doc(leak(path_str)),
                audio: None,
            })
        })
        .collect()
}

/// Registry entries live for the process lifetime; leaking their strings
/// keeps `SceneEntry` uniformly `&'static` for both builtins and documents.
fn leak(s: String) -> &'static str {
    Box::leak(s.into_boxed_str())
}

pub fn find(name: &str) -> Option<&'static SceneEntry> {
    scenes().iter().find(|e| e.name == name)
}

pub fn names() -> Vec<&'static str> {
    scenes().iter().map(|e| e.name).collect()
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
