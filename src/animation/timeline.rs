use crate::camera::Camera;
use crate::scene::Scene;

use super::track::{Track, TrackTarget};

pub struct Timeline {
    pub tracks: Vec<Track>,
}

impl Timeline {
    pub fn new() -> Self {
        Self { tracks: Vec::new() }
    }

    pub fn add_track(&mut self, track: Track) {
        self.tracks.push(track);
    }

    /// Apply all tracks (scene objects + camera) at the given time.
    pub fn apply(&self, time: f32, scene: &mut Scene, camera: &mut Camera) {
        use crate::scene::traits::Animatable;
        for track in &self.tracks {
            if let Some(value) = track.evaluate(time) {
                match track.target {
                    TrackTarget::Object(id) => {
                        if let Some(obj) = scene.get_mut(id) {
                            obj.set(&track.property_name, value);
                        }
                    }
                    TrackTarget::Camera => {
                        camera.set(&track.property_name, value);
                    }
                }
            }
        }
    }

    /// Apply only scene object tracks, skipping camera tracks.
    pub fn apply_scene_only(&self, time: f32, scene: &mut Scene) {
        for track in &self.tracks {
            if let TrackTarget::Object(id) = track.target
                && let Some(value) = track.evaluate(time)
                && let Some(obj) = scene.get_mut(id)
            {
                obj.set(&track.property_name, value);
            }
        }
    }

    pub fn duration(&self) -> f32 {
        self.tracks.iter().filter_map(|track| track.max_time()).fold(0.0_f32, f32::max)
    }
}

impl Default for Timeline {
    fn default() -> Self {
        Self::new()
    }
}
