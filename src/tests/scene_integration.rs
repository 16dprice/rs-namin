use macroquad::prelude::*;

use crate::scene::objects::{Disk, Line, Polygon, Rectangle, Torus};
use crate::scene::traits::Animatable;
use crate::scene::value::AnimValue;
use crate::scene::Scene;

#[test]
fn add_and_get_object() {
    let mut scene = Scene::new();
    let id = scene.add(Disk::new(vec3(0.0, 0.0, 0.0), 50.0, BLUE));
    assert!(scene.get(id).is_some());
    assert_eq!(scene.len(), 1);
}

#[test]
fn remove_object() {
    let mut scene = Scene::new();
    let id = scene.add(Disk::new(vec3(0.0, 0.0, 0.0), 50.0, BLUE));
    assert!(scene.remove(id).is_some());
    assert!(scene.get(id).is_none());
    assert_eq!(scene.len(), 0);
}

#[test]
fn remove_nonexistent_returns_none() {
    let mut scene = Scene::new();
    let id = scene.add(Disk::new(vec3(0.0, 0.0, 0.0), 50.0, BLUE));
    scene.remove(id);
    assert!(scene.remove(id).is_none());
}

#[test]
fn multiple_objects() {
    let mut scene = Scene::new();
    let id1 = scene.add(Disk::new(vec3(0.0, 0.0, 0.0), 50.0, BLUE));
    let id2 = scene.add(Line::new(
        vec3(0.0, 0.0, 0.0),
        vec3(100.0, 100.0, 0.0),
        2.0,
        RED,
    ));
    assert_ne!(id1, id2);
    assert_eq!(scene.len(), 2);
}

#[test]
fn iter_returns_all_objects() {
    let mut scene = Scene::new();
    scene.add(Disk::new(vec3(0.0, 0.0, 0.0), 50.0, BLUE));
    scene.add(Disk::new(vec3(100.0, 100.0, 0.0), 30.0, RED));
    let count = scene.iter().count();
    assert_eq!(count, 2);
}

#[test]
fn is_empty() {
    let scene = Scene::new();
    assert!(scene.is_empty());
}

// -- Disk property round-trip tests --

#[test]
fn circle_property_roundtrip_position() {
    let mut circle = Disk::new(vec3(0.0, 0.0, 0.0), 50.0, BLUE);
    let new_pos = AnimValue::Vec3(vec3(10.0, 20.0, 30.0));
    circle.set("position", new_pos.clone());
    assert_eq!(circle.get("position"), Some(new_pos));
}

#[test]
fn circle_property_roundtrip_radius() {
    let mut circle = Disk::new(vec3(0.0, 0.0, 0.0), 50.0, BLUE);
    let new_radius = AnimValue::Float(75.0);
    circle.set("radius", new_radius.clone());
    assert_eq!(circle.get("radius"), Some(new_radius));
}

#[test]
fn circle_property_roundtrip_color() {
    let mut circle = Disk::new(vec3(0.0, 0.0, 0.0), 50.0, BLUE);
    let new_color = AnimValue::Vec4(vec4(1.0, 0.0, 0.0, 1.0));
    circle.set("color", new_color.clone());
    assert_eq!(circle.get("color"), Some(new_color));
}

#[test]
fn circle_get_unknown_property_returns_none() {
    let circle = Disk::new(vec3(0.0, 0.0, 0.0), 50.0, BLUE);
    assert_eq!(circle.get("nonexistent"), None);
}

#[test]
fn circle_property_names_match_getters() {
    let circle = Disk::new(vec3(0.0, 0.0, 0.0), 50.0, BLUE);
    for name in circle.property_names() {
        assert!(
            circle.get(name).is_some(),
            "property '{}' returned None",
            name
        );
    }
}

// -- Line property round-trip tests --

