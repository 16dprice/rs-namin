use macroquad::prelude::{Vec2, Vec3, Vec4};

#[derive(Debug, Clone, PartialEq)]
pub struct Transform2D {
    pub position: Vec2,
    pub rotation: f32,
    pub scale: Vec2,
}

#[derive(Debug, Clone, PartialEq)]
pub enum AnimValue {
    Float(f32),
    Vec2(Vec2),
    Vec3(Vec3),
    Vec4(Vec4),
    Bool(bool),
    Transform2D(Transform2D),
}

impl AnimValue {
    /// Linearly interpolate between two `AnimValue`s of the same variant.
    /// Panics if the variants don't match.
    /// Bool snaps to `b` at t >= 0.5.
    pub fn lerp(a: &AnimValue, b: &AnimValue, t: f32) -> AnimValue {
        match (a, b) {
            (AnimValue::Float(a), AnimValue::Float(b)) => AnimValue::Float(a + (b - a) * t),
            (AnimValue::Vec2(a), AnimValue::Vec2(b)) => AnimValue::Vec2(a.lerp(*b, t)),
            (AnimValue::Vec3(a), AnimValue::Vec3(b)) => AnimValue::Vec3(a.lerp(*b, t)),
            (AnimValue::Vec4(a), AnimValue::Vec4(b)) => AnimValue::Vec4(a.lerp(*b, t)),
            (AnimValue::Bool(_), AnimValue::Bool(b)) => AnimValue::Bool(if t >= 0.5 { *b } else { false }),
            (AnimValue::Transform2D(a), AnimValue::Transform2D(b)) => {
                AnimValue::Transform2D(Transform2D {
                    position: a.position.lerp(b.position, t),
                    rotation: a.rotation + (b.rotation - a.rotation) * t,
                    scale: a.scale.lerp(b.scale, t),
                })
            }
            _ => panic!("Cannot lerp between different AnimValue variants"),
        }
    }
}
