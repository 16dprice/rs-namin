//! Scene-document editor: object palette and property inspector.
//!
//! The document is the single source of truth — every edit marks the state
//! dirty and queues a scene rebuild (`ViewerMode` rebuilds from the doc at
//! the end of the frame). Document mutations live on `EditorState` as plain
//! methods so they stay unit-testable; the egui panels call into them.

use egui_macroquad::egui;
use macroquad::prelude::*;

use crate::animation::easing::Easing;
use crate::animation::track::Track;
use crate::camera::Camera;
use crate::clock::Clock;
use crate::doc::{BindingDoc, KeyframeDoc, ObjectDoc, ObjectSpec, SceneDoc, TrackDoc};
use crate::scene::traits::Animatable;
use crate::scene::value::AnimValue;

pub struct EditorState {
    pub doc: SceneDoc,
    pub path: &'static str,
    pub selected: Option<usize>,
    /// The camera is selected in the palette (mutually exclusive with
    /// `selected`); the inspector shows the camera page.
    pub camera_selected: bool,
    /// Selected keyframe in the dope sheet: (track index, keyframe index).
    pub selected_keyframe: Option<(usize, usize)>,
    pub dirty: bool,
    /// A doc edit happened this frame; the owner rebuilds the scene.
    pub rebuild_needed: bool,
    /// The scene file was renamed this frame; the owner re-resolves its
    /// registry entry.
    pub renamed: bool,
    /// Last build or save error (editing continues).
    pub error: Option<String>,
    /// Buffer for the id TextEdit (committed on focus loss / enter).
    id_buffer: String,
    /// Buffer for the scene-name TextEdit (committed on focus loss / enter).
    name_buffer: String,
    /// Which bezier handle (0/1) a curve-widget drag is holding.
    bezier_drag: Option<u8>,
}

/// The scene's registry name: the file stem of its .ron path.
pub fn scene_stem(path: &str) -> &str {
    let file = path.rsplit(['/', '\\']).next().unwrap_or(path);
    file.strip_suffix(".ron").unwrap_or(file)
}

impl EditorState {
    pub fn new(doc: SceneDoc, path: &'static str) -> Self {
        let mut state = Self {
            doc,
            path,
            selected: None,
            camera_selected: false,
            selected_keyframe: None,
            dirty: false,
            rebuild_needed: false,
            renamed: false,
            error: None,
            id_buffer: String::new(),
            name_buffer: scene_stem(path).to_string(),
            bezier_drag: None,
        };
        if !state.doc.objects.is_empty() {
            state.select(Some(0));
        }
        state
    }

    fn touch(&mut self) {
        self.dirty = true;
        self.rebuild_needed = true;
    }

    pub fn select(&mut self, index: Option<usize>) {
        self.selected = index;
        self.camera_selected = false;
        self.id_buffer = match index {
            Some(i) => self.doc.objects[i].id.clone(),
            None => String::new(),
        };
    }

    pub fn select_camera(&mut self) {
        self.selected = None;
        self.camera_selected = true;
        self.id_buffer = String::new();
    }

    /// Add a new object from a palette template with a unique generated id.
    /// Returns the new object's index.
    pub fn add_object(&mut self, type_name: &str, spec: ObjectSpec) -> usize {
        let base = type_name.to_lowercase();
        let mut n = 1;
        let mut id = format!("{base}_{n}");
        while self.doc.objects.iter().any(|o| o.id == id) {
            n += 1;
            id = format!("{base}_{n}");
        }
        self.doc.objects.push(ObjectDoc {
            id,
            object: spec,
            set: Vec::new(),
        });
        self.touch();
        self.doc.objects.len() - 1
    }

    /// Remove an object and every track and binding that references it.
    pub fn remove_object(&mut self, index: usize) {
        let id = self.doc.objects.remove(index).id;
        self.doc.tracks.retain(|t| t.object != id);
        self.doc.bindings.retain(|b| b.target != id && b.source != id);
        match self.selected {
            Some(s) if s == index => self.select(None),
            Some(s) if s > index => self.selected = Some(s - 1),
            _ => {}
        }
        self.touch();
    }

    /// Duplicate an object: the same spec and initial overrides under a
    /// fresh id, nudged slightly so the copy is visible beside the original.
    /// Tracks, keyframes, and bindings are deliberately NOT copied — this is
    /// "add a new object whose starting values come from an existing one".
    /// Returns the new object's index.
    pub fn duplicate_object(&mut self, index: usize) -> usize {
        let source = self.doc.objects[index].clone();
        let mut id = format!("{}_copy", source.id);
        let mut n = 2;
        while self.doc.objects.iter().any(|o| o.id == id) {
            id = format!("{}_copy_{n}", source.id);
            n += 1;
        }
        self.doc.objects.push(ObjectDoc {
            id,
            object: source.object,
            set: source.set,
        });
        let new_index = self.doc.objects.len() - 1;
        // Nudge spatial anchors (position, or start+end for Line/Arrow) so
        // the copy doesn't sit exactly behind the original.
        for property in ["position", "start", "end"] {
            match self.effective_value(new_index, property) {
                Some(AnimValue::Vec3(v)) => {
                    self.upsert_override(new_index, property, AnimValue::Vec3(v + vec3(0.4, -0.4, 0.0)));
                }
                Some(AnimValue::Vec2(v)) => {
                    // Screen-space Text: design-canvas pixels, y-down.
                    self.upsert_override(new_index, property, AnimValue::Vec2(v + vec2(25.0, 25.0)));
                }
                _ => {}
            }
        }
        self.touch();
        new_index
    }

    /// Rename an object, cascading to tracks and bindings. Rejects empty,
    /// duplicate, and reserved ids; returns whether the rename applied.
    pub fn rename_object(&mut self, index: usize, new_id: &str) -> bool {
        let new_id = new_id.trim();
        let old_id = self.doc.objects[index].id.clone();
        if new_id == old_id {
            return true;
        }
        if new_id.is_empty() || new_id == "camera" || self.doc.objects.iter().any(|o| o.id == new_id) {
            return false;
        }
        self.doc.objects[index].id = new_id.to_string();
        for track in &mut self.doc.tracks {
            if track.object == old_id {
                track.object = new_id.to_string();
            }
        }
        for binding in &mut self.doc.bindings {
            if binding.target == old_id {
                binding.target = new_id.to_string();
            }
            if binding.source == old_id {
                binding.source = new_id.to_string();
            }
        }
        self.touch();
        true
    }

    /// Set an initial-property override (insert or update).
    pub fn upsert_override(&mut self, index: usize, property: &str, value: AnimValue) {
        let set = &mut self.doc.objects[index].set;
        match set.iter_mut().find(|(p, _)| p == property) {
            Some((_, v)) => *v = value,
            None => set.push((property.to_string(), value)),
        }
        self.touch();
    }

    /// Effective initial value of a property: spec-spawned, then overrides.
    pub fn effective_value(&self, index: usize, property: &str) -> Option<AnimValue> {
        let obj = &self.doc.objects[index];
        let mut instance = obj.object.spawn();
        for (p, v) in &obj.set {
            instance.set(p, v.clone());
        }
        instance.get(property)
    }

    /// Animatable property names of the object's type.
    pub fn property_names(&self, index: usize) -> Vec<String> {
        let instance = self.doc.objects[index].object.spawn();
        instance.property_names().iter().map(|s| s.to_string()).collect()
    }

    /// Property names for a track target: an object id or "camera".
    pub fn target_property_names(&self, target: &str) -> Option<Vec<String>> {
        if target == "camera" {
            return Some(Camera::default().property_names().iter().map(|s| s.to_string()).collect());
        }
        let index = self.doc.objects.iter().position(|o| o.id == target)?;
        Some(self.property_names(index))
    }

    /// Property names usable as a binding source: settable properties plus
    /// read-only outputs (e.g. LSystem's `pen_position`).
    pub fn source_property_names(&self, target: &str) -> Option<Vec<String>> {
        if target == "camera" {
            return self.target_property_names(target);
        }
        let index = self.doc.objects.iter().position(|o| o.id == target)?;
        let instance = self.doc.objects[index].object.spawn();
        Some(
            instance
                .property_names()
                .iter()
                .chain(instance.output_names().iter())
                .map(|s| s.to_string())
                .collect(),
        )
    }

    /// Effective initial value of a camera property: the doc's camera fields
    /// plus its `set` overrides.
    pub fn camera_effective_value(&self, property: &str) -> Option<AnimValue> {
        let mut camera = Camera::new(self.doc.camera.position, self.doc.camera.target);
        camera.fov = self.doc.camera.fov;
        for (p, v) in &self.doc.camera.set {
            camera.set(p, v.clone());
        }
        camera.get(property)
    }

    /// Set an initial camera property. `position`/`target`/`fov` write the
    /// typed CameraDoc fields (removing any shadowing `set` entry); anything
    /// else becomes a `set` override.
    pub fn upsert_camera_override(&mut self, property: &str, value: AnimValue) {
        match (property, &value) {
            ("position", AnimValue::Vec3(v)) => self.doc.camera.position = *v,
            ("target", AnimValue::Vec3(v)) => self.doc.camera.target = *v,
            ("fov", AnimValue::Float(f)) => self.doc.camera.fov = *f,
            _ => {
                match self.doc.camera.set.iter_mut().find(|(p, _)| p == property) {
                    Some((_, v)) => *v = value,
                    None => self.doc.camera.set.push((property.to_string(), value)),
                }
                self.touch();
                return;
            }
        }
        // A typed field was written; drop any set entry that would shadow it.
        self.doc.camera.set.retain(|(p, _)| p != property);
        self.touch();
    }

    /// The AnimValue a track's keyframes must carry (from the target's
    /// property type).
    fn target_value_template(&self, target: &str, property: &str) -> Option<AnimValue> {
        if target == "camera" {
            return self.camera_effective_value(property);
        }
        let index = self.doc.objects.iter().position(|o| o.id == target)?;
        self.effective_value(index, property)
    }

    /// Add a track for `target.property` with one keyframe at t=0 holding
    /// the current initial value. Rejects unknown targets/properties,
    /// duplicate tracks, and always-bound properties (windowed bindings
    /// coexist with tracks).
    pub fn add_track(&mut self, target: &str, property: &str) -> Result<usize, String> {
        if self.doc.tracks.iter().any(|t| t.object == target && t.property == property) {
            return Err(format!("track {target}.{property} already exists"));
        }
        if self
            .doc
            .bindings
            .iter()
            .any(|b| b.target == target && b.property == property && !b.is_windowed())
        {
            return Err(format!("{target}.{property} is always-bound — window or remove the binding first"));
        }
        let value = self
            .target_value_template(target, property)
            .ok_or_else(|| format!("no property {property:?} on {target:?}"))?;
        self.doc.tracks.push(TrackDoc {
            object: target.to_string(),
            property: property.to_string(),
            keyframes: vec![KeyframeDoc {
                time: 0.0,
                value,
                easing: Easing::Linear,
                steps: None,
            }],
        });
        self.touch();
        Ok(self.doc.tracks.len() - 1)
    }

    /// Add a binding locking `target.property` to `source.source_property`.
    /// Validated by a trial doc build (unknown ids, types, window overlaps,
    /// track conflicts, cycles); on failure the doc is unchanged and the
    /// build error is returned.
    pub fn add_binding(&mut self, target: &str, property: &str, source: &str, source_property: &str) -> Result<usize, String> {
        self.add_binding_windowed(target, property, source, source_property, None, None)
    }

