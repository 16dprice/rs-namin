#[cfg(test)]
mod tests {
    use macroquad::prelude::*;

    use crate::scene::value::{AnimValue, Transform2D};

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
