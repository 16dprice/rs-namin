use macroquad::prelude::*;

use crate::scene::Scene;
use crate::scene::objects::{Disk, Line, Text};

// Per-object property round-trip coverage lives inline in each object file
// (see the `property_round_trip` test generated alongside each `animatable!`
// declaration). This file covers Scene container behavior only.

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
    let id2 = scene.add(Line::new(vec3(0.0, 0.0, 0.0), vec3(100.0, 100.0, 0.0), RED));
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

// Regression: a sprite added before a ring at lower Z drew first and its
// transparent texels depth-clipped the ring. The world pass must draw
// back-to-front by Z regardless of insertion order.
#[test]
fn world_draw_order_sorts_back_to_front() {
    let mut scene = Scene::new();
    let front = scene.add(Disk::new(vec3(0.0, 0.0, 0.0), 50.0, BLUE));
    let back = scene.add(Disk::new(vec3(0.0, 0.0, -0.01), 50.0, RED));
    assert_eq!(scene.world_draw_order(), vec![back, front]);
}

#[test]
fn world_draw_order_ties_keep_insertion_order() {
    let mut scene = Scene::new();
    let first = scene.add(Disk::new(vec3(0.0, 0.0, 0.0), 50.0, BLUE));
    let second = scene.add(Disk::new(vec3(1.0, 0.0, 0.0), 50.0, RED));
    assert_eq!(scene.world_draw_order(), vec![first, second]);
}

#[test]
fn world_draw_order_skips_hidden_and_screen_space() {
    let mut scene = Scene::new();
    let visible = scene.add(Disk::new(vec3(0.0, 0.0, 0.0), 50.0, BLUE));
    let hidden = scene.add(Disk::new(vec3(0.0, 0.0, -1.0), 50.0, RED));
    scene.add(Text::new("hud", vec2(0.0, 0.0), 24.0, WHITE));
    scene.set_visible(hidden, false);
    assert_eq!(scene.world_draw_order(), vec![visible]);
}
