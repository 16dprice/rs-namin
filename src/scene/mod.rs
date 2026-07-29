pub mod bezier;
pub mod color;
pub mod expr;
pub mod font;
pub mod l_system;
pub mod latex;
pub mod mesh;
pub mod objects;
pub mod polyline;
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

pub struct Scene {
    objects: Vec<Option<Box<dyn SceneNode>>>,
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
        self.objects.push(Some(object));
        id
    }

    pub fn remove(&mut self, id: ObjectId) -> Option<Box<dyn SceneNode>> {
        self.objects.get_mut(id.0)?.take()
    }

    pub fn get(&self, id: ObjectId) -> Option<&dyn SceneNode> {
        self.objects.get(id.0)?.as_deref()
    }

    pub fn get_mut(&mut self, id: ObjectId) -> Option<&mut (dyn SceneNode + 'static)> {
        self.objects.get_mut(id.0)?.as_deref_mut()
    }

    pub fn iter(&self) -> impl Iterator<Item = (ObjectId, &dyn SceneNode)> {
        self.objects
            .iter()
            .enumerate()
            .filter_map(|(i, o)| o.as_deref().map(|obj| (ObjectId(i), obj)))
    }

    pub fn len(&self) -> usize {
        self.objects.iter().filter(|o| o.is_some()).count()
    }

    pub fn is_empty(&self) -> bool {
        self.objects.iter().all(|o| o.is_none())
    }

    /// Draw all world-space objects (called during the 3D camera pass).
    pub fn draw_world(&self) {
        for object in self.objects.iter().flatten() {
            if !object.is_screen_space() {
                object.draw();
            }
        }
    }

    /// Draw all screen-space objects (called after set_default_camera).
    pub fn draw_screen(&self) {
        for object in self.objects.iter().flatten() {
            if object.is_screen_space() {
                object.draw();
            }
        }
    }
}

impl Default for Scene {
    fn default() -> Self {
        Self::new()
    }
}
