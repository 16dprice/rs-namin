use crate::scene::Scene;

use super::track::Track;

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

    pub fn apply(&self, time: f32, scene: &mut Scene) {
        for track in &self.tracks {
            if let Some(value) = track.evaluate(time)
                && let Some(obj) = scene.get_mut(track.object_id)
            {
                obj.set(&track.property_name, value);
            }
        }
    }

    pub fn duration(&self) -> f32 {
        self.tracks
            .iter()
            .filter_map(|track| track.max_time())
            .fold(0.0_f32, f32::max)
    }
}

impl Default for Timeline {
    fn default() -> Self {
        Self::new()
    }
}
