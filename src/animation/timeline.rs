use crate::camera::Camera;
use crate::scene::Scene;
use crate::scene::traits::Animatable;
use crate::scene::value::AnimValue;

use super::binding::Binding;
use super::track::{Track, TrackTarget};

pub struct Timeline {
    pub tracks: Vec<Track>,
    /// Property bindings, applied after tracks each frame. Kept in dependency
    /// order — populate via [`add_binding`](Self::add_binding), then call
    /// [`sort_bindings`](Self::sort_bindings) once before playback.
    pub bindings: Vec<Binding>,
}

impl Timeline {
    pub fn new() -> Self {
        Self {
            tracks: Vec::new(),
            bindings: Vec::new(),
        }
    }

    pub fn add_track(&mut self, track: Track) {
        self.tracks.push(track);
    }

    pub fn add_binding(&mut self, binding: Binding) {
        self.bindings.push(binding);
    }

    /// Topologically sort bindings so chained bindings evaluate in dependency
    /// order. Call once after all bindings are added (SceneBuilder and doc
    /// builds do this). On a cycle, returns the targets that could not be
    /// ordered; the bindings are discarded.
    pub fn sort_bindings(&mut self) -> Result<(), Vec<TrackTarget>> {
        let bindings = std::mem::take(&mut self.bindings);
        self.bindings = Binding::sort(bindings)?;
        Ok(())
    }

    /// Apply all tracks (scene objects + camera) at the given time, then all
    /// bindings in dependency order.
    pub fn apply(&self, time: f32, scene: &mut Scene, camera: &mut Camera) {
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
        for binding in &self.bindings {
            if !binding.active_at(time) {
                continue;
            }
            let Some(value) = Self::binding_value(binding, scene, camera) else {
                continue;
            };
            match binding.target {
                TrackTarget::Object(id) => {
                    if let Some(obj) = scene.get_mut(id) {
                        obj.set(&binding.target_property, value);
                    }
                }
                TrackTarget::Camera => camera.set(&binding.target_property, value),
            }
        }
    }

    /// Apply only scene object tracks and bindings, skipping anything that
    /// writes the camera. Bindings may still *read* the camera (`camera` is
    /// the live one, e.g. the orbit camera in manual mode).
    pub fn apply_scene_only(&self, time: f32, scene: &mut Scene, camera: &Camera) {
        for track in &self.tracks {
            if let TrackTarget::Object(id) = track.target
                && let Some(value) = track.evaluate(time)
                && let Some(obj) = scene.get_mut(id)
            {
                obj.set(&track.property_name, value);
            }
        }
        for binding in &self.bindings {
            if let TrackTarget::Object(id) = binding.target
                && binding.active_at(time)
                && let Some(value) = Self::binding_value(binding, scene, camera)
                && let Some(obj) = scene.get_mut(id)
            {
                obj.set(&binding.target_property, value);
            }
        }
    }

    /// The value a binding writes this frame: the source property, plus the
    /// optional offset.
    fn binding_value(binding: &Binding, scene: &Scene, camera: &Camera) -> Option<AnimValue> {
        let value = match binding.source {
            TrackTarget::Object(id) => scene.get(id)?.get(&binding.source_property)?,
            TrackTarget::Camera => camera.get(&binding.source_property)?,
        };
        Some(match &binding.offset {
            Some(offset) => value.add_offset(offset),
            None => value,
        })
    }

    pub fn duration(&self) -> f32 {
        // Finite binding-window edges count: "bound until 10s" should be
        // reachable by the transport even if no track goes that far.
        let track_max = self.tracks.iter().filter_map(|track| track.max_time()).fold(0.0_f32, f32::max);
        self.bindings
            .iter()
            .flat_map(|b| [b.start, b.end])
            .flatten()
            .fold(track_max, f32::max)
    }
}

impl Default for Timeline {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use macroquad::prelude::{Vec3, WHITE, vec3};

    use super::*;
    use crate::animation::track::Keyframe;
    use crate::scene::ObjectId;
    use crate::scene::objects::Disk;

