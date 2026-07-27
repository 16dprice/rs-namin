use macroquad::prelude::Vec3;

use super::value::AnimValue;

#[derive(Debug, Clone, Copy)]
pub struct BoundingBox {
    pub min: Vec3,
    pub max: Vec3,
}

impl BoundingBox {
    /// Ray-AABB intersection (slab method). Returns the distance along the
    /// ray to the entry point, or None if the ray misses or the box is
    /// entirely behind the origin. Zero-extent axes (flat boxes, e.g. XY
    /// objects) are handled with a small tolerance.
    pub fn ray_intersect(&self, origin: Vec3, dir: Vec3) -> Option<f32> {
        const PAD: f32 = 1e-4;
        let mut t_min = 0.0_f32;
        let mut t_max = f32::MAX;
        for axis in 0..3 {
            let (o, d) = (origin[axis], dir[axis]);
            let (lo, hi) = (self.min[axis] - PAD, self.max[axis] + PAD);
            if d.abs() < 1e-8 {
                if o < lo || o > hi {
                    return None;
                }
            } else {
                let mut t0 = (lo - o) / d;
                let mut t1 = (hi - o) / d;
                if t0 > t1 {
                    std::mem::swap(&mut t0, &mut t1);
                }
                t_min = t_min.max(t0);
                t_max = t_max.min(t1);
                if t_min > t_max {
                    return None;
                }
            }
        }
        Some(t_min)
    }
}

pub trait SceneObject {
    fn draw(&self);
    fn bounding_box(&self) -> BoundingBox;
    /// Returns true if this object should be drawn in screen space (after set_default_camera).
    /// Default is false (world-space, drawn during the 3D camera pass).
    fn is_screen_space(&self) -> bool {
        false
    }
}

pub trait Animatable {
    fn get(&self, property_name: &str) -> Option<AnimValue>;
    fn set(&mut self, property_name: &str, value: AnimValue);
    fn property_names(&self) -> &[&str];
    /// Read-only derived properties: served by `get`, never settable, and
    /// usable as binding sources (e.g. LSystem's `pen_position`). Each name
    /// corresponds to a same-named computing method on the type.
    fn output_names(&self) -> &[&str] {
        &[]
    }
}

/// Generates an `Animatable` impl from a single list of `field: Variant` pairs.
///
/// The field name is the property name, so the name list, getter, and setter are
/// derived from one declaration and cannot drift apart. Fields must be `Copy` and
/// public on the type. Types whose setters need side effects (e.g. `Turtle`)
/// implement `Animatable` by hand instead.
///
/// An optional `outputs` block declares read-only derived properties; each
/// name is a same-named zero-arg method on the type returning the field type
/// of its variant. Outputs are served by `get` (so bindings can source them)
/// but are not settable and not in `property_names`.
///
/// In debug builds, `set` panics on an unknown property name or a mismatched
/// `AnimValue` variant; in release builds it is a no-op (Timeline calls `set`
/// every frame, and `SceneBuilder` already validates at construction time).
///
/// ```ignore
/// animatable!(LSystem {
///     position: Vec3,
///     progress: Float,
/// } outputs {
///     pen_position: Vec3,
/// });
/// ```
macro_rules! animatable {
    ($ty:ty { $($prop:ident: $variant:ident),+ $(,)? } $(outputs { $($out:ident: $outvariant:ident),+ $(,)? })?) => {
        impl $crate::scene::traits::Animatable for $ty {
            fn get(&self, property_name: &str) -> Option<$crate::scene::value::AnimValue> {
                match property_name {
                    $(stringify!($prop) => Some($crate::scene::value::AnimValue::$variant(self.$prop)),)+
                    $($(stringify!($out) => Some($crate::scene::value::AnimValue::$outvariant(self.$out())),)+)?
                    _ => None,
                }
            }

            fn set(&mut self, property_name: &str, value: $crate::scene::value::AnimValue) {
                match (property_name, value) {
                    $((stringify!($prop), $crate::scene::value::AnimValue::$variant(v)) => self.$prop = v,)+
                    (name, value) => {
                        debug_assert!(
                            false,
                            "Animatable::set on {}: unknown property {:?} or wrong value type {:?}",
                            stringify!($ty),
                            name,
                            value
                        );
                    }
                }
            }

            fn property_names(&self) -> &[&str] {
                &[$(stringify!($prop)),+]
            }

            fn output_names(&self) -> &[&str] {
                &[$($(stringify!($out)),+)?]
            }
        }
    };
}
pub(crate) use animatable;