    /// Like [`add_binding`](Self::add_binding) with an active time window —
    /// required when the property already has a track or another binding.
    pub fn add_binding_windowed(
        &mut self,
        target: &str,
        property: &str,
        source: &str,
        source_property: &str,
        start: Option<f32>,
        end: Option<f32>,
    ) -> Result<usize, String> {
        self.doc.bindings.push(BindingDoc {
            target: target.to_string(),
            property: property.to_string(),
            source: source.to_string(),
            source_property: source_property.to_string(),
            offset: None,
            start,
            end,
        });
        match self.doc.build() {
            Ok(_) => {
                self.touch();
                Ok(self.doc.bindings.len() - 1)
            }
            Err(e) => {
                self.doc.bindings.pop();
                Err(e)
            }
        }
    }

    /// Set or clear a binding's time window, reverting on a failed trial
    /// build (overlap with a sibling binding, inverted window).
    pub fn set_binding_window(&mut self, index: usize, start: Option<f32>, end: Option<f32>) -> Result<(), String> {
        let previous = (self.doc.bindings[index].start, self.doc.bindings[index].end);
        self.doc.bindings[index].start = start;
        self.doc.bindings[index].end = end;
        match self.doc.build() {
            Ok(_) => {
                self.touch();
                Ok(())
            }
            Err(e) => {
                (self.doc.bindings[index].start, self.doc.bindings[index].end) = previous;
                Err(e)
            }
        }
    }

    pub fn remove_binding(&mut self, index: usize) {
        self.doc.bindings.remove(index);
        self.touch();
    }

    /// Set or clear a binding's offset, reverting on a failed trial build
    /// (wrong variant, non-offsetable property type).
    pub fn set_binding_offset(&mut self, index: usize, offset: Option<AnimValue>) -> Result<(), String> {
        let previous = std::mem::replace(&mut self.doc.bindings[index].offset, offset);
        match self.doc.build() {
            Ok(_) => {
                self.touch();
                Ok(())
            }
            Err(e) => {
                self.doc.bindings[index].offset = previous;
                Err(e)
            }
        }
    }

    /// Index of the first binding driving `object_id.property`, if any.
    pub fn binding_for(&self, object_id: &str, property: &str) -> Option<usize> {
        self.doc
            .bindings
            .iter()
            .position(|b| b.target == object_id && b.property == property)
    }

    /// Indices of every binding driving `object_id.property` (several may
    /// share a property with disjoint windows).
    pub fn bindings_for(&self, object_id: &str, property: &str) -> Vec<usize> {
        self.doc
            .bindings
            .iter()
            .enumerate()
            .filter(|(_, b)| b.target == object_id && b.property == property)
            .map(|(i, _)| i)
            .collect()
    }

    /// Whether binding `target.<prop>` to `source` would create a dependency
    /// cycle at object granularity: true if `source` already depends on
    /// `target` (directly or through a chain of bindings), or is `target`.
    pub fn binding_would_cycle(&self, target: &str, source: &str) -> bool {
        let mut stack = vec![source.to_string()];
        let mut visited = Vec::new();
        while let Some(node) = stack.pop() {
            if node == target {
                return true;
            }
            if visited.contains(&node) {
                continue;
            }
            for b in self.doc.bindings.iter().filter(|b| b.target == node) {
                stack.push(b.source.clone());
            }
            visited.push(node);
        }
        false
    }

    pub fn remove_track(&mut self, index: usize) {
        self.doc.tracks.remove(index);
        match self.selected_keyframe {
            Some((t, _)) if t == index => self.selected_keyframe = None,
            Some((t, k)) if t > index => self.selected_keyframe = Some((t - 1, k)),
            _ => {}
        }
        self.touch();
    }

    /// The track's animated value at `time` (interpolated), falling back to
    /// the target's initial value for empty tracks.
    pub fn track_value_at(&self, track_index: usize, time: f32) -> Option<AnimValue> {
        let track_doc = &self.doc.tracks[track_index];
        if track_doc.keyframes.is_empty() {
            return self.target_value_template(&track_doc.object, &track_doc.property);
        }
        // A temporary runtime track purely for evaluation (the camera
        // constructor is just the target-free way to build one).
        let mut track = Track::camera(track_doc.property.clone());
        for kf in &track_doc.keyframes {
            track.add_keyframe(
                crate::animation::track::Keyframe::with_easing(kf.time, kf.value.clone(), kf.easing).with_steps(kf.steps.unwrap_or(1)),
            );
        }
        track.evaluate(time)
    }

    /// Add a keyframe at `time` carrying the track's interpolated value there.
    /// Returns the new keyframe's index.
    pub fn add_keyframe(&mut self, track_index: usize, time: f32) -> Option<usize> {
        let value = self.track_value_at(track_index, time)?;
        let keyframes = &mut self.doc.tracks[track_index].keyframes;
        keyframes.push(KeyframeDoc {
            time: time.max(0.0),
            value,
            easing: Easing::Linear,
            steps: None,
        });
        let index = keyframes.len() - 1;
        self.touch();
        Some(index)
    }

    pub fn remove_keyframe(&mut self, track_index: usize, kf_index: usize) {
        self.doc.tracks[track_index].keyframes.remove(kf_index);
        match self.selected_keyframe {
            Some((t, k)) if t == track_index && k == kf_index => self.selected_keyframe = None,
            Some((t, k)) if t == track_index && k > kf_index => self.selected_keyframe = Some((t, k - 1)),
            _ => {}
        }
        self.touch();
    }

    /// Move a keyframe in time. Indices stay stable (the doc list is not
    /// re-sorted — the engine sorts at build; `save` normalizes order).
    pub fn move_keyframe(&mut self, track_index: usize, kf_index: usize, time: f32) {
        self.doc.tracks[track_index].keyframes[kf_index].time = time.max(0.0);
        self.touch();
    }

    pub fn set_keyframe_easing(&mut self, track_index: usize, kf_index: usize, easing: Easing) {
        self.doc.tracks[track_index].keyframes[kf_index].easing = easing;
        self.touch();
    }

    /// Set the keyframe's sub-step count (arrive in N eased steps); 0/1
    /// stores as None (plain interpolation, no field in the file).
    pub fn set_keyframe_steps(&mut self, track_index: usize, kf_index: usize, steps: u32) {
        self.doc.tracks[track_index].keyframes[kf_index].steps = (steps > 1).then_some(steps);
        self.touch();
    }

    /// Replace a keyframe's value; rejects a variant mismatch with the
    /// track's property type.
    pub fn set_keyframe_value(&mut self, track_index: usize, kf_index: usize, value: AnimValue) -> bool {
        let track = &self.doc.tracks[track_index];
        let Some(template) = self.target_value_template(&track.object, &track.property) else {
            return false;
        };
        if std::mem::discriminant(&template) != std::mem::discriminant(&value) {
            return false;
        }
        self.doc.tracks[track_index].keyframes[kf_index].value = value;
        self.touch();
        true
    }

    /// Write a property value the auto-key way: if the property has a track,
    /// update the keyframe at `time` (or insert one there) and select it;
    /// otherwise write an initial-state override. Used by viewport dragging.
    pub fn auto_key(&mut self, object_index: usize, property: &str, value: AnimValue, time: f32) {
        let id = self.doc.objects[object_index].id.clone();
        if let Some(track_index) = self.doc.tracks.iter().position(|t| t.object == id && t.property == property) {
            let keyframes = &mut self.doc.tracks[track_index].keyframes;
            let kf_index = match keyframes.iter().position(|k| (k.time - time).abs() < 1e-3) {
                Some(i) => {
                    keyframes[i].value = value;
                    i
                }
                None => {
                    keyframes.push(KeyframeDoc {
                        time,
                        value,
                        easing: Easing::default(),
                        steps: None,
                    });
                    keyframes.len() - 1
                }
            };
            self.selected_keyframe = Some((track_index, kf_index));
            self.touch();
        } else {
            self.upsert_override(object_index, property, value);
        }
    }

    pub fn save(&mut self) -> Result<(), String> {
        for track in &mut self.doc.tracks {
            track.keyframes.sort_by(|a, b| a.time.total_cmp(&b.time));
        }
        self.selected_keyframe = None;
        let ron = self.doc.to_ron_string()?;
        std::fs::write(self.path, ron).map_err(|e| format!("cannot write {}: {e}", self.path))?;
        self.dirty = false;
        Ok(())
    }

    /// Rename the scene: move the backing .ron file and update `path` (the
    /// registry name is the file stem). Rejects empty or path-escaping
    /// names, and collisions with built-in scenes or existing files. On
    /// success sets `renamed` so the owner re-resolves its registry entry.
    pub fn rename_scene(&mut self, new_name: &str) -> Result<(), String> {
        let new_name = new_name.trim();
        if new_name == scene_stem(self.path) {
            return Ok(());
        }
        if new_name.is_empty() {
            return Err("scene name cannot be empty".to_string());
        }
        if new_name.contains('/') || new_name.contains('\\') || new_name.starts_with('.') {
            return Err(format!("invalid scene name {new_name:?}"));
        }
        // Built-ins can never be shadowed. Doc entries are checked against
        // the filesystem instead of the registry snapshot, which may hold a
        // stale name from a previous rename in this session.
        if let Some(entry) = crate::registry::find(new_name)
            && matches!(entry.source, crate::registry::SceneSource::Builtin(_))
        {
            return Err(format!("{new_name:?} is a built-in scene name"));
        }
        let dir = std::path::Path::new(self.path).parent().filter(|d| !d.as_os_str().is_empty());
        let new_path = dir.unwrap_or_else(|| std::path::Path::new(".")).join(format!("{new_name}.ron"));
        if new_path.exists() {
            return Err(format!("{} already exists", new_path.display()));
        }
        std::fs::rename(self.path, &new_path).map_err(|e| format!("cannot rename to {}: {e}", new_path.display()))?;
        self.path = Box::leak(new_path.to_string_lossy().into_owned().into_boxed_str());
        self.name_buffer = new_name.to_string();
        self.renamed = true;
        Ok(())
    }
}

