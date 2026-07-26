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
use crate::doc::{KeyframeDoc, ObjectDoc, ObjectSpec, SceneDoc, TrackDoc};
use crate::scene::traits::Animatable;
use crate::scene::value::AnimValue;

pub struct EditorState {
    pub doc: SceneDoc,
    pub path: &'static str,
    pub selected: Option<usize>,
    /// Selected keyframe in the dope sheet: (track index, keyframe index).
    pub selected_keyframe: Option<(usize, usize)>,
    pub dirty: bool,
    /// A doc edit happened this frame; the owner rebuilds the scene.
    pub rebuild_needed: bool,
    /// Last build or save error (editing continues).
    pub error: Option<String>,
    /// Buffer for the id TextEdit (committed on focus loss / enter).
    id_buffer: String,
}

impl EditorState {
    pub fn new(doc: SceneDoc, path: &'static str) -> Self {
        let mut state = Self {
            doc,
            path,
            selected: None,
            selected_keyframe: None,
            dirty: false,
            rebuild_needed: false,
            error: None,
            id_buffer: String::new(),
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
        self.id_buffer = match index {
            Some(i) => self.doc.objects[i].id.clone(),
            None => String::new(),
        };
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

    /// Remove an object and every track that references it.
    pub fn remove_object(&mut self, index: usize) {
        let id = self.doc.objects.remove(index).id;
        self.doc.tracks.retain(|t| t.object != id);
        match self.selected {
            Some(s) if s == index => self.select(None),
            Some(s) if s > index => self.selected = Some(s - 1),
            _ => {}
        }
        self.touch();
    }

    /// Rename an object, cascading to tracks. Rejects empty, duplicate, and
    /// reserved ids; returns whether the rename applied.
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

    /// The AnimValue a track's keyframes must carry (from the target's
    /// property type).
    fn target_value_template(&self, target: &str, property: &str) -> Option<AnimValue> {
        if target == "camera" {
            return Camera::default().get(property);
        }
        let index = self.doc.objects.iter().position(|o| o.id == target)?;
        self.effective_value(index, property)
    }

    /// Add a track for `target.property` with one keyframe at t=0 holding
    /// the current initial value. Rejects unknown targets/properties and
    /// duplicate tracks.
    pub fn add_track(&mut self, target: &str, property: &str) -> Result<usize, String> {
        if self.doc.tracks.iter().any(|t| t.object == target && t.property == property) {
            return Err(format!("track {target}.{property} already exists"));
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
            }],
        });
        self.touch();
        Ok(self.doc.tracks.len() - 1)
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
            track.add_keyframe(crate::animation::track::Keyframe::with_easing(kf.time, kf.value.clone(), kf.easing));
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
                position: vec2(40.0, 80.0),
                font_size: 40.0,
                color: white,
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
    }
}

/// Draw the editor panels (palette left, inspector right). Call inside the
/// egui pass, viewer mode only.
pub fn panels(ctx: &egui::Context, editor: &mut EditorState) {
    palette_panel(ctx, editor);
    if editor.selected.is_some() {
        inspector_panel(ctx, editor);
    }
}

