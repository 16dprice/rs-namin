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

#[cfg(test)]
mod tests {
    use macroquad::prelude::*;

    use super::*;

    #[test]
    fn lerp_float_boundaries() {
        let a = AnimValue::Float(0.0);
        let b = AnimValue::Float(10.0);
        assert_eq!(AnimValue::lerp(&a, &b, 0.0), AnimValue::Float(0.0));
        assert_eq!(AnimValue::lerp(&a, &b, 1.0), AnimValue::Float(10.0));
    }

    #[test]
    fn lerp_float_midpoint() {
        let a = AnimValue::Float(0.0);
        let b = AnimValue::Float(10.0);
        assert_eq!(AnimValue::lerp(&a, &b, 0.5), AnimValue::Float(5.0));
    }

    #[test]
    fn lerp_vec3_midpoint() {
        let a = AnimValue::Vec3(vec3(0.0, 0.0, 0.0));
        let b = AnimValue::Vec3(vec3(10.0, 20.0, 30.0));
        let result = AnimValue::lerp(&a, &b, 0.5);
        assert_eq!(result, AnimValue::Vec3(vec3(5.0, 10.0, 15.0)));
    }

    #[test]
    fn lerp_vec4_boundaries() {
        let a = AnimValue::Vec4(vec4(0.0, 0.0, 0.0, 0.0));
        let b = AnimValue::Vec4(vec4(1.0, 1.0, 1.0, 1.0));
        assert_eq!(AnimValue::lerp(&a, &b, 0.0), a);
        assert_eq!(AnimValue::lerp(&a, &b, 1.0), b);
    }

    #[test]
    fn lerp_bool_snaps_at_half() {
        let a = AnimValue::Bool(false);
        let b = AnimValue::Bool(true);
        assert_eq!(AnimValue::lerp(&a, &b, 0.0), AnimValue::Bool(false));
        assert_eq!(AnimValue::lerp(&a, &b, 0.49), AnimValue::Bool(false));
        assert_eq!(AnimValue::lerp(&a, &b, 0.5), AnimValue::Bool(true));
        assert_eq!(AnimValue::lerp(&a, &b, 1.0), AnimValue::Bool(true));
    }

    #[test]
    fn lerp_transform2d_midpoint() {
        let a = AnimValue::Transform2D(Transform2D {
            position: vec2(0.0, 0.0),
            rotation: 0.0,
            scale: vec2(1.0, 1.0),
        });
        let b = AnimValue::Transform2D(Transform2D {
            position: vec2(10.0, 20.0),
            rotation: 3.14,
            scale: vec2(2.0, 2.0),
        });
        let result = AnimValue::lerp(&a, &b, 0.5);
        match result {
            AnimValue::Transform2D(t) => {
                assert_eq!(t.position, vec2(5.0, 10.0));
                assert!((t.rotation - 1.57).abs() < 0.001);
                assert_eq!(t.scale, vec2(1.5, 1.5));
            }
            _ => panic!("Expected Transform2D"),
        }
    }

    #[test]
    #[should_panic(expected = "Cannot lerp between different AnimValue variants")]
    fn lerp_mismatched_variants_panics() {
        let a = AnimValue::Float(0.0);
        let b = AnimValue::Vec3(vec3(1.0, 2.0, 3.0));
        AnimValue::lerp(&a, &b, 0.5);
    }
}