/// Palette templates: every spawnable object type with sensible defaults.
pub fn palette_templates() -> Vec<(&'static str, ObjectSpec)> {
    let white = vec4(1.0, 1.0, 1.0, 1.0);
    vec![
        (
            "Disk",
            ObjectSpec::Disk {
                position: Vec3::ZERO,
                radius: 1.0,
                color: white,
            },
        ),
        (
            "Ring",
            ObjectSpec::Ring {
                position: Vec3::ZERO,
                radius: 1.0,
                color: white,
                progress: 1.0,
            },
        ),
        (
            "Rectangle",
            ObjectSpec::Rectangle {
                position: Vec3::ZERO,
                size: vec2(2.0, 1.0),
                color: white,
            },
        ),
        (
            "Polygon",
            ObjectSpec::Polygon {
                position: Vec3::ZERO,
                radius: 1.0,
                sides: 6,
                color: white,
            },
        ),
        (
            "Line",
            ObjectSpec::Line {
                start: vec3(-1.0, 0.0, 0.0),
                end: vec3(1.0, 0.0, 0.0),
                color: white,
                thickness: 0.05,
            },
        ),
        (
            "Arc",
            ObjectSpec::Arc {
                position: Vec3::ZERO,
                inner_radius: 0.6,
                outer_radius: 1.0,
                start_angle: 0.0,
                sweep_angle: std::f32::consts::PI,
                color: white,
            },
        ),
        (
            "Arrow",
            ObjectSpec::Arrow {
                start: Vec3::ZERO,
                end: vec3(2.0, 0.0, 0.0),
                color: white,
            },
        ),
        (
            "Spiral",
            ObjectSpec::Spiral {
                position: Vec3::ZERO,
                delta_radius: 0.02,
                delta_theta: 2.4,
                color: white,
                num_points: 300,
                dot_radius: 0.05,
            },
        ),
        (
            "Torus",
            ObjectSpec::Torus {
                position: Vec3::ZERO,
                major_radius: 1.5,
                minor_radius: 0.4,
                color: white,
            },
        ),
        (
            "Tube",
            ObjectSpec::Tube {
                points: vec![vec3(-1.0, 0.0, 0.0), vec3(0.0, 1.0, 0.0), vec3(1.0, 0.0, 0.0)],
                radius: 0.2,
                color: white,
                colors: Vec::new(),
                closed: false,
            },
        ),
        (
            "Text",
            ObjectSpec::Text {
                content: "text".to_string(),
                // Design-canvas center-ish: clear of the palette/inspector
                // panels that overlay the canvas edges in editor mode.
                position: vec2(600.0, 340.0),
                font_size: 40.0,
                color: white,
            },
        ),
        (
            "VectorText",
            ObjectSpec::VectorText {
                content: "vector text".to_string(),
                // Glyphs extend rightward from position; offset so the
                // default string sits roughly centered in the viewport.
                position: vec3(-2.8, 0.0, 0.0),
                scale: 1.0,
                color: white,
            },
        ),
        (
            "Sprite",
            ObjectSpec::Sprite {
                image: "assets/aseprite-files/turtle.png".to_string(),
                position: Vec3::ZERO,
                size: vec2(1.0, 1.0),
                color: white,
            },
        ),
        (
            "LSystem",
            // The dragon curve: iteration 10 at this scale roughly fills
            // the default viewport.
            ObjectSpec::LSystem {
                axiom: "F".to_string(),
                rules: vec![("F".to_string(), "F+G".to_string()), ("G".to_string(), "F-G".to_string())],
                theta: std::f32::consts::FRAC_PI_2,
                iterations: 10.0,
                position: vec3(1.0, -2.0, 0.0),
                scale: 0.15,
                color: white,
                colors: Vec::new(),
            },
        ),
        (
            "Plot",
            ObjectSpec::Plot {
                expression: "sin(x)".to_string(),
                position: Vec3::ZERO,
                size: vec2(8.0, 4.5),
                x_bounds: vec2(-6.3, 6.3),
                y_bounds: vec2(-1.5, 1.5),
                color: vec4(0.35, 0.8, 1.0, 1.0),
                samples: 200,
            },
        ),
    ]
}

fn spec_type_name(spec: &ObjectSpec) -> &'static str {
    match spec {
        ObjectSpec::Disk { .. } => "Disk",
        ObjectSpec::Ring { .. } => "Ring",
        ObjectSpec::Rectangle { .. } => "Rectangle",
        ObjectSpec::Polygon { .. } => "Polygon",
        ObjectSpec::Line { .. } => "Line",
        ObjectSpec::Arc { .. } => "Arc",
        ObjectSpec::Arrow { .. } => "Arrow",
        ObjectSpec::Spiral { .. } => "Spiral",
        ObjectSpec::Torus { .. } => "Torus",
        ObjectSpec::Tube { .. } => "Tube",
        ObjectSpec::Text { .. } => "Text",
        ObjectSpec::VectorText { .. } => "VectorText",
        ObjectSpec::Sprite { .. } => "Sprite",
        ObjectSpec::LSystem { .. } => "LSystem",
        ObjectSpec::Plot { .. } => "Plot",
    }
}

/// Draw the editor panels (palette left, inspector right). Call inside the
/// egui pass, viewer mode only.
pub fn panels(ctx: &egui::Context, editor: &mut EditorState, playhead: f32) {
    // Ctrl+S (Cmd+S on mac) saves, same as the Save button. consume_key
    // works even while a text field has keyboard focus — and eats the
    // event so no stray "s" lands in it.
    if ctx.input_mut(|i| i.consume_key(egui::Modifiers::COMMAND, egui::Key::S)) {
        save_scene(editor);
    }

    // Ctrl+D duplicates the selected object (initial properties only).
    if ctx.input_mut(|i| i.consume_key(egui::Modifiers::COMMAND, egui::Key::D))
        && let Some(index) = editor.selected
    {
        let new_index = editor.duplicate_object(index);
        editor.select(Some(new_index));
    }

    palette_panel(ctx, editor);
    if editor.camera_selected {
        camera_inspector(ctx, editor, playhead);
    } else if editor.selected.is_some() {
        inspector_panel(ctx, editor, playhead);
    }
}

fn save_scene(editor: &mut EditorState) {
    match editor.save() {
        Ok(()) => editor.error = None,
        Err(e) => editor.error = Some(e),
    }
}

fn palette_panel(ctx: &egui::Context, editor: &mut EditorState) {
    egui::SidePanel::left("editor_palette")
        .resizable(true)
        .default_width(230.0)
        .width_range(140.0..=420.0)
        .show(ctx, |ui| {
            ui.add_space(6.0);
            ui.horizontal(|ui| {
                ui.heading("Scene");
                if editor.dirty {
                    ui.colored_label(egui::Color32::LIGHT_YELLOW, "●");
                }
            });

            // Scene name = file stem = registry name; committing the edit
            // renames the .ron file.
            ui.horizontal(|ui| {
                ui.weak("name");
                let response = ui.text_edit_singleline(&mut editor.name_buffer);
                if response.lost_focus() {
                    let requested = editor.name_buffer.clone();
                    if let Err(e) = editor.rename_scene(&requested) {
                        editor.error = Some(e);
                    }
                    editor.name_buffer = scene_stem(editor.path).to_string();
                }
            });
            ui.horizontal(|ui| {
                ui.weak("desc");
                if ui.text_edit_singleline(&mut editor.doc.description).changed() {
                    editor.dirty = true;
                }
            });

            ui.add_space(4.0);
            ui.horizontal(|ui| {
                if ui.button("Save").on_hover_text("Ctrl+S").clicked() {
                    save_scene(editor);
                }
                ui.weak(editor.path);
            });

            if let Some(error) = &editor.error {
                ui.add_space(4.0);
                ui.colored_label(egui::Color32::LIGHT_RED, error);
            }

            ui.separator();

            // Add-object menu.
            ui.menu_button("+ Add object", |ui| {
                for (name, spec) in palette_templates() {
                    if ui.button(name).clicked() {
                        let index = editor.add_object(name, spec);
                        editor.select(Some(index));
                        ui.close_menu();
                    }
                }
            });

            ui.add_space(4.0);

            // Object list, with the camera as a scene-level pseudo-object.
            let mut remove: Option<usize> = None;
            let mut select: Option<usize> = None;
            egui::ScrollArea::vertical().show(ui, |ui| {
                ui.horizontal(|ui| {
                    if ui.selectable_label(editor.camera_selected, "camera").clicked() {
                        editor.select_camera();
                    }
                    ui.weak("Camera");
                });
                for (i, obj) in editor.doc.objects.iter().enumerate() {
                    ui.horizontal(|ui| {
                        let selected = editor.selected == Some(i);
                        if ui.selectable_label(selected, &obj.id).clicked() {
                            select = Some(i);
                        }
                        ui.weak(spec_type_name(&obj.object));
                        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                            if ui.small_button("x").on_hover_text("Delete object and its tracks").clicked() {
                                remove = Some(i);
                            }
                        });
                    });
                }
            });
            if let Some(i) = select {
                editor.select(Some(i));
            }
            if let Some(i) = remove {
                editor.remove_object(i);
            }
        });
}

const LSYSTEM_ALPHABET_HELP: &str = "F/G draw forward, + turns left and - turns right by theta (radians), \
[ ] push/pop the turtle state, other letters are silent rewriting variables";

fn inspector_panel(ctx: &egui::Context, editor: &mut EditorState, playhead: f32) {
    let Some(index) = editor.selected else { return };
    if index >= editor.doc.objects.len() {
        editor.select(None);
        return;
    }

    egui::SidePanel::right("editor_inspector")
        .resizable(true)
        .default_width(280.0)
        .width_range(200.0..=520.0)
        .show(ctx, |ui| {
            ui.add_space(6.0);
            ui.horizontal(|ui| {
                ui.heading(spec_type_name(&editor.doc.objects[index].object));
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    if ui
                        .button("Duplicate")
                        .on_hover_text("Copy this object's spec and initial properties (no tracks/bindings) — Ctrl+D")
                        .clicked()
                    {
                        let new_index = editor.duplicate_object(index);
                        editor.select(Some(new_index));
                    }
                });
            });

            // Id rename: commit on enter / focus loss.
            ui.horizontal(|ui| {
                ui.weak("id");
                let response = ui.text_edit_singleline(&mut editor.id_buffer);
                if response.lost_focus() {
                    let requested = editor.id_buffer.clone();
                    if !editor.rename_object(index, &requested) {
                        editor.id_buffer = editor.doc.objects[index].id.clone();
                    }
                }
            });

            // Non-animatable spec params with dedicated editors.
            let mut spec_changed = false;
            if let ObjectSpec::Text { content, .. } | ObjectSpec::VectorText { content, .. } = &mut editor.doc.objects[index].object {
                ui.horizontal(|ui| {
                    ui.weak("content");
                    spec_changed |= ui.text_edit_singleline(content).changed();
                });
            }
            if let ObjectSpec::Sprite { image, .. } = &mut editor.doc.objects[index].object {
                ui.horizontal(|ui| {
                    ui.weak("image")
                        .on_hover_text("Path to a PNG (relative to the app's working directory)");
                    spec_changed |= ui.text_edit_singleline(image).changed();
                });
                if !std::path::Path::new(image.as_str()).exists() {
                    ui.colored_label(ui.visuals().error_fg_color, "file not found — drawing a placeholder");
                }
            }
            if let ObjectSpec::LSystem { axiom, rules, colors, .. } = &mut editor.doc.objects[index].object {
                ui.horizontal(|ui| {
                    ui.weak("axiom").on_hover_text(LSYSTEM_ALPHABET_HELP);
                    spec_changed |= ui.text_edit_singleline(axiom).changed();
                });
                ui.weak("rules").on_hover_text(LSYSTEM_ALPHABET_HELP);
                let mut remove_rule: Option<usize> = None;
                for (i, (from, to)) in rules.iter_mut().enumerate() {
                    ui.horizontal(|ui| {
                        spec_changed |= ui.add(egui::TextEdit::singleline(from).char_limit(1).desired_width(20.0)).changed();
                        ui.weak("->");
                        spec_changed |= ui.add(egui::TextEdit::singleline(to).desired_width(140.0)).changed();
                        if ui.small_button("x").on_hover_text("Remove rule").clicked() {
                            remove_rule = Some(i);
                        }
                    });
                }
                if let Some(i) = remove_rule {
                    rules.remove(i);
                    spec_changed = true;
                }
                if ui.small_button("+ rule").clicked() {
                    rules.push(("X".to_string(), String::new()));
                    spec_changed = true;
                }

                ui.horizontal(|ui| {
                    ui.weak("gradient")
                        .on_hover_text("2+ colors: segments blend through these in draw order; otherwise the color property is used");
                    let mut remove_color: Option<usize> = None;
                    for (i, c) in colors.iter_mut().enumerate() {
                        let mut rgba = [c.x, c.y, c.z, c.w];
                        if ui.color_edit_button_rgba_unmultiplied(&mut rgba).changed() {
                            *c = vec4(rgba[0], rgba[1], rgba[2], rgba[3]);
                            spec_changed = true;
                        }
                        if ui.small_button("x").clicked() {
                            remove_color = Some(i);
                        }
                    }
                    if let Some(i) = remove_color {
                        colors.remove(i);
                        spec_changed = true;
                    }
                    if ui.small_button("+").on_hover_text("Add gradient color").clicked() {
                        colors.push(vec4(1.0, 1.0, 1.0, 1.0));
                        spec_changed = true;
                    }
                });
            }
            if let ObjectSpec::Plot { expression, samples, .. } = &mut editor.doc.objects[index].object {
                ui.horizontal(|ui| {
                    ui.weak("f(x)");
                    spec_changed |= ui.text_edit_singleline(expression).changed();
                });
                if let Err(e) = crate::scene::expr::parse(expression) {
                    ui.colored_label(ui.visuals().error_fg_color, format!("axes only — {e}"));
                }
                ui.horizontal(|ui| {
                    ui.weak("samples");
                    spec_changed |= ui.add(egui::DragValue::new(samples).range(8..=4000)).changed();
                });
            }
            if spec_changed {
                editor.touch();
            }

            ui.separator();
            ui.weak("Initial properties (saved as overrides)");
            ui.add_space(4.0);

            // Generated editors over the Animatable surface. A bound
            // property shows its binding rows (link/offset/window) instead
            // of — or, when all bindings are windowed, in addition to — a
            // value widget.
            let object_id = editor.doc.objects[index].id.clone();
            let properties = editor.property_names(index);
            egui::ScrollArea::vertical().show(ui, |ui| {
                egui::Grid::new("editor_props").num_columns(2).spacing([10.0, 6.0]).show(ui, |ui| {
                    // Narrow panel: slim the drag boxes so vector rows keep
                    // all components visible instead of clipping.
                    let drag_width = ((ui.available_width() - 100.0) / 3.0 - 8.0).clamp(26.0, 40.0);
                    ui.spacing_mut().interact_size.x = drag_width;
                    for property in &properties {
                        let Some(value) = editor.effective_value(index, property) else {
                            continue;
                        };
                        ui.weak(property);
                        if let Some(new_value) = property_rows(ui, editor, &object_id, property, &value, playhead) {
                            editor.upsert_override(index, property, new_value);
                        }
                    }
                });
                ui.add_space(6.0);
                bind_menu_for(ui, editor, &object_id, playhead);
            });
        });
}