#[cfg(test)]
mod bounding_box_tests {
    use macroquad::prelude::vec3;

    use super::BoundingBox;

    fn unit_box() -> BoundingBox {
        BoundingBox {
            min: vec3(-1.0, -1.0, -1.0),
            max: vec3(1.0, 1.0, 1.0),
        }
    }

    #[test]
    fn ray_hits_box_head_on() {
        let t = unit_box().ray_intersect(vec3(0.0, 0.0, 5.0), vec3(0.0, 0.0, -1.0)).unwrap();
        assert!((t - 4.0).abs() < 1e-3);
    }

    #[test]
    fn ray_misses_box() {
        assert!(unit_box().ray_intersect(vec3(5.0, 0.0, 5.0), vec3(0.0, 0.0, -1.0)).is_none());
    }

    #[test]
    fn box_behind_origin_misses() {
        assert!(unit_box().ray_intersect(vec3(0.0, 0.0, 5.0), vec3(0.0, 0.0, 1.0)).is_none());
    }

    #[test]
    fn flat_box_is_hittable() {
        // XY-plane objects have zero Z extent.
        let flat = BoundingBox {
            min: vec3(-1.0, -1.0, 0.0),
            max: vec3(1.0, 1.0, 0.0),
        };
        assert!(flat.ray_intersect(vec3(0.0, 0.0, 5.0), vec3(0.0, 0.0, -1.0)).is_some());
        // A ray inside the flat slab travelling parallel to it still hits.
        assert!(flat.ray_intersect(vec3(5.0, 0.0, 0.0), vec3(-1.0, 0.0, 0.0)).is_some());
    }

    #[test]
    fn ray_starting_inside_returns_zero() {
        let t = unit_box().ray_intersect(vec3(0.0, 0.0, 0.0), vec3(0.0, 0.0, -1.0)).unwrap();
        assert!(t.abs() < 1e-3);
    }
}

#[cfg(test)]
pub mod test_support {
    use macroquad::prelude::{Mat4, Vec3};

    use super::Animatable;
    use crate::scene::value::AnimValue;

    fn perturbed(v: &AnimValue) -> AnimValue {
        match v {
            AnimValue::Float(f) => AnimValue::Float(f + 1.0),
            AnimValue::Vec2(v) => AnimValue::Vec2(*v + 1.0),
            AnimValue::Vec3(v) => AnimValue::Vec3(*v + 1.0),
            AnimValue::Vec4(v) => AnimValue::Vec4(*v + 0.25),
            AnimValue::Bool(b) => AnimValue::Bool(!b),
            AnimValue::Transform2D(t) => {
                let mut t = t.clone();
                t.rotation += 1.0;
                AnimValue::Transform2D(t)
            }
            AnimValue::Mat4(m) => AnimValue::Mat4(*m * Mat4::from_translation(Vec3::ONE)),
        }
    }

    /// Assert every declared property round-trips: `get` returns a value, `set`
    /// with a perturbed value of the same variant sticks, and unknown property
    /// names return `None`. Output properties must be readable but never
    /// listed as settable. Shared by all object test modules.
    pub fn assert_property_roundtrip(obj: &mut dyn Animatable) {
        let names: Vec<String> = obj.property_names().iter().map(|s| s.to_string()).collect();
        assert!(!names.is_empty(), "object declares no animatable properties");
        for name in &names {
            let value = obj
                .get(name)
                .unwrap_or_else(|| panic!("property {name:?} is in property_names() but get() returned None"));
            let new_value = perturbed(&value);
            obj.set(name, new_value.clone());
            assert_eq!(
                obj.get(name),
                Some(new_value),
                "property {name:?} did not round-trip through set/get"
            );
        }
        let outputs: Vec<String> = obj.output_names().iter().map(|s| s.to_string()).collect();
        for name in &outputs {
            assert!(
                obj.get(name).is_some(),
                "output {name:?} is in output_names() but get() returned None"
            );
            assert!(!names.contains(name), "output {name:?} must not also be in property_names()");
        }
        assert_eq!(obj.get("nonexistent_property"), None);
    }
}