fn palette_panel(ctx: &egui::Context, editor: &mut EditorState) {
    egui::SidePanel::left("editor_palette")
        .resizable(false)
        .exact_width(230.0)
        .show(ctx, |ui| {
            ui.add_space(6.0);
            ui.horizontal(|ui| {
                ui.heading("Scene");
                if editor.dirty {
                    ui.colored_label(egui::Color32::LIGHT_YELLOW, "●");
                }
            });

            if ui.text_edit_singleline(&mut editor.doc.description).changed() {
                editor.dirty = true;
            }

            ui.add_space(4.0);
            ui.horizontal(|ui| {
                if ui.button("Save").clicked() {
                    match editor.save() {
                        Ok(()) => editor.error = None,
                        Err(e) => editor.error = Some(e),
                    }
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

            // Object list.
            let mut remove: Option<usize> = None;
            let mut select: Option<usize> = None;
            egui::ScrollArea::vertical().show(ui, |ui| {
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

fn inspector_panel(ctx: &egui::Context, editor: &mut EditorState) {
    let Some(index) = editor.selected else { return };
    if index >= editor.doc.objects.len() {
        editor.select(None);
        return;
    }

    egui::SidePanel::right("editor_inspector")
        .resizable(false)
        .exact_width(280.0)
        .show(ctx, |ui| {
            ui.add_space(6.0);
            ui.heading(spec_type_name(&editor.doc.objects[index].object));

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
            if let ObjectSpec::Text { content, .. } = &mut editor.doc.objects[index].object {
                ui.horizontal(|ui| {
                    ui.weak("content");
                    spec_changed |= ui.text_edit_singleline(content).changed();
                });
            }
            if spec_changed {
                editor.touch();
            }

            ui.separator();
            ui.weak("Initial properties (saved as overrides)");
            ui.add_space(4.0);

            // Generated editors over the Animatable surface.
            let properties = editor.property_names(index);
            egui::ScrollArea::vertical().show(ui, |ui| {
                egui::Grid::new("editor_props").num_columns(2).spacing([10.0, 6.0]).show(ui, |ui| {
                    for property in &properties {
                        let Some(mut value) = editor.effective_value(index, property) else {
                            continue;
                        };
                        ui.weak(property);
                        if anim_value_edit(ui, property, &mut value) {
                            editor.upsert_override(index, property, value);
                        }
                        ui.end_row();
                    }
                });
            });
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
}

/// Human label for an easing (named variants read as their name).
pub fn easing_label(easing: Easing) -> String {
    match easing {
        Easing::Custom(_) => "Custom".to_string(),
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
    egui::ScrollArea::vertical().max_height(5.5 * SHEET_ROW_H).show(ui, |ui| {
        for track_index in 0..editor.doc.tracks.len() {
            let label = format!(
                "{}.{}",
                editor.doc.tracks[track_index].object, editor.doc.tracks[track_index].property
            );
            ui.horizontal(|ui| {
                let (label_rect, _) = ui.allocate_exact_size(egui::vec2(SHEET_LABEL_W, SHEET_ROW_H), egui::Sense::hover());
                {
                    let text_area = egui::Rect::from_min_max(label_rect.min, egui::pos2(label_rect.right() - 40.0, label_rect.bottom()));
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
                let delete_rect = egui::Rect::from_min_size(label_rect.right_top() - egui::vec2(18.0, 0.0), egui::vec2(16.0, SHEET_ROW_H));
                if ui
                    .put(delete_rect, egui::Button::new("x").small())
                    .on_hover_text("Delete track")
                    .clicked()
                {
                    action = Some(SheetAction::RemoveTrack(track_index));
                }

                let (lane_rect, lane_resp) = ui.allocate_exact_size(egui::vec2(ui.available_width(), SHEET_ROW_H), egui::Sense::click());
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
        None => {}
    }

    // Delete key removes the selected keyframe (unless a widget has focus).
    let delete_pressed = ui.ctx().input(|i| i.key_pressed(egui::Key::Delete)) && ui.ctx().memory(|m| m.focused().is_none());
    if delete_pressed && let Some((t, k)) = editor.selected_keyframe {
        editor.remove_keyframe(t, k);
    }

    keyframe_detail_strip(ui, editor);
}

fn add_track_menu(ui: &mut egui::Ui, editor: &mut EditorState) {
    let mut targets: Vec<String> = editor.doc.objects.iter().map(|o| o.id.clone()).collect();
    targets.push("camera".to_string());

    ui.menu_button("+ Track", |ui| {
        for target in &targets {
            ui.menu_button(target, |ui| {
                for property in editor.target_property_names(target).unwrap_or_default() {
                    let exists = editor.doc.tracks.iter().any(|t| &t.object == target && t.property == property);
                    if ui.add_enabled(!exists, egui::Button::new(&property)).clicked() {
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
                });
            if easing != kf.easing {
                editor.set_keyframe_easing(track_index, kf_index, easing);
            }
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
            objects,
            tracks,
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
            }],
        }
    }

    fn editor(doc: SceneDoc) -> EditorState {
        EditorState::new(doc, "scenes/test.ron")
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