/// The camera's inspector page: same generated property widgets and bind
/// menu as objects, writing to `CameraDoc` (typed fields + `set` overrides).
fn camera_inspector(ctx: &egui::Context, editor: &mut EditorState, playhead: f32) {
    egui::SidePanel::right("editor_inspector")
        .resizable(true)
        .default_width(280.0)
        .width_range(200.0..=520.0)
        .show(ctx, |ui| {
            ui.add_space(6.0);
            ui.heading("Camera");
            ui.weak("Timeline camera. Toggle \"Camera follows timeline\" (F5) to preview tracks and bindings; orbit ignores them.");

            ui.separator();
            ui.weak("Initial properties");
            ui.add_space(4.0);

            let properties: Vec<String> = Camera::default().property_names().iter().map(|s| s.to_string()).collect();
            egui::ScrollArea::vertical().show(ui, |ui| {
                egui::Grid::new("editor_camera_props")
                    .num_columns(2)
                    .spacing([10.0, 6.0])
                    .show(ui, |ui| {
                        // Narrow panel: slim the drag boxes so vector rows keep
                        // all components visible instead of clipping.
                        let drag_width = ((ui.available_width() - 100.0) / 3.0 - 8.0).clamp(26.0, 40.0);
                        ui.spacing_mut().interact_size.x = drag_width;
                        for property in &properties {
                            let Some(value) = editor.camera_effective_value(property) else {
                                continue;
                            };
                            ui.weak(property);
                            if let Some(new_value) = property_rows(ui, editor, "camera", property, &value, playhead) {
                                editor.upsert_camera_override(property, new_value);
                            }
                        }
                    });
                ui.add_space(6.0);
                bind_menu_for(ui, editor, "camera", playhead);
            });
        });
}

/// Paint the second grid column (and any extra rows) for one property of
/// `target_id`: every binding on the property gets link/offset/window rows;
/// a value editor follows unless an unwindowed binding owns the property
/// outright. Returns the edited initial value, if any. Callers paint the
/// property-name cell first.
fn property_rows(
    ui: &mut egui::Ui,
    editor: &mut EditorState,
    target_id: &str,
    property: &str,
    value: &AnimValue,
    playhead: f32,
) -> Option<AnimValue> {
    let binding_indices = editor.bindings_for(target_id, property);
    let mut fully_owned = false;
    for (n, &binding_index) in binding_indices.iter().enumerate() {
        // A removal earlier in this frame may have shifted indices; skip the
        // rest of the frame's rows rather than paint the wrong binding.
        let Some(binding) = editor.doc.bindings.get(binding_index).cloned() else {
            continue;
        };
        if n > 0 {
            ui.weak("");
        }
        fully_owned |= !binding.is_windowed();
        ui.horizontal(|ui| {
            ui.label(format!("<- {}.{}", binding.source, binding.source_property));
            if ui.small_button("x").on_hover_text("Remove binding").clicked() {
                editor.remove_binding(binding_index);
            }
        });
        ui.end_row();
        if value.supports_offset() {
            ui.weak("    offset");
            let mut offset = binding.offset.clone().unwrap_or_else(|| zero_offset(value));
            if anim_value_edit(ui, "offset", &mut offset) {
                let _ = editor.set_binding_offset(binding_index, Some(offset));
            }
            ui.end_row();
        }
        ui.weak("    active");
        window_widgets(ui, editor, binding_index, &binding, playhead);
        ui.end_row();
    }
    if fully_owned {
        return None;
    }

    let mut value = value.clone();
    if !binding_indices.is_empty() {
        // All bindings are windowed: the initial value still applies
        // outside their windows.
        ui.weak("    value");
    }
    let edited = anim_value_edit(ui, property, &mut value);
    ui.end_row();
    edited.then_some(value)
}

/// One row of window controls for a binding: from/until checkboxes with
/// second drags. Unchecked = open on that side.
fn window_widgets(ui: &mut egui::Ui, editor: &mut EditorState, binding_index: usize, binding: &BindingDoc, playhead: f32) {
    ui.horizontal(|ui| {
        let mut has_start = binding.start.is_some();
        let mut start = binding.start.unwrap_or(0.0);
        let mut has_end = binding.end.is_some();
        let mut end = binding.end.unwrap_or(if playhead > start { playhead } else { start + 1.0 });
        let mut changed = false;

        changed |= ui
            .checkbox(&mut has_start, "from")
            .on_hover_text("Inactive before this time")
            .changed();
        changed |= ui
            .add_enabled(has_start, egui::DragValue::new(&mut start).speed(0.05).suffix("s"))
            .changed();
        changed |= ui
            .checkbox(&mut has_end, "until")
            .on_hover_text("Inactive from this time on")
            .changed();
        changed |= ui
            .add_enabled(has_end, egui::DragValue::new(&mut end).speed(0.05).suffix("s"))
            .changed();

        if changed && let Err(e) = editor.set_binding_window(binding_index, has_start.then_some(start), has_end.then_some(end)) {
            editor.error = Some(e);
        }
    });
}

/// A zero-valued offset matching the property's variant (offsetable
/// variants only; others are returned unchanged and never shown).
fn zero_offset(template: &AnimValue) -> AnimValue {
    match template {
        AnimValue::Float(_) => AnimValue::Float(0.0),
        AnimValue::Vec2(_) => AnimValue::Vec2(Vec2::ZERO),
        AnimValue::Vec3(_) => AnimValue::Vec3(Vec3::ZERO),
        AnimValue::Vec4(_) => AnimValue::Vec4(Vec4::ZERO),
        other => other.clone(),
    }
}

/// Three-level "+ Bind property" menu for `target_id` (an object id or
/// "camera"): property -> source object -> source property. Sources are
/// limited to type-matched, cycle-free choices. Properties owned by an
/// unwindowed binding are disabled; tracked or windowed-bound properties
/// stay available and create a binding active from the playhead onward.
fn bind_menu_for(ui: &mut egui::Ui, editor: &mut EditorState, target_id: &str, playhead: f32) {
    let mut sources: Vec<String> = editor
        .doc
        .objects
        .iter()
        .map(|o| o.id.clone())
        .filter(|id| id != target_id)
        .collect();
    if target_id != "camera" {
        sources.push("camera".to_string());
    }

    ui.menu_button("+ Bind property", |ui| {
        for property in editor.target_property_names(target_id).unwrap_or_default() {
            let existing = editor.bindings_for(target_id, &property);
            let always_bound = existing.iter().any(|&i| !editor.doc.bindings[i].is_windowed());
            if always_bound {
                ui.add_enabled(false, egui::Button::new(&property))
                    .on_disabled_hover_text("already bound at every time — window or remove that binding first");
                continue;
            }
            let tracked = editor.doc.tracks.iter().any(|t| t.object == target_id && t.property == property);
            let needs_window = tracked || !existing.is_empty();
            let Some(expected) = editor.target_value_template(target_id, &property) else {
                continue;
            };
            let response = ui.menu_button(&property, |ui| {
                for source in &sources {
                    if editor.binding_would_cycle(target_id, source) {
                        ui.add_enabled(false, egui::Button::new(source))
                            .on_disabled_hover_text("would create a binding cycle");
                        continue;
                    }
                    ui.menu_button(source, |ui| {
                        for source_property in editor.source_property_names(source).unwrap_or_default() {
                            let matches = editor
                                .target_value_template(source, &source_property)
                                .is_some_and(|v| std::mem::discriminant(&v) == std::mem::discriminant(&expected));
                            if !matches {
                                continue;
                            }
                            if ui.button(&source_property).clicked() {
                                let result = if needs_window {
                                    editor.add_binding_windowed(target_id, &property, source, &source_property, Some(playhead), None)
                                } else {
                                    editor.add_binding(target_id, &property, source, &source_property)
                                };
                                if let Err(e) = result {
                                    editor.error = Some(e);
                                }
                                ui.close_menu();
                            }
                        }
                    });
                }
            });
            if needs_window {
                response
                    .response
                    .on_hover_text("already tracked or window-bound — the new binding will be active from the playhead onward");
            }
        }
    });
}

fn drag(v: &mut f32) -> egui::DragValue<'_> {
    egui::DragValue::new(v).speed(0.05)
}

// ---------------------------------------------------------------------------
// Dope sheet
// ---------------------------------------------------------------------------

const SHEET_LABEL_W: f32 = 190.0;
const SHEET_ROW_H: f32 = 20.0;
const SHEET_RULER_H: f32 = 18.0;
const KEYFRAME_RADIUS: f32 = 5.0;

enum SheetAction {
    Select(usize, usize),
    Move(usize, usize, f32),
    AddKeyframe(usize, f32),
    RemoveTrack(usize),
    RemoveBinding(usize),
}

/// Human label for an easing (named variants read as their name).
pub fn easing_label(easing: Easing) -> String {
    match easing {
        Easing::Custom(_) => "Custom".to_string(),
        Easing::CubicBezier { .. } => "CubicBezier".to_string(),
        named => format!("{named:?}"),
    }
}