#[test]
fn line_property_roundtrip_start() {
    let mut line = Line::new(vec3(0.0, 0.0, 0.0), vec3(1.0, 1.0, 0.0), 2.0, RED);
    let new_start = AnimValue::Vec3(vec3(5.0, 5.0, 5.0));
    line.set("start", new_start.clone());
    assert_eq!(line.get("start"), Some(new_start));
}

#[test]
fn line_property_roundtrip_end() {
    let mut line = Line::new(vec3(0.0, 0.0, 0.0), vec3(1.0, 1.0, 0.0), 2.0, RED);
    let new_end = AnimValue::Vec3(vec3(50.0, 50.0, 0.0));
    line.set("end", new_end.clone());
    assert_eq!(line.get("end"), Some(new_end));
}

#[test]
fn line_property_roundtrip_thickness() {
    let mut line = Line::new(vec3(0.0, 0.0, 0.0), vec3(1.0, 1.0, 0.0), 2.0, RED);
    let new_thickness = AnimValue::Float(5.0);
    line.set("thickness", new_thickness.clone());
    assert_eq!(line.get("thickness"), Some(new_thickness));
}

#[test]
fn line_property_names_match_getters() {
    let line = Line::new(vec3(0.0, 0.0, 0.0), vec3(1.0, 1.0, 0.0), 2.0, RED);
    for name in line.property_names() {
        assert!(
            line.get(name).is_some(),
            "property '{}' returned None",
            name
        );
    }
}

// -- Rectangle property round-trip tests --

#[test]
fn rectangle_property_roundtrip_position() {
    let mut rect = Rectangle::new(vec3(0.0, 0.0, 0.0), vec2(2.0, 3.0), GREEN);
    let new_pos = AnimValue::Vec3(vec3(1.0, 2.0, 3.0));
    rect.set("position", new_pos.clone());
    assert_eq!(rect.get("position"), Some(new_pos));
}

#[test]
fn rectangle_property_roundtrip_size() {
    let mut rect = Rectangle::new(vec3(0.0, 0.0, 0.0), vec2(2.0, 3.0), GREEN);
    let new_size = AnimValue::Vec2(vec2(5.0, 10.0));
    rect.set("size", new_size.clone());
    assert_eq!(rect.get("size"), Some(new_size));
}

#[test]
fn rectangle_property_roundtrip_color() {
    let mut rect = Rectangle::new(vec3(0.0, 0.0, 0.0), vec2(2.0, 3.0), GREEN);
    let new_color = AnimValue::Vec4(vec4(1.0, 0.0, 0.0, 1.0));
    rect.set("color", new_color.clone());
    assert_eq!(rect.get("color"), Some(new_color));
}

#[test]
fn rectangle_property_names_match_getters() {
    let rect = Rectangle::new(vec3(0.0, 0.0, 0.0), vec2(2.0, 3.0), GREEN);
    for name in rect.property_names() {
        assert!(
            rect.get(name).is_some(),
            "property '{}' returned None",
            name
        );
    }
}

// -- Polygon property round-trip tests --

#[test]
fn polygon_property_roundtrip_position() {
    let mut poly = Polygon::new(vec3(0.0, 0.0, 0.0), 1.0, 6, YELLOW);
    let new_pos = AnimValue::Vec3(vec3(5.0, 5.0, 0.0));
    poly.set("position", new_pos.clone());
    assert_eq!(poly.get("position"), Some(new_pos));
}

#[test]
fn polygon_property_roundtrip_radius() {
    let mut poly = Polygon::new(vec3(0.0, 0.0, 0.0), 1.0, 6, YELLOW);
    let new_radius = AnimValue::Float(3.0);
    poly.set("radius", new_radius.clone());
    assert_eq!(poly.get("radius"), Some(new_radius));
}

#[test]
fn polygon_property_roundtrip_sides() {
    let mut poly = Polygon::new(vec3(0.0, 0.0, 0.0), 1.0, 6, YELLOW);
    poly.set("sides", AnimValue::Float(5.0));
    assert_eq!(poly.get("sides"), Some(AnimValue::Float(5.0)));
}

