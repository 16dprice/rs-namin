use macroquad::prelude::Vec3;

use super::value::AnimValue;

#[derive(Debug, Clone, Copy)]
pub struct BoundingBox {
    pub min: Vec3,
    pub max: Vec3,
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
}

/// Generates an `Animatable` impl from a single list of `field: Variant` pairs.
///
/// The field name is the property name, so the name list, getter, and setter are
/// derived from one declaration and cannot drift apart. Fields must be `Copy` and
/// public on the type. Types whose setters need side effects (e.g. `Turtle`)
/// implement `Animatable` by hand instead.
///
/// In debug builds, `set` panics on an unknown property name or a mismatched
/// `AnimValue` variant; in release builds it is a no-op (Timeline calls `set`
/// every frame, and `SceneBuilder` already validates at construction time).
///
/// ```ignore
/// animatable!(Disk {
///     position: Vec3,
///     radius: Float,
///     color: Vec4,
/// });
/// ```
macro_rules! animatable {
    ($ty:ty { $($prop:ident: $variant:ident),+ $(,)? }) => {
        impl $crate::scene::traits::Animatable for $ty {
            fn get(&self, property_name: &str) -> Option<$crate::scene::value::AnimValue> {
                match property_name {
                    $(stringify!($prop) => Some($crate::scene::value::AnimValue::$variant(self.$prop)),)+
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
        }
    };
}
pub(crate) use animatable;

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
    /// names return `None`. Shared by all object test modules.
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
        assert_eq!(obj.get("nonexistent_property"), None);
    }
}