/// The dope sheet: one lane per track, keyframe diamonds, playhead, and a
/// detail strip for the selected keyframe. Rendered inside the transport
/// panel when a document is open.
pub fn dope_sheet(ui: &mut egui::Ui, editor: &mut EditorState, clock: &mut Clock, transport: &mut crate::ui::TransportState) {
    ui.separator();

    let display_duration = (clock.duration * 1.05).max(1.0);
    let mut action: Option<SheetAction> = None;

    // Header: "+ Track" menu in the label column, time ruler beside it.
    // The menu draws first, then a spacer pads the row out to exactly
    // SHEET_LABEL_W so the ruler's x-axis aligns with the lanes below.
    ui.horizontal(|ui| {
        let row_start = ui.cursor().min.x;
        add_track_menu(ui, editor);
        let used = ui.cursor().min.x - row_start;
        if used < SHEET_LABEL_W {
            ui.add_space(SHEET_LABEL_W - used);
        }

        let (ruler_rect, ruler_resp) =
            ui.allocate_exact_size(egui::vec2(ui.available_width(), SHEET_RULER_H), egui::Sense::click_and_drag());
        paint_ruler(ui, ruler_rect, display_duration);
        paint_playhead(ui, ruler_rect, clock.current_time, display_duration);

        // Click/drag the ruler to scrub (same pause/resume semantics as the slider).
        let scrub_to = ruler_resp
            .interact_pointer_pos()
            .filter(|_| ruler_resp.dragged() || ruler_resp.clicked())
            .map(|pos| x_to_time(pos.x, ruler_rect, display_duration));
        crate::ui::apply_scrub(transport, clock, ruler_resp.drag_started(), scrub_to, ruler_resp.drag_stopped());
    });

    // Track lanes.
    // Fill whatever height the (resizable) transport panel provides,
    // reserving room for the detail strip below when a keyframe is selected.
    let detail_reserve = if editor.selected_keyframe.is_some() { 84.0 } else { 8.0 };
    let lanes_height = (ui.available_height() - detail_reserve).max(2.0 * SHEET_ROW_H);
    // auto_shrink(false): claim the full lane height even when there are
    // few lanes — the panel persists its *content* height, so a shrinking
    // scroll area would snap a resize right back (egui panel.rs stores
    // inner_response.response.rect).
    egui::ScrollArea::vertical()
        .max_height(lanes_height)
        .auto_shrink([false, false])
        .show(ui, |ui| {
            for track_index in 0..editor.doc.tracks.len() {
                let label = format!(
                    "{}.{}",
                    editor.doc.tracks[track_index].object, editor.doc.tracks[track_index].property
                );
                ui.horizontal(|ui| {
                    let (label_rect, _) = ui.allocate_exact_size(egui::vec2(SHEET_LABEL_W, SHEET_ROW_H), egui::Sense::hover());
                    {
                        let text_area =
                            egui::Rect::from_min_max(label_rect.min, egui::pos2(label_rect.right() - 40.0, label_rect.bottom()));
                        let clip = ui.painter().with_clip_rect(text_area);
                        clip.text(
                            label_rect.left_center() + egui::vec2(4.0, 0.0),
                            egui::Align2::LEFT_CENTER,
                            &label,
                            egui::FontId::proportional(12.0),
                            ui.visuals().weak_text_color(),
                        );
                    }
                    let add_rect = egui::Rect::from_min_size(label_rect.right_top() - egui::vec2(38.0, 0.0), egui::vec2(16.0, SHEET_ROW_H));
                    if ui
                        .put(add_rect, egui::Button::new("+").small())
                        .on_hover_text("Add keyframe at the playhead (or double-click the lane at any time)")
                        .clicked()
                    {
                        action = Some(SheetAction::AddKeyframe(track_index, clock.current_time));
                    }
                    let delete_rect =
                        egui::Rect::from_min_size(label_rect.right_top() - egui::vec2(18.0, 0.0), egui::vec2(16.0, SHEET_ROW_H));
                    if ui
                        .put(delete_rect, egui::Button::new("x").small())
                        .on_hover_text("Delete track")
                        .clicked()
                    {
                        action = Some(SheetAction::RemoveTrack(track_index));
                    }

                    let (lane_rect, lane_resp) =
                        ui.allocate_exact_size(egui::vec2(ui.available_width(), SHEET_ROW_H), egui::Sense::click());
                    let lane_resp = lane_resp
                        .on_hover_text("Double-click: add keyframe · drag a diamond to retime · a track needs 2+ keyframes to animate");
                    if track_index % 2 == 0 {
                        ui.painter().rect_filled(lane_rect, 0.0, ui.visuals().faint_bg_color);
                    }
                    if lane_resp.double_clicked()
                        && let Some(pos) = lane_resp.interact_pointer_pos()
                    {
                        action = Some(SheetAction::AddKeyframe(track_index, x_to_time(pos.x, lane_rect, display_duration)));
                    }

                    for kf_index in 0..editor.doc.tracks[track_index].keyframes.len() {
                        let kf_time = editor.doc.tracks[track_index].keyframes[kf_index].time;
                        let x = time_to_x(kf_time, lane_rect, display_duration);
                        let center = egui::pos2(x, lane_rect.center().y);
                        let hit = egui::Rect::from_center_size(center, egui::vec2(12.0, SHEET_ROW_H));
                        let id = ui.id().with(("kf", track_index, kf_index));
                        let resp = ui.interact(hit, id, egui::Sense::click_and_drag());

                        if resp.clicked() || resp.drag_started() {
                            action = Some(SheetAction::Select(track_index, kf_index));
                        }
                        if resp.dragged() && resp.drag_delta().x != 0.0 {
                            let dt = resp.drag_delta().x / lane_rect.width() * display_duration;
                            action = Some(SheetAction::Move(
                                track_index,
                                kf_index,
                                (kf_time + dt).clamp(0.0, display_duration),
                            ));
                        }

                        let selected = editor.selected_keyframe == Some((track_index, kf_index));
                        let fill = if selected {
                            ui.visuals().selection.bg_fill
                        } else {
                            egui::Color32::from_gray(200)
                        };
                        paint_diamond(ui.painter(), center, KEYFRAME_RADIUS, fill);
                    }

                    paint_playhead(ui, lane_rect, clock.current_time, display_duration);
                });
            }

            // Binding lanes: no keyframes — a flat bar spanning the binding's
            // active window (full width when unwindowed).
            let track_count = editor.doc.tracks.len();
            for binding_index in 0..editor.doc.bindings.len() {
                let binding = &editor.doc.bindings[binding_index];
                let label = format!("{}.{}", binding.target, binding.property);
                let source_label = format!("<- {}.{}", binding.source, binding.source_property);
                let (window_start, window_end) = (binding.start, binding.end);
                ui.horizontal(|ui| {
                    let (label_rect, _) = ui.allocate_exact_size(egui::vec2(SHEET_LABEL_W, SHEET_ROW_H), egui::Sense::hover());
                    {
                        let text_area =
                            egui::Rect::from_min_max(label_rect.min, egui::pos2(label_rect.right() - 20.0, label_rect.bottom()));
                        let clip = ui.painter().with_clip_rect(text_area);
                        clip.text(
                            label_rect.left_center() + egui::vec2(4.0, 0.0),
                            egui::Align2::LEFT_CENTER,
                            &label,
                            egui::FontId::proportional(12.0),
                            ui.visuals().weak_text_color(),
                        );
                    }
                    let delete_rect =
                        egui::Rect::from_min_size(label_rect.right_top() - egui::vec2(18.0, 0.0), egui::vec2(16.0, SHEET_ROW_H));
                    if ui
                        .put(delete_rect, egui::Button::new("x").small())
                        .on_hover_text("Remove binding")
                        .clicked()
                    {
                        action = Some(SheetAction::RemoveBinding(binding_index));
                    }

                    let (lane_rect, _) = ui.allocate_exact_size(egui::vec2(ui.available_width(), SHEET_ROW_H), egui::Sense::hover());
                    if (track_count + binding_index).is_multiple_of(2) {
                        ui.painter().rect_filled(lane_rect, 0.0, ui.visuals().faint_bg_color);
                    }
                    let bar_left = match window_start {
                        Some(s) => time_to_x(s, lane_rect, display_duration),
                        None => lane_rect.left() + 4.0,
                    };
                    let bar_right = match window_end {
                        Some(e) => time_to_x(e, lane_rect, display_duration),
                        None => lane_rect.right() - 4.0,
                    };
                    let bar = egui::Rect::from_min_max(
                        egui::pos2(bar_left.max(lane_rect.left() + 4.0), lane_rect.bottom() - 6.0),
                        egui::pos2(bar_right.min(lane_rect.right() - 4.0), lane_rect.bottom() - 3.0),
                    );
                    ui.painter().rect_filled(bar, 1.5, egui::Color32::from_gray(120));
                    ui.painter().text(
                        lane_rect.left_center() + egui::vec2(8.0, -2.0),
                        egui::Align2::LEFT_CENTER,
                        &source_label,
                        egui::FontId::proportional(11.0),
                        ui.visuals().weak_text_color(),
                    );
                    paint_playhead(ui, lane_rect, clock.current_time, display_duration);
                });
            }
        });

    match action {
        Some(SheetAction::Select(t, k)) => editor.selected_keyframe = Some((t, k)),
        Some(SheetAction::Move(t, k, time)) => {
            editor.selected_keyframe = Some((t, k));
            editor.move_keyframe(t, k, time);
        }
        Some(SheetAction::AddKeyframe(t, time)) => {
            if let Some(k) = editor.add_keyframe(t, time) {
                editor.selected_keyframe = Some((t, k));
            }
        }
        Some(SheetAction::RemoveTrack(t)) => editor.remove_track(t),
        Some(SheetAction::RemoveBinding(b)) => editor.remove_binding(b),
        None => {}
    }

    // Delete key removes the selected keyframe (unless a widget has focus).
    let delete_pressed = ui.ctx().input(|i| i.key_pressed(egui::Key::Delete)) && ui.ctx().memory(|m| m.focused().is_none());
    if delete_pressed && let Some((t, k)) = editor.selected_keyframe {
        editor.remove_keyframe(t, k);
    }

    keyframe_detail_strip(ui, editor);

    // Pin the content's bottom to exactly the panel's inner bottom. The
    // panel persists its *content* height every frame, so any constant gap
    // (reserve over-estimate) or overshoot (allocate_space adds item
    // spacing) feeds back and makes a resized panel creep down or up a few
    // pixels per frame until it hits a range limit. An exact allocate_rect
    // is the fixed point. Pinned by ui::tests::editor_transport_panel_holds_resized_height.
    let bottom = ui.max_rect().bottom();
    let cursor = ui.next_widget_position().y;
    if bottom > cursor {
        ui.allocate_rect(
            egui::Rect::from_min_max(egui::pos2(ui.max_rect().left(), cursor), egui::pos2(ui.max_rect().right(), bottom)),
            egui::Sense::hover(),
        );
    }
}

fn add_track_menu(ui: &mut egui::Ui, editor: &mut EditorState) {
    let mut targets: Vec<String> = editor.doc.objects.iter().map(|o| o.id.clone()).collect();
    targets.push("camera".to_string());

    ui.menu_button("+ Track", |ui| {
        for target in &targets {
            ui.menu_button(target, |ui| {
                for property in editor.target_property_names(target).unwrap_or_default() {
                    let exists = editor.doc.tracks.iter().any(|t| &t.object == target && t.property == property);
                    let bound = editor.binding_for(target, &property).is_some();
                    let response = ui.add_enabled(!exists && !bound, egui::Button::new(&property));
                    if bound {
                        response.on_disabled_hover_text("bound — a property is tracked or bound, never both");
                    } else if response.clicked() {
                        let _ = editor.add_track(target, &property);
                        ui.close_menu();
                    }
                }
            });
        }
    });
}