#[test]
fn polygon_sides_clamps_to_minimum_3() {
    let mut poly = Polygon::new(vec3(0.0, 0.0, 0.0), 1.0, 6, YELLOW);
    poly.set("sides", AnimValue::Float(1.0));
    assert_eq!(poly.get("sides"), Some(AnimValue::Float(3.0)));
}

#[test]
fn polygon_property_roundtrip_rotation() {
    let mut poly = Polygon::new(vec3(0.0, 0.0, 0.0), 1.0, 6, YELLOW);
    let new_rot = AnimValue::Float(1.5);
    poly.set("rotation", new_rot.clone());
    assert_eq!(poly.get("rotation"), Some(new_rot));
}

#[test]
fn polygon_property_roundtrip_color() {
    let mut poly = Polygon::new(vec3(0.0, 0.0, 0.0), 1.0, 6, YELLOW);
    let new_color = AnimValue::Vec4(vec4(0.0, 0.0, 1.0, 1.0));
    poly.set("color", new_color.clone());
    assert_eq!(poly.get("color"), Some(new_color));
}

#[test]
fn polygon_property_names_match_getters() {
    let poly = Polygon::new(vec3(0.0, 0.0, 0.0), 1.0, 6, YELLOW);
    for name in poly.property_names() {
        assert!(
            poly.get(name).is_some(),
            "property '{}' returned None",
            name
        );
    }
}

#[test]
fn polygon_new_clamps_sides() {
    let poly = Polygon::new(vec3(0.0, 0.0, 0.0), 1.0, 2, RED);
    assert_eq!(poly.get("sides"), Some(AnimValue::Float(3.0)));
}

// -- Torus property round-trip tests --

#[test]
fn torus_property_roundtrip_position() {
    let mut torus = Torus::new(Vec3::ZERO, 2.0, 0.5, BLUE);
    let new_pos = AnimValue::Vec3(vec3(10.0, 20.0, 30.0));
    torus.set("position", new_pos.clone());
    assert_eq!(torus.get("position"), Some(new_pos));
}

#[test]
fn torus_property_roundtrip_major_radius() {
    let mut torus = Torus::new(Vec3::ZERO, 2.0, 0.5, BLUE);
    let new_val = AnimValue::Float(5.0);
    torus.set("major_radius", new_val.clone());
    assert_eq!(torus.get("major_radius"), Some(new_val));
}

#[test]
fn torus_property_roundtrip_minor_radius() {
    let mut torus = Torus::new(Vec3::ZERO, 2.0, 0.5, BLUE);
    let new_val = AnimValue::Float(1.0);
    torus.set("minor_radius", new_val.clone());
    assert_eq!(torus.get("minor_radius"), Some(new_val));
}

#[test]
fn torus_property_roundtrip_rotation() {
    let mut torus = Torus::new(Vec3::ZERO, 2.0, 0.5, BLUE);
    let new_rot = AnimValue::Mat4(Mat4::from_rotation_z(std::f32::consts::FRAC_PI_4));
    torus.set("rotation", new_rot.clone());
    assert_eq!(torus.get("rotation"), Some(new_rot));
}

#[test]
fn torus_property_roundtrip_color() {
    let mut torus = Torus::new(Vec3::ZERO, 2.0, 0.5, BLUE);
    let new_color = AnimValue::Vec4(vec4(1.0, 0.0, 0.0, 1.0));
    torus.set("color", new_color.clone());
    assert_eq!(torus.get("color"), Some(new_color));
}

#[test]
fn torus_property_names_match_getters() {
    let torus = Torus::new(Vec3::ZERO, 2.0, 0.5, BLUE);
    for name in torus.property_names() {
        assert!(
            torus.get(name).is_some(),
            "property '{}' returned None",
            name
        );
    }
}
