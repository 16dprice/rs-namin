pub mod bezier;
pub mod color;
pub mod expr;
pub mod font;
pub mod l_system;
pub mod latex;
pub mod mesh;
pub mod objects;
pub mod polyline;
pub mod texture_cache;
pub mod traits;
pub mod value;

use traits::{Animatable, SceneObject};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct ObjectId(usize);

impl ObjectId {
    #[cfg(test)]
    pub fn test_id(index: usize) -> Self {
        Self(index)
    }
}

pub trait SceneNode: SceneObject + Animatable {}
impl<T: SceneObject + Animatable> SceneNode for T {}

struct Entry {
    object: Box<dyn SceneNode>,
    /// Hidden objects skip both draw passes but stay fully addressable
    /// (get/set/iter). Derived per-frame from appearance times by
    /// `Timeline::apply` — never persisted state.
    visible: bool,
}

pub struct Scene {
    objects: Vec<Option<Entry>>,
}

impl Scene {
    pub fn new() -> Self {
        Self { objects: Vec::new() }
    }

    pub fn add(&mut self, object: impl SceneNode + 'static) -> ObjectId {
        self.add_boxed(Box::new(object))
    }

    /// Add an already-boxed object (used by data-driven construction, where
    /// the concrete type is decided at runtime).
    pub fn add_boxed(&mut self, object: Box<dyn SceneNode>) -> ObjectId {
        let id = ObjectId(self.objects.len());
        self.objects.push(Some(Entry { object, visible: true }));
        id
    }

    pub fn remove(&mut self, id: ObjectId) -> Option<Box<dyn SceneNode>> {
        Some(self.objects.get_mut(id.0)?.take()?.object)
    }

    pub fn get(&self, id: ObjectId) -> Option<&dyn SceneNode> {
        Some(self.objects.get(id.0)?.as_ref()?.object.as_ref())
    }

    pub fn get_mut(&mut self, id: ObjectId) -> Option<&mut (dyn SceneNode + 'static)> {
        Some(self.objects.get_mut(id.0)?.as_mut()?.object.as_mut())
    }

    pub fn set_visible(&mut self, id: ObjectId, visible: bool) {
        if let Some(Some(entry)) = self.objects.get_mut(id.0) {
            entry.visible = visible;
        }
    }

    pub fn is_visible(&self, id: ObjectId) -> bool {
        matches!(self.objects.get(id.0), Some(Some(entry)) if entry.visible)
    }

    pub fn iter(&self) -> impl Iterator<Item = (ObjectId, &dyn SceneNode)> {
        self.objects
            .iter()
            .enumerate()
            .filter_map(|(i, o)| o.as_ref().map(|entry| (ObjectId(i), entry.object.as_ref())))
    }

    pub fn len(&self) -> usize {
        self.objects.iter().filter(|o| o.is_some()).count()
    }

    pub fn is_empty(&self) -> bool {
        self.objects.iter().all(|o| o.is_none())
    }

    /// Draw all visible world-space objects (called during the 3D camera pass).
    pub fn draw_world(&self) {
        for entry in self.objects.iter().flatten() {
            if entry.visible && !entry.object.is_screen_space() {
                entry.object.draw();
            }
        }
    }

    /// Draw all visible screen-space objects (called after set_default_camera).
    pub fn draw_screen(&self) {
        for entry in self.objects.iter().flatten() {
            if entry.visible && entry.object.is_screen_space() {
                entry.object.draw();
            }
        }
    }
}

impl Default for Scene {
    fn default() -> Self {
        Self::new()
    }
}