fn keyframe_detail_strip(ui: &mut egui::Ui, editor: &mut EditorState) {
    let Some((track_index, kf_index)) = editor.selected_keyframe else {
        return;
    };
    if track_index >= editor.doc.tracks.len() || kf_index >= editor.doc.tracks[track_index].keyframes.len() {
        editor.selected_keyframe = None;
        return;
    }

    ui.separator();
    let label = format!(
        "{}.{}",
        editor.doc.tracks[track_index].object, editor.doc.tracks[track_index].property
    );
    let kf = editor.doc.tracks[track_index].keyframes[kf_index].clone();

    ui.horizontal(|ui| {
        ui.strong(label);

        let mut time = kf.time;
        ui.weak("t");
        if ui
            .add(egui::DragValue::new(&mut time).speed(0.02).range(0.0..=f32::MAX).suffix(" s"))
            .changed()
        {
            editor.move_keyframe(track_index, kf_index, time);
        }

        // Easing shapes the segment *into* this keyframe; the track's
        // earliest keyframe has no incoming segment, so no picker.
        let earliest = editor.doc.tracks[track_index]
            .keyframes
            .iter()
            .enumerate()
            .min_by(|(_, a), (_, b)| a.time.total_cmp(&b.time))
            .map(|(i, _)| i);
        if earliest == Some(kf_index) {
            ui.weak("(first keyframe — nothing to ease in from)");
        } else {
            ui.weak("ease in");
            let mut easing = kf.easing;
            egui::ComboBox::from_id_salt("kf_easing")
                .selected_text(easing_label(easing))
                .width(110.0)
                .show_ui(ui, |ui| {
                    for candidate in Easing::NAMED {
                        if ui.selectable_label(easing == candidate, easing_label(candidate)).clicked() {
                            easing = candidate;
                        }
                    }
                    let is_bezier = matches!(easing, Easing::CubicBezier { .. });
                    if ui.selectable_label(is_bezier, "CubicBezier").clicked() && !is_bezier {
                        easing = Easing::DEFAULT_BEZIER;
                    }
                });
            if easing != kf.easing {
                editor.set_keyframe_easing(track_index, kf_index, easing);
            }

            let mut steps = kf.steps.unwrap_or(1);
            ui.weak("steps");
            if ui
                .add(egui::DragValue::new(&mut steps).speed(0.2).range(1..=100_000))
                .on_hover_text("Arrive in N equal sub-steps, each eased by this curve — e.g. steps = segment count reveals an L-system one eased segment at a time")
                .changed()
            {
                editor.set_keyframe_steps(track_index, kf_index, steps);
            }

            curve_widget(ui, editor, track_index, kf_index);
        }

        let mut value = kf.value.clone();
        if anim_value_edit(ui, &editor.doc.tracks[track_index].property.clone(), &mut value) {
            editor.set_keyframe_value(track_index, kf_index, value);
        }

        if ui.button("Delete").on_hover_text("Delete keyframe (Del)").clicked() {
            editor.remove_keyframe(track_index, kf_index);
        }
    });
}

/// Curve preview for the selected keyframe's incoming easing. Read-only for
/// named easings; CubicBezier gets two draggable control-point handles.
fn curve_widget(ui: &mut egui::Ui, editor: &mut EditorState, track_index: usize, kf_index: usize) {
    const W: f32 = 96.0;
    const H: f32 = 56.0;
    // Value range shown: -0.5..1.5 so overshoot easings stay visible.
    let (rect, resp) = ui.allocate_exact_size(egui::vec2(W, H), egui::Sense::click_and_drag());
    let painter = ui.painter();
    painter.rect_filled(rect, 2.0, ui.visuals().extreme_bg_color);

    let to_px = |t: f32, v: f32| egui::pos2(rect.left() + t * rect.width(), rect.bottom() - (v + 0.5) / 2.0 * rect.height());
    let weak = egui::Stroke::new(1.0, ui.visuals().weak_text_color().linear_multiply(0.4));
    painter.hline(rect.x_range(), to_px(0.0, 0.0).y, weak);
    painter.hline(rect.x_range(), to_px(0.0, 1.0).y, weak);

    // Plot the effective interpolation, staircase included when stepped.
    let easing = editor.doc.tracks[track_index].keyframes[kf_index].easing;
    let steps = editor.doc.tracks[track_index].keyframes[kf_index].steps.unwrap_or(1);
    let eval = |t: f32| -> f32 {
        if steps > 1 {
            let n = steps as f32;
            let x = (t * n).clamp(0.0, n);
            let i = x.floor().min(n - 1.0);
            (i + easing.eval(x - i)) / n
        } else {
            easing.eval(t)
        }
    };
    let samples = if steps > 1 { 128 } else { 32 };
    let points: Vec<egui::Pos2> = (0..=samples)
        .map(|i| {
            let t = i as f32 / samples as f32;
            to_px(t, eval(t).clamp(-0.5, 1.5))
        })
        .collect();
    painter.add(egui::Shape::line(points, egui::Stroke::new(1.5, ui.visuals().selection.bg_fill)));

    if let Easing::CubicBezier { x1, y1, x2, y2 } = easing {
        let h0 = to_px(x1, y1);
        let h1 = to_px(x2, y2);
        let handle_stroke = egui::Stroke::new(1.0, egui::Color32::from_gray(140));
        painter.line_segment([to_px(0.0, 0.0), h0], handle_stroke);
        painter.line_segment([to_px(1.0, 1.0), h1], handle_stroke);
        painter.circle_filled(h0, 3.5, egui::Color32::LIGHT_RED);
        painter.circle_filled(h1, 3.5, egui::Color32::LIGHT_GREEN);

        if resp.drag_started()
            && let Some(pos) = resp.interact_pointer_pos()
        {
            editor.bezier_drag = Some(if pos.distance(h0) <= pos.distance(h1) { 0 } else { 1 });
        }
        if resp.dragged()
            && let (Some(handle), Some(pos)) = (editor.bezier_drag, resp.interact_pointer_pos())
        {
            let t = ((pos.x - rect.left()) / rect.width()).clamp(0.0, 1.0);
            let v = ((rect.bottom() - pos.y) / rect.height() * 2.0 - 0.5).clamp(-0.5, 1.5);
            let new_easing = if handle == 0 {
                Easing::CubicBezier { x1: t, y1: v, x2, y2 }
            } else {
                Easing::CubicBezier { x1, y1, x2: t, y2: v }
            };
            editor.set_keyframe_easing(track_index, kf_index, new_easing);
        }
        if resp.drag_stopped() {
            editor.bezier_drag = None;
        }
    } else {
        resp.on_hover_text("Curve preview — pick CubicBezier in the easing list for editable handles");
    }
}

fn time_to_x(time: f32, rect: egui::Rect, duration: f32) -> f32 {
    rect.left() + (time / duration).clamp(0.0, 1.0) * rect.width()
}

fn x_to_time(x: f32, rect: egui::Rect, duration: f32) -> f32 {
    ((x - rect.left()) / rect.width()).clamp(0.0, 1.0) * duration
}

/// Pick a tick step that yields a readable number of ruler labels.
fn ruler_step(duration: f32) -> f32 {
    for step in [0.25, 0.5, 1.0, 2.0, 5.0, 10.0, 30.0, 60.0] {
        if duration / step <= 10.0 {
            return step;
        }
    }
    120.0
}

fn paint_ruler(ui: &egui::Ui, rect: egui::Rect, duration: f32) {
    let painter = ui.painter();
    painter.rect_filled(rect, 0.0, ui.visuals().extreme_bg_color);
    let step = ruler_step(duration);
    let mut t = 0.0;
    while t <= duration {
        let x = time_to_x(t, rect, duration);
        painter.vline(
            x,
            egui::Rangef::new(rect.bottom() - 5.0, rect.bottom()),
            egui::Stroke::new(1.0, ui.visuals().weak_text_color()),
        );
        painter.text(
            egui::pos2(x + 3.0, rect.top()),
            egui::Align2::LEFT_TOP,
            format!("{t:.2}"),
            egui::FontId::proportional(10.0),
            ui.visuals().weak_text_color(),
        );
        t += step;
    }
}

fn paint_playhead(ui: &egui::Ui, rect: egui::Rect, time: f32, duration: f32) {
    let x = time_to_x(time, rect, duration);
    ui.painter()
        .vline(x, rect.y_range(), egui::Stroke::new(1.5, egui::Color32::from_rgb(255, 96, 96)));
}

fn paint_diamond(painter: &egui::Painter, center: egui::Pos2, r: f32, fill: egui::Color32) {
    let points = vec![
        center + egui::vec2(0.0, -r),
        center + egui::vec2(r, 0.0),
        center + egui::vec2(0.0, r),
        center + egui::vec2(-r, 0.0),
    ];
    painter.add(egui::Shape::convex_polygon(
        points,
        fill,
        egui::Stroke::new(1.0, egui::Color32::from_gray(60)),
    ));
}