    fn scene_with_disks(n: usize) -> (Scene, Vec<ObjectId>) {
        let mut scene = Scene::new();
        let ids = (0..n).map(|_| scene.add(Disk::new(Vec3::ZERO, 1.0, WHITE))).collect();
        (scene, ids)
    }

    fn vec3_prop(scene: &Scene, id: ObjectId, prop: &str) -> Vec3 {
        match scene.get(id).unwrap().get(prop).unwrap() {
            AnimValue::Vec3(v) => v,
            other => panic!("expected Vec3, got {other:?}"),
        }
    }

    fn position_track(id: ObjectId, to: Vec3) -> Track {
        let mut track = Track::new(id, "position");
        track.add_keyframe(Keyframe::new(0.0, AnimValue::Vec3(Vec3::ZERO)));
        track.add_keyframe(Keyframe::new(1.0, AnimValue::Vec3(to)));
        track
    }

    fn bind(target: ObjectId, source: ObjectId, offset: Option<AnimValue>) -> Binding {
        Binding {
            target: TrackTarget::Object(target),
            target_property: "position".to_string(),
            source: TrackTarget::Object(source),
            source_property: "position".to_string(),
            offset,
            start: None,
            end: None,
        }
    }

    #[test]
    fn binding_copies_source_after_tracks() {
        let (mut scene, ids) = scene_with_disks(2);
        let mut timeline = Timeline::new();
        timeline.add_track(position_track(ids[0], vec3(10.0, 0.0, 0.0)));
        timeline.add_binding(bind(ids[1], ids[0], None));

        let mut camera = Camera::default();
        timeline.apply(1.0, &mut scene, &mut camera);
        assert_eq!(vec3_prop(&scene, ids[1], "position"), vec3(10.0, 0.0, 0.0));

        // Scrubbing back re-derives the bound value too.
        timeline.apply(0.5, &mut scene, &mut camera);
        assert_eq!(vec3_prop(&scene, ids[1], "position"), vec3(5.0, 0.0, 0.0));
    }

    #[test]
    fn binding_adds_offset() {
        let (mut scene, ids) = scene_with_disks(2);
        let mut timeline = Timeline::new();
        timeline.add_track(position_track(ids[0], vec3(10.0, 0.0, 0.0)));
        timeline.add_binding(bind(ids[1], ids[0], Some(AnimValue::Vec3(vec3(0.0, 1.5, 0.0)))));

        let mut camera = Camera::default();
        timeline.apply(1.0, &mut scene, &mut camera);
        assert_eq!(vec3_prop(&scene, ids[1], "position"), vec3(10.0, 1.5, 0.0));
    }

    #[test]
    fn chained_bindings_apply_in_dependency_order_after_sort() {
        let (mut scene, ids) = scene_with_disks(3);
        let mut timeline = Timeline::new();
        timeline.add_track(position_track(ids[0], vec3(10.0, 0.0, 0.0)));
        // Added in reverse dependency order; sort fixes it. Without the sort
        // the chain end would lag by an evaluation.
        timeline.add_binding(bind(ids[2], ids[1], Some(AnimValue::Vec3(vec3(0.0, 1.0, 0.0)))));
        timeline.add_binding(bind(ids[1], ids[0], Some(AnimValue::Vec3(vec3(0.0, 1.0, 0.0)))));
        timeline.sort_bindings().unwrap();

        let mut camera = Camera::default();
        timeline.apply(1.0, &mut scene, &mut camera);
        assert_eq!(vec3_prop(&scene, ids[2], "position"), vec3(10.0, 2.0, 0.0));
    }

