//! Scene-document editor: object palette and property inspector.
//!
//! The document is the single source of truth — every edit marks the state
//! dirty and queues a scene rebuild (`ViewerMode` rebuilds from the doc at
//! the end of the frame). Document mutations live on `EditorState` as plain
//! methods so they stay unit-testable; the egui panels call into them.

use egui_macroquad::egui;
use macroquad::prelude::*;

use crate::doc::{ObjectDoc, ObjectSpec, SceneDoc};
use crate::scene::value::AnimValue;

pub struct EditorState {
    pub doc: SceneDoc,
    pub path: &'static str,
    pub selected: Option<usize>,
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

    pub fn save(&mut self) -> Result<(), String> {
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