/// A typed editor widget for an `AnimValue`. Returns true when changed.
fn anim_value_edit(ui: &mut egui::Ui, property: &str, value: &mut AnimValue) -> bool {
    match value {
        AnimValue::Float(f) => ui.add(drag(f)).changed(),
        AnimValue::Vec2(v) => {
            let mut changed = false;
            ui.horizontal(|ui| {
                changed |= ui.add(drag(&mut v.x)).changed();
                changed |= ui.add(drag(&mut v.y)).changed();
            });
            changed
        }
        AnimValue::Vec3(v) => {
            let mut changed = false;
            ui.horizontal(|ui| {
                changed |= ui.add(drag(&mut v.x)).changed();
                changed |= ui.add(drag(&mut v.y)).changed();
                changed |= ui.add(drag(&mut v.z)).changed();
            });
            changed
        }
        AnimValue::Vec4(v) if property.contains("color") => {
            let mut rgba = [v.x, v.y, v.z, v.w];
            let changed = ui.color_edit_button_rgba_unmultiplied(&mut rgba).changed();
            if changed {
                *v = vec4(rgba[0], rgba[1], rgba[2], rgba[3]);
            }
            changed
        }
        AnimValue::Vec4(v) => {
            let mut changed = false;
            ui.horizontal(|ui| {
                changed |= ui.add(drag(&mut v.x)).changed();
                changed |= ui.add(drag(&mut v.y)).changed();
                changed |= ui.add(drag(&mut v.z)).changed();
                changed |= ui.add(drag(&mut v.w)).changed();
            });
            changed
        }
        AnimValue::Bool(b) => ui.checkbox(b, "").changed(),
        AnimValue::Transform2D(t) => {
            let mut changed = false;
            ui.vertical(|ui| {
                ui.horizontal(|ui| {
                    changed |= ui.add(drag(&mut t.position.x)).changed();
                    changed |= ui.add(drag(&mut t.position.y)).changed();
                });
                changed |= ui.add(drag(&mut t.rotation)).changed();
            });
            changed
        }
        AnimValue::Mat4(_) => {
            ui.weak("matrix (not editable)");
            false
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::doc::{KeyframeDoc, TrackDoc};

    fn doc_with(objects: Vec<ObjectDoc>, tracks: Vec<TrackDoc>) -> SceneDoc {
        SceneDoc {
            description: String::new(),
            camera: Default::default(),
            export: Default::default(),
            objects,
            tracks,
            bindings: Vec::new(),
        }
    }

    fn disk(id: &str) -> ObjectDoc {
        ObjectDoc {
            id: id.to_string(),
            object: ObjectSpec::Disk {
                position: Vec3::ZERO,
                radius: 1.0,
                color: vec4(1.0, 1.0, 1.0, 1.0),
            },
            set: Vec::new(),
        }
    }

    fn track_for(id: &str) -> TrackDoc {
        TrackDoc {
            object: id.to_string(),
            property: "radius".to_string(),
            keyframes: vec![KeyframeDoc {
                time: 0.0,
                value: AnimValue::Float(1.0),
                easing: Default::default(),
                steps: None,
            }],
        }
    }

    fn editor(doc: SceneDoc) -> EditorState {
        EditorState::new(doc, "scenes/test.ron")
    }

    #[test]
    fn scene_stem_extracts_file_stem() {
        assert_eq!(scene_stem("scenes/foo.ron"), "foo");
        assert_eq!(scene_stem("foo.ron"), "foo");
        assert_eq!(scene_stem("a/b/c.ron"), "c");
        assert_eq!(scene_stem("scenes/noext"), "noext");
    }

    /// Create a real .ron file in a per-process temp dir and leak its path.
    fn temp_scene(name: &str) -> &'static str {
        let dir = std::env::temp_dir().join(format!("rs_namin_editor_tests_{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join(format!("{name}.ron"));
        std::fs::write(&path, "(objects: [], tracks: [])").unwrap();
        Box::leak(path.to_string_lossy().into_owned().into_boxed_str())
    }

    #[test]
    fn rename_scene_moves_the_file_and_updates_path() {
        let path = temp_scene("rename_me_src");
        // A stale target from an earlier failed run would block the rename.
        let target = std::path::Path::new(path).with_file_name("rename_me_dst.ron");
        let _ = std::fs::remove_file(&target);

        let mut ed = EditorState::new(doc_with(vec![], vec![]), path);
        assert_eq!(ed.name_buffer, "rename_me_src");
        ed.rename_scene("rename_me_dst").unwrap();
        assert!(ed.renamed);
        assert!(ed.path.ends_with("rename_me_dst.ron"));
        assert_eq!(ed.name_buffer, "rename_me_dst");
        assert!(!std::path::Path::new(path).exists());
        assert!(std::path::Path::new(ed.path).exists());

        // Renaming to the current name is a no-op.
        ed.renamed = false;
        ed.rename_scene("rename_me_dst").unwrap();
        assert!(!ed.renamed);

        // Saving writes to the new path.
        ed.dirty = true;
        ed.save().unwrap();
        assert!(!ed.dirty);
        std::fs::remove_file(ed.path).unwrap();
    }

    #[test]
    fn rename_scene_rejects_bad_names_and_collisions() {
        let path = temp_scene("rename_reject_src");
        let other = temp_scene("rename_reject_existing");
        let mut ed = EditorState::new(doc_with(vec![], vec![]), path);

        assert!(ed.rename_scene("").is_err());
        assert!(ed.rename_scene("  ").is_err());
        assert!(ed.rename_scene("a/b").is_err());
        assert!(ed.rename_scene("a\\b").is_err());
        assert!(ed.rename_scene(".hidden").is_err());
        assert!(ed.rename_scene("rename_reject_existing").is_err()); // file exists
        assert!(ed.rename_scene("turtle_intro").is_err()); // built-in name

        assert_eq!(scene_stem(ed.path), "rename_reject_src");
        assert!(!ed.renamed);
        std::fs::remove_file(path).unwrap();
        std::fs::remove_file(other).unwrap();
    }

    #[test]
    fn add_object_generates_unique_ids() {
        let mut ed = editor(doc_with(vec![disk("disk_1")], vec![]));
        let templates = palette_templates();
        let (name, spec) = templates[0].clone();
        let index = ed.add_object(name, spec);
        assert_eq!(ed.doc.objects[index].id, "disk_2");
        assert!(ed.dirty && ed.rebuild_needed);
    }

    #[test]
    fn remove_object_cascades_tracks_and_selection() {
        let mut ed = editor(doc_with(vec![disk("a"), disk("b")], vec![track_for("a"), track_for("b")]));
        ed.select(Some(1));
        ed.remove_object(0);
        assert_eq!(ed.doc.objects.len(), 1);
        assert_eq!(ed.doc.tracks.len(), 1);
        assert_eq!(ed.doc.tracks[0].object, "b");
        assert_eq!(ed.selected, Some(0));
    }

    #[test]
    fn rename_cascades_tracks() {
        let mut ed = editor(doc_with(vec![disk("a")], vec![track_for("a")]));
        assert!(ed.rename_object(0, "ball"));
        assert_eq!(ed.doc.objects[0].id, "ball");
        assert_eq!(ed.doc.tracks[0].object, "ball");
    }

    fn binding(target: &str, source: &str) -> crate::doc::BindingDoc {
        crate::doc::BindingDoc {
            target: target.to_string(),
            property: "radius".to_string(),
            source: source.to_string(),
            source_property: "radius".to_string(),
            offset: None,
            start: None,
            end: None,
        }
    }

    #[test]
    fn remove_object_cascades_bindings_on_both_ends() {
        let mut ed = editor(doc_with(vec![disk("a"), disk("b"), disk("c")], vec![]));
        ed.doc.bindings = vec![binding("a", "b"), binding("b", "c"), binding("c", "a")];
        ed.remove_object(0);
        // Only the binding not touching "a" survives.
        assert_eq!(ed.doc.bindings, vec![binding("b", "c")]);
        ed.doc.build().unwrap();
    }

    #[test]
    fn rename_cascades_binding_targets_and_sources() {
        let mut ed = editor(doc_with(vec![disk("a"), disk("b")], vec![]));
        ed.doc.bindings = vec![binding("a", "b")];
        assert!(ed.rename_object(0, "leader"));
        assert!(ed.rename_object(1, "follower"));
        assert_eq!(ed.doc.bindings, vec![binding("leader", "follower")]);
        ed.doc.build().unwrap();
    }

    #[test]
    fn add_binding_validates_and_rejects_cycles() {
        let mut ed = editor(doc_with(vec![disk("a"), disk("b")], vec![]));
        let b = ed.add_binding("a", "radius", "b", "radius").unwrap();
        assert_eq!(ed.doc.bindings[b], binding("a", "b"));

        // Reverse direction would cycle; doc must be left unchanged.
        assert!(ed.add_binding("b", "position", "a", "position").is_err());
        assert_eq!(ed.doc.bindings.len(), 1);

        // Duplicate target property, unknown source, type mismatch.
        assert!(ed.add_binding("a", "radius", "b", "radius").is_err());
        assert!(ed.add_binding("a", "position", "ghost", "position").is_err());
        assert!(ed.add_binding("a", "position", "b", "radius").is_err());
        assert_eq!(ed.doc.bindings.len(), 1);
    }

    #[test]
    fn add_binding_rejects_tracked_property() {
        let mut ed = editor(doc_with(vec![disk("a"), disk("b")], vec![track_for("a")]));
        let err = ed.add_binding("a", "radius", "b", "radius").unwrap_err();
        assert!(err.contains("keyframed and always-bound"), "unexpected error: {err}");
    }

    #[test]
    fn set_binding_offset_applies_and_reverts_invalid() {
        let mut ed = editor(doc_with(vec![disk("a"), disk("b")], vec![]));
        let b = ed.add_binding("a", "radius", "b", "radius").unwrap();
        ed.set_binding_offset(b, Some(AnimValue::Float(0.5))).unwrap();
        assert_eq!(ed.doc.bindings[b].offset, Some(AnimValue::Float(0.5)));

        assert!(ed.set_binding_offset(b, Some(AnimValue::Vec3(Vec3::ZERO))).is_err());
        assert_eq!(ed.doc.bindings[b].offset, Some(AnimValue::Float(0.5)));

        ed.set_binding_offset(b, None).unwrap();
        assert_eq!(ed.doc.bindings[b].offset, None);
    }

    #[test]
    fn binding_would_cycle_walks_chains() {
        let mut ed = editor(doc_with(vec![disk("a"), disk("b"), disk("c")], vec![]));
        ed.doc.bindings = vec![binding("b", "a"), binding("c", "b")];
        // c depends on b depends on a: binding a to c (or b) would cycle.
        assert!(ed.binding_would_cycle("a", "c"));
        assert!(ed.binding_would_cycle("a", "b"));
        assert!(ed.binding_would_cycle("a", "a"));
        // The other direction is fine, as is an uninvolved source.
        assert!(!ed.binding_would_cycle("c", "a"));
        assert!(!ed.binding_would_cycle("a", "camera"));
    }

    #[test]
    fn binding_for_finds_target_property() {
        let mut ed = editor(doc_with(vec![disk("a"), disk("b")], vec![]));
        ed.doc.bindings = vec![binding("a", "b")];
        assert_eq!(ed.binding_for("a", "radius"), Some(0));
        assert_eq!(ed.binding_for("a", "position"), None);
        assert_eq!(ed.binding_for("b", "radius"), None);
    }

    #[test]
    fn duplicate_copies_spec_and_overrides_but_not_tracks_or_bindings() {
        let mut ed = editor(doc_with(vec![disk("a"), disk("b")], vec![track_for("a")]));
        ed.doc.objects[0].set = vec![("radius".to_string(), AnimValue::Float(7.0))];
        ed.doc.bindings = vec![binding("b", "a")];

        let new_index = ed.duplicate_object(0);
        assert_eq!(ed.doc.objects.len(), 3);
        assert_eq!(ed.doc.objects[new_index].id, "a_copy");
        // Initial values carried over (radius override), position nudged.
        assert_eq!(ed.effective_value(new_index, "radius"), Some(AnimValue::Float(7.0)));
        assert_ne!(
            ed.effective_value(new_index, "position"),
            ed.effective_value(0, "position"),
            "copy should be nudged off the original"
        );
        // Nothing animated or bound references the copy.
        assert!(!ed.doc.tracks.iter().any(|t| t.object == "a_copy"));
        assert!(!ed.doc.bindings.iter().any(|b| b.target == "a_copy" || b.source == "a_copy"));
        ed.doc.build().unwrap();

        // Ids stay unique on repeat duplication.
        let again = ed.duplicate_object(0);
        assert_eq!(ed.doc.objects[again].id, "a_copy_2");
    }

    #[test]
    fn ctrl_d_duplicates_the_selected_object() {
        let mut ed = editor(doc_with(vec![disk("a")], vec![]));
        ed.select(Some(0));

        let ctx = egui::Context::default();
        let input = egui::RawInput {
            screen_rect: Some(egui::Rect::from_min_size(egui::pos2(0.0, 0.0), egui::vec2(1280.0, 720.0))),
            events: vec![egui::Event::Key {
                key: egui::Key::D,
                physical_key: None,
                pressed: true,
                repeat: false,
                modifiers: egui::Modifiers::COMMAND,
            }],
            ..Default::default()
        };
        ctx.run(input, |ctx| panels(ctx, &mut ed, 0.0));

        assert_eq!(ed.doc.objects.len(), 2);
        assert_eq!(ed.doc.objects[1].id, "a_copy");
        assert_eq!(ed.selected, Some(1), "the copy should be selected");
    }

    #[test]
    fn ctrl_s_saves_the_scene() {
        let path = temp_scene("ctrl_s_save");
        let mut ed = EditorState::new(doc_with(vec![disk("a")], vec![]), path);
        ed.doc.description = "saved by shortcut".to_string();
        ed.dirty = true;

        let ctx = egui::Context::default();
        let input = egui::RawInput {
            screen_rect: Some(egui::Rect::from_min_size(egui::pos2(0.0, 0.0), egui::vec2(1280.0, 720.0))),
            events: vec![egui::Event::Key {
                key: egui::Key::S,
                physical_key: None,
                pressed: true,
                repeat: false,
                modifiers: egui::Modifiers::COMMAND,
            }],
            ..Default::default()
        };
        ctx.run(input, |ctx| panels(ctx, &mut ed, 0.0));

        assert!(!ed.dirty, "Ctrl+S should have saved");
        let saved = std::fs::read_to_string(ed.path).unwrap();
        assert!(saved.contains("saved by shortcut"), "file should hold the edited doc");
        std::fs::remove_file(ed.path).unwrap();
    }

    #[test]
    fn set_keyframe_steps_normalizes_to_none_at_one() {
        let mut ed = editor(doc_with(vec![disk("a")], vec![track_for("a")]));
        ed.set_keyframe_steps(0, 0, 16);
        assert_eq!(ed.doc.tracks[0].keyframes[0].steps, Some(16));
        ed.set_keyframe_steps(0, 0, 1);
        assert_eq!(ed.doc.tracks[0].keyframes[0].steps, None);
        ed.set_keyframe_steps(0, 0, 0);
        assert_eq!(ed.doc.tracks[0].keyframes[0].steps, None);
        ed.doc.build().unwrap();
    }

    #[test]
    fn camera_selection_is_mutually_exclusive_with_objects() {
        let mut ed = editor(doc_with(vec![disk("a")], vec![]));
        assert!(!ed.camera_selected);
        ed.select_camera();
        assert!(ed.camera_selected);
        assert_eq!(ed.selected, None);
        ed.select(Some(0));
        assert!(!ed.camera_selected);
        assert_eq!(ed.selected, Some(0));
    }

    #[test]
    fn camera_overrides_route_to_fields_and_set() {
        let mut ed = editor(doc_with(vec![], vec![]));
        // Typed fields.
        ed.upsert_camera_override("position", AnimValue::Vec3(vec3(1.0, 2.0, 3.0)));
        ed.upsert_camera_override("fov", AnimValue::Float(45.0));
        assert_eq!(ed.doc.camera.position, vec3(1.0, 2.0, 3.0));
        assert_eq!(ed.doc.camera.fov, 45.0);
        assert!(ed.doc.camera.set.is_empty());
        // Other properties become set overrides.
        ed.upsert_camera_override("rotation_z", AnimValue::Float(0.7));
        ed.upsert_camera_override("rotation_z", AnimValue::Float(0.9));
        assert_eq!(ed.doc.camera.set, vec![("rotation_z".to_string(), AnimValue::Float(0.9))]);
        // Effective values read through both.
        assert_eq!(ed.camera_effective_value("fov"), Some(AnimValue::Float(45.0)));
        assert_eq!(ed.camera_effective_value("rotation_z"), Some(AnimValue::Float(0.9)));
        // A hand-written set entry shadowing a typed field is dropped when
        // the field is edited.
        ed.doc.camera.set.push(("fov".to_string(), AnimValue::Float(20.0)));
        assert_eq!(ed.camera_effective_value("fov"), Some(AnimValue::Float(20.0)));
        ed.upsert_camera_override("fov", AnimValue::Float(50.0));
        assert_eq!(ed.camera_effective_value("fov"), Some(AnimValue::Float(50.0)));
        ed.doc.build().unwrap();
    }

    #[test]
    fn camera_can_be_bound_through_the_editor() {
        let mut ed = editor(doc_with(vec![disk("a")], vec![]));
        let b = ed.add_binding("camera", "position", "a", "position").unwrap();
        assert_eq!(ed.doc.bindings[b].target, "camera");
        ed.doc.build().unwrap();
    }

    #[test]
    fn windowed_binding_coexists_with_track_via_editor() {
        let mut ed = editor(doc_with(vec![disk("a"), disk("b")], vec![track_for("a")]));
        // Unwindowed fails on the tracked property…
        assert!(ed.add_binding("a", "radius", "b", "radius").is_err());
        // …windowed from the playhead succeeds.
        let b = ed.add_binding_windowed("a", "radius", "b", "radius", None, Some(2.0)).unwrap();
        assert_eq!(ed.doc.bindings[b].end, Some(2.0));

        // And a second binding may follow in a disjoint window.
        let b2 = ed.add_binding_windowed("a", "radius", "b", "radius", Some(2.0), None).unwrap();
        assert_eq!(ed.doc.bindings[b2].start, Some(2.0));

        // Overlapping window edit reverts.
        let err = ed.set_binding_window(b2, Some(1.0), None).unwrap_err();
        assert!(err.contains("overlapping"), "unexpected error: {err}");
        assert_eq!(ed.doc.bindings[b2].start, Some(2.0));

        // Tracks are still addable on a property with only windowed bindings.
        assert!(ed.add_track("a", "position").is_ok());
        ed.doc.build().unwrap();
    }

    #[test]
    fn add_track_rejects_bound_property() {
        let mut ed = editor(doc_with(vec![disk("a"), disk("b")], vec![]));
        ed.doc.bindings = vec![binding("a", "b")];
        let err = ed.add_track("a", "radius").unwrap_err();
        assert!(err.contains("bound"), "unexpected error: {err}");
        // The same property on the source object is still trackable.
        assert!(ed.add_track("b", "radius").is_ok());
    }

    #[test]
    fn rename_rejects_duplicates_and_reserved() {
        let mut ed = editor(doc_with(vec![disk("a"), disk("b")], vec![]));
        assert!(!ed.rename_object(0, "b"));
        assert!(!ed.rename_object(0, "camera"));
        assert!(!ed.rename_object(0, ""));
        assert_eq!(ed.doc.objects[0].id, "a");
    }

    #[test]
    fn upsert_override_inserts_then_updates() {
        let mut ed = editor(doc_with(vec![disk("a")], vec![]));
        ed.upsert_override(0, "radius", AnimValue::Float(2.0));
        ed.upsert_override(0, "radius", AnimValue::Float(3.0));
        assert_eq!(ed.doc.objects[0].set.len(), 1);
        assert_eq!(ed.effective_value(0, "radius"), Some(AnimValue::Float(3.0)));
    }

    #[test]
    fn effective_value_reads_spec_when_no_override() {
        let ed = editor(doc_with(vec![disk("a")], vec![]));
        assert_eq!(ed.effective_value(0, "radius"), Some(AnimValue::Float(1.0)));
    }

    #[test]
    fn add_track_seeds_keyframe_and_rejects_duplicates() {
        let mut ed = editor(doc_with(vec![disk("a")], vec![]));
        let t = ed.add_track("a", "radius").unwrap();
        assert_eq!(ed.doc.tracks[t].keyframes.len(), 1);
        assert_eq!(ed.doc.tracks[t].keyframes[0].value, AnimValue::Float(1.0));
        assert!(ed.add_track("a", "radius").is_err());
        assert!(ed.add_track("a", "nope").is_err());
        assert!(ed.add_track("ghost", "radius").is_err());
    }

    #[test]
    fn add_camera_track() {
        let mut ed = editor(doc_with(vec![disk("a")], vec![]));
        let t = ed.add_track("camera", "fov").unwrap();
        assert_eq!(ed.doc.tracks[t].object, "camera");
        assert_eq!(ed.doc.tracks[t].keyframes[0].value, AnimValue::Float(60.0));
        ed.doc.build().unwrap();
    }

    #[test]
    fn add_keyframe_interpolates_default_value() {
        let mut ed = editor(doc_with(vec![disk("a")], vec![]));
        let t = ed.add_track("a", "radius").unwrap();
        ed.move_keyframe(t, 0, 0.0);
        ed.doc.tracks[t].keyframes[0].value = AnimValue::Float(0.0);
        let k2 = ed.add_keyframe(t, 2.0).unwrap();
        ed.doc.tracks[t].keyframes[k2].value = AnimValue::Float(10.0);
        // Midpoint of a 0..10 linear segment
        let k_mid = ed.add_keyframe(t, 1.0).unwrap();
        assert_eq!(ed.doc.tracks[t].keyframes[k_mid].value, AnimValue::Float(5.0));
    }

    #[test]
    fn move_keyframe_keeps_indices_and_builds_sorted() {
        let mut ed = editor(doc_with(vec![disk("a")], vec![]));
        let t = ed.add_track("a", "radius").unwrap();
        let k2 = ed.add_keyframe(t, 2.0).unwrap();
        // Drag the second keyframe before the first: doc order unchanged...
        ed.move_keyframe(t, k2, 0.5);
        assert_eq!(ed.doc.tracks[t].keyframes[k2].time, 0.5);
        // ...and the runtime build still sorts (duration = max time).
        let (_, timeline, _) = ed.doc.build().unwrap();
        assert!((timeline.duration() - 0.5).abs() < f32::EPSILON);
        ed.move_keyframe(t, k2, -3.0);
        assert_eq!(ed.doc.tracks[t].keyframes[k2].time, 0.0);
    }

    #[test]
    fn remove_keyframe_fixes_selection() {
        let mut ed = editor(doc_with(vec![disk("a")], vec![]));
        let t = ed.add_track("a", "radius").unwrap();
        let k2 = ed.add_keyframe(t, 2.0).unwrap();
        ed.selected_keyframe = Some((t, k2));
        ed.remove_keyframe(t, 0);
        assert_eq!(ed.selected_keyframe, Some((t, k2 - 1)));
        ed.remove_keyframe(t, 0);
        assert_eq!(ed.selected_keyframe, None);
    }

    #[test]
    fn remove_track_fixes_selection() {
        let mut ed = editor(doc_with(vec![disk("a")], vec![]));
        let t1 = ed.add_track("a", "radius").unwrap();
        let t2 = ed.add_track("a", "position").unwrap();
        ed.selected_keyframe = Some((t2, 0));
        ed.remove_track(t1);
        assert_eq!(ed.selected_keyframe, Some((t2 - 1, 0)));
    }

    #[test]
    fn set_keyframe_value_rejects_wrong_variant() {
        let mut ed = editor(doc_with(vec![disk("a")], vec![]));
        let t = ed.add_track("a", "radius").unwrap();
        assert!(ed.set_keyframe_value(t, 0, AnimValue::Float(4.0)));
        assert!(!ed.set_keyframe_value(t, 0, AnimValue::Bool(true)));
        assert_eq!(ed.doc.tracks[t].keyframes[0].value, AnimValue::Float(4.0));
    }

    #[test]
    fn save_normalizes_keyframe_order() {
        let mut ed = editor(doc_with(vec![disk("a")], vec![]));
        let t = ed.add_track("a", "radius").unwrap();
        let k2 = ed.add_keyframe(t, 2.0).unwrap();
        ed.move_keyframe(t, k2, 0.25);
        ed.path = "/tmp/rs_namin_editor_save_test.ron";
        ed.save().unwrap();
        assert!(ed.doc.tracks[t].keyframes.windows(2).all(|w| w[0].time <= w[1].time));
        std::fs::remove_file("/tmp/rs_namin_editor_save_test.ron").ok();
    }

    #[test]
    fn auto_key_without_track_writes_override() {
        let mut ed = editor(doc_with(vec![disk("a")], vec![]));
        ed.auto_key(0, "radius", AnimValue::Float(2.5), 1.0);
        assert!(ed.doc.tracks.is_empty());
        assert_eq!(ed.effective_value(0, "radius"), Some(AnimValue::Float(2.5)));
    }

    #[test]
    fn auto_key_with_track_inserts_then_updates_keyframe() {
        let mut ed = editor(doc_with(vec![disk("a")], vec![]));
        let t = ed.add_track("a", "radius").unwrap();
        ed.auto_key(0, "radius", AnimValue::Float(2.0), 1.0);
        assert_eq!(ed.doc.tracks[t].keyframes.len(), 2);
        assert_eq!(ed.selected_keyframe, Some((t, 1)));
        // Same playhead time again: updates in place instead of stacking.
        ed.auto_key(0, "radius", AnimValue::Float(3.0), 1.0);
        assert_eq!(ed.doc.tracks[t].keyframes.len(), 2);
        assert_eq!(ed.doc.tracks[t].keyframes[1].value, AnimValue::Float(3.0));
    }

    #[test]
    fn doc_still_builds_after_mutations() {
        let mut ed = editor(doc_with(vec![disk("a")], vec![track_for("a")]));
        let templates = palette_templates();
        for (name, spec) in templates {
            ed.add_object(name, spec);
        }
        ed.rename_object(0, "ball");
        ed.upsert_override(0, "radius", AnimValue::Float(2.0));
        ed.doc.build().unwrap();
    }
}
