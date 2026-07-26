use macroquad::prelude::*;

use crate::scene::polyline::{self, LineSegment, PolylineStyle, PolylineTransform, draw_polyline_mesh};
use crate::scene::traits::{Animatable, BoundingBox, SceneObject};
use crate::scene::value::AnimValue;

pub struct Polyline {
    pub segments: Vec<LineSegment>,
    pub position: Vec3,
    pub color: Vec4,
    pub scale: f32,
    pub progress: f32,
    pub line_width: f32,
    colors: Vec<Vec4>,
}

impl Polyline {
    const PROPERTY_NAMES: &[&str] = &["position", "color", "scale", "progress", "line_width"];

    pub fn new(segments: Vec<LineSegment>, color: Color) -> Self {
        Self {
            segments,
            position: Vec3::ZERO,
            color: vec4(color.r, color.g, color.b, color.a),
            scale: 1.0,
            progress: 1.0,
            line_width: 0.02,
            colors: Vec::new(),
        }
    }

    pub fn with_colors(mut self, colors: Vec<Color>) -> Self {
        self.colors = colors.into_iter().map(|c| vec4(c.r, c.g, c.b, c.a)).collect();
        self
    }
}

impl SceneObject for Polyline {
    fn draw(&self) {
        let visible = polyline::take_progress(&self.segments, self.progress);
        draw_polyline_mesh(
            &visible,
            &PolylineStyle {
                line_width: self.line_width,
                color: self.color,
                colors: &self.colors,
                color_total: self.segments.len(),
            },
            &PolylineTransform {
                position: self.position,
                scale: self.scale,
            },
        );
    }

    fn bounding_box(&self) -> BoundingBox {
        let visible = polyline::take_progress(&self.segments, self.progress);
        if visible.is_empty() {
            return BoundingBox {
                min: self.position,
                max: self.position,
            };
        }

        let mut min_x = f32::MAX;
        let mut min_y = f32::MAX;
        let mut max_x = f32::MIN;
        let mut max_y = f32::MIN;

        for seg in &visible {
            min_x = min_x.min(seg.start.x).min(seg.end.x);
            min_y = min_y.min(seg.start.y).min(seg.end.y);
            max_x = max_x.max(seg.start.x).max(seg.end.x);
            max_y = max_y.max(seg.start.y).max(seg.end.y);
        }

        BoundingBox {
            min: vec3(
                min_x * self.scale + self.position.x,
                min_y * self.scale + self.position.y,
                self.position.z,
            ),
            max: vec3(
                max_x * self.scale + self.position.x,
                max_y * self.scale + self.position.y,
                self.position.z,
            ),
        }
    }
}

impl Animatable for Polyline {
    fn get(&self, property_name: &str) -> Option<AnimValue> {
        match property_name {
            "position" => Some(AnimValue::Vec3(self.position)),
            "color" => Some(AnimValue::Vec4(self.color)),
            "scale" => Some(AnimValue::Float(self.scale)),
            "progress" => Some(AnimValue::Float(self.progress)),
            "line_width" => Some(AnimValue::Float(self.line_width)),
            _ => None,
        }
    }

    fn set(&mut self, property_name: &str, value: AnimValue) {
        match (property_name, value) {
            ("position", AnimValue::Vec3(v)) => self.position = v,
            ("color", AnimValue::Vec4(v)) => self.color = v,
            ("scale", AnimValue::Float(v)) => self.scale = v,
            ("progress", AnimValue::Float(v)) => self.progress = v,
            ("line_width", AnimValue::Float(v)) => self.line_width = v,
            _ => {}
        }
    }

    fn property_names(&self) -> &[&str] {
        Self::PROPERTY_NAMES
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn seg(x0: f32, y0: f32, x1: f32, y1: f32) -> LineSegment {
        LineSegment {
            start: vec2(x0, y0),
            end: vec2(x1, y1),
        }
    }

    fn make_polyline() -> Polyline {
        Polyline::new(vec![seg(0.0, 0.0, 1.0, 0.0), seg(1.0, 0.0, 1.0, 1.0)], WHITE)
    }

    #[test]
    fn property_round_trip() {
        let mut pl = make_polyline();
        for name in Polyline::PROPERTY_NAMES {
            let val = pl.get(name).unwrap();
            pl.set(name, val.clone());
            assert_eq!(pl.get(name).unwrap(), val, "round-trip failed for {name}");
        }
    }

    #[test]
    fn unknown_property_returns_none() {
        let pl = make_polyline();
        assert!(pl.get("nonexistent").is_none());
    }

    #[test]
    fn progress_zero_empty_bbox() {
        let mut pl = make_polyline();
        pl.progress = 0.0;
        let bb = pl.bounding_box();
        assert_eq!(bb.min, pl.position);
        assert_eq!(bb.max, pl.position);
    }

    #[test]
    fn bounding_box_covers_segments() {
        let pl = make_polyline();
        let bb = pl.bounding_box();
        assert!(bb.max.x >= 1.0);
        assert!(bb.max.y >= 1.0);
    }
}
