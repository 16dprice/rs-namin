use macroquad::prelude::*;

use crate::scene::objects::{Circle, Line};
use crate::scene::traits::Animatable;
use crate::scene::value::AnimValue;
use crate::scene::Scene;

#[test]
fn add_and_get_object() {
    let mut scene = Scene::new();
    let id = scene.add(Circle::new(vec3(0.0, 0.0, 0.0), 50.0, BLUE));
    assert!(scene.get(id).is_some());
    assert_eq!(scene.len(), 1);
}

#[test]
fn remove_object() {
    let mut scene = Scene::new();
    let id = scene.add(Circle::new(vec3(0.0, 0.0, 0.0), 50.0, BLUE));
    assert!(scene.remove(id).is_some());
    assert!(scene.get(id).is_none());
    assert_eq!(scene.len(), 0);
}

#[test]
fn remove_nonexistent_returns_none() {
    let mut scene = Scene::new();
    let id = scene.add(Circle::new(vec3(0.0, 0.0, 0.0), 50.0, BLUE));
    scene.remove(id);
    assert!(scene.remove(id).is_none());
}

#[test]
fn multiple_objects() {
    let mut scene = Scene::new();
    let id1 = scene.add(Circle::new(vec3(0.0, 0.0, 0.0), 50.0, BLUE));
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
    scene.add(Circle::new(vec3(0.0, 0.0, 0.0), 50.0, BLUE));
    scene.add(Circle::new(vec3(100.0, 100.0, 0.0), 30.0, RED));
    let count = scene.iter().count();
    assert_eq!(count, 2);
}

#[test]
fn is_empty() {
    let scene = Scene::new();
    assert!(scene.is_empty());
}

// -- Circle property round-trip tests --

#[test]
fn circle_property_roundtrip_position() {
    let mut circle = Circle::new(vec3(0.0, 0.0, 0.0), 50.0, BLUE);
    let new_pos = AnimValue::Vec3(vec3(10.0, 20.0, 30.0));
    circle.set("position", new_pos.clone());
    assert_eq!(circle.get("position"), Some(new_pos));
}

#[test]
fn circle_property_roundtrip_radius() {
    let mut circle = Circle::new(vec3(0.0, 0.0, 0.0), 50.0, BLUE);
    let new_radius = AnimValue::Float(75.0);
    circle.set("radius", new_radius.clone());
    assert_eq!(circle.get("radius"), Some(new_radius));
}

#[test]
fn circle_property_roundtrip_color() {
    let mut circle = Circle::new(vec3(0.0, 0.0, 0.0), 50.0, BLUE);
    let new_color = AnimValue::Vec4(vec4(1.0, 0.0, 0.0, 1.0));
    circle.set("color", new_color.clone());
    assert_eq!(circle.get("color"), Some(new_color));
}

#[test]
fn circle_get_unknown_property_returns_none() {
    let circle = Circle::new(vec3(0.0, 0.0, 0.0), 50.0, BLUE);
    assert_eq!(circle.get("nonexistent"), None);
}

#[test]
fn circle_property_names_match_getters() {
    let circle = Circle::new(vec3(0.0, 0.0, 0.0), 50.0, BLUE);
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