    #[test]
    fn windowed_binding_yields_to_track_outside_its_window() {
        let (mut scene, ids) = scene_with_disks(2);
        let mut timeline = Timeline::new();
        // Leader sweeps to (10,0,0) over 1s; follower has its own track to
        // (0,-5,0) AND a binding to the leader active only in [0.25, 0.75).
        timeline.add_track(position_track(ids[0], vec3(10.0, 0.0, 0.0)));
        timeline.add_track(position_track(ids[1], vec3(0.0, -5.0, 0.0)));
        let mut binding = bind(ids[1], ids[0], None);
        binding.start = Some(0.25);
        binding.end = Some(0.75);
        timeline.add_binding(binding);

        let mut camera = Camera::default();
        // Before the window: own track.
        timeline.apply(0.2, &mut scene, &mut camera);
        assert_eq!(vec3_prop(&scene, ids[1], "position"), vec3(0.0, -1.0, 0.0));
        // Inside: bound to the leader.
        timeline.apply(0.5, &mut scene, &mut camera);
        assert_eq!(vec3_prop(&scene, ids[1], "position"), vec3(5.0, 0.0, 0.0));
        // After: own track again.
        timeline.apply(0.8, &mut scene, &mut camera);
        assert_eq!(vec3_prop(&scene, ids[1], "position"), vec3(0.0, -4.0, 0.0));
    }

    #[test]
    fn duration_includes_finite_window_edges() {
        let (_, ids) = scene_with_disks(2);
        let mut timeline = Timeline::new();
        timeline.add_track(position_track(ids[0], Vec3::ONE)); // max time 1.0
        let mut binding = bind(ids[1], ids[0], None);
        binding.end = Some(10.0);
        timeline.add_binding(binding);
        assert!((timeline.duration() - 10.0).abs() < 1e-6);
    }

    #[test]
    fn binding_can_target_camera_in_full_apply() {
        let (mut scene, ids) = scene_with_disks(1);
        let mut timeline = Timeline::new();
        let mut track = Track::new(ids[0], "radius");
        track.add_keyframe(Keyframe::new(0.0, AnimValue::Float(42.0)));
        timeline.add_track(track);
        timeline.add_binding(Binding {
            target: TrackTarget::Camera,
            target_property: "fov".to_string(),
            source: TrackTarget::Object(ids[0]),
            source_property: "radius".to_string(),
            offset: None,
            start: None,
            end: None,
        });

        let mut camera = Camera::default();
        timeline.apply(0.0, &mut scene, &mut camera);
        assert!((camera.fov - 42.0).abs() < 1e-5);
    }

    #[test]
    fn apply_scene_only_skips_camera_targets_but_reads_camera_sources() {
        let (mut scene, ids) = scene_with_disks(2);
        let mut timeline = Timeline::new();
        timeline.add_binding(Binding {
            target: TrackTarget::Camera,
            target_property: "fov".to_string(),
            source: TrackTarget::Object(ids[0]),
            source_property: "radius".to_string(),
            offset: None,
            start: None,
            end: None,
        });
        timeline.add_binding(Binding {
            target: TrackTarget::Object(ids[1]),
            target_property: "position".to_string(),
            source: TrackTarget::Camera,
            source_property: "position".to_string(),
            offset: None,
            start: None,
            end: None,
        });

        let camera = Camera::new(vec3(5.0, 6.0, 7.0), Vec3::ZERO);
        let fov_before = camera.fov;
        timeline.apply_scene_only(0.0, &mut scene, &camera);
        // Camera-target binding skipped (camera is immutable here)…
        assert!((camera.fov - fov_before).abs() < 1e-5);
        // …but the camera-sourced binding still drives the object.
        assert_eq!(vec3_prop(&scene, ids[1], "position"), vec3(5.0, 6.0, 7.0));
    }

    #[test]
    fn binding_with_missing_source_object_is_skipped() {
        let (mut scene, ids) = scene_with_disks(2);
        let mut timeline = Timeline::new();
        timeline.add_binding(bind(ids[1], ids[0], None));
        scene.remove(ids[0]);

        let mut camera = Camera::default();
        timeline.apply(0.0, &mut scene, &mut camera);
        assert_eq!(vec3_prop(&scene, ids[1], "position"), Vec3::ZERO);
    }

    #[test]
    fn sort_bindings_reports_cycle() {
        let mut timeline = Timeline::new();
        let (_, ids) = scene_with_disks(2);
        timeline.add_binding(bind(ids[0], ids[1], None));
        timeline.add_binding(bind(ids[1], ids[0], None));
        assert!(timeline.sort_bindings().is_err());
    }
}
