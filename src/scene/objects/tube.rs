use std::f32::consts::TAU;

use macroquad::prelude::*;

use crate::scene::traits::{Animatable, BoundingBox, SceneObject};
use crate::scene::value::AnimValue;

const RING_SEGMENTS: usize = 16;

// macroquad's default draw call buffer is 10,000 vertices / 5,000 indices.
// Each ring has RING_SEGMENTS+1 vertices. Each segment between rings has RING_SEGMENTS*6 indices.
// The index limit is binding: 5000 / (16*6) = 52 segments → 53 rings per chunk.
const MAX_RINGS_PER_CHUNK: usize = 5_000 / (RING_SEGMENTS * 6) + 1;

pub struct Tube {
    pub position: Vec3,
    pub points: Vec<Vec3>,
    pub radius: f32,
    pub colors: Vec<Vec4>,
    pub closed: bool,
}

impl Tube {
    const PROPERTY_NAMES: &[&str] = &["position", "radius", "closed"];

    pub fn new(points: Vec<Vec3>, radius: f32, color: Color) -> Self {
        Self {
            position: Vec3::ZERO,
            points,
            radius,
            colors: vec![vec4(color.r, color.g, color.b, color.a)],
            closed: false,
        }
    }

    pub fn with_colors(points: Vec<Vec3>, radius: f32, colors: Vec<Color>) -> Self {
        Self {
            position: Vec3::ZERO,
            points,
            radius,
            colors: colors
                .iter()
                .map(|c| vec4(c.r, c.g, c.b, c.a))
                .collect(),
            closed: false,
        }
    }

    pub fn closed(mut self, val: bool) -> Self {
        self.closed = val;
        self
    }

    /// Sample the gradient color at position `u` (0..1 along the tube).
    /// Colors are evenly spaced and wrap back to the first color.
    fn sample_color(&self, u: f32) -> [u8; 4] {
        let n = self.colors.len();
        if n == 1 {
            return Color::new(
                self.colors[0].x,
                self.colors[0].y,
                self.colors[0].z,
                self.colors[0].w,
            )
            .into();
        }
        let scaled = u * n as f32;
        let idx = (scaled.floor() as usize) % n;
        let frac = scaled.fract();
        let next = (idx + 1) % n;
        let c = self.colors[idx].lerp(self.colors[next], frac);
        Color::new(c.x, c.y, c.z, c.w).into()
    }

    /// Compute tangent at point `i`, using central differences for interior
    /// points and forward/backward for endpoints. Wraps for closed paths.
    fn tangent_at(&self, i: usize) -> Vec3 {
        let n = self.points.len();
        let (prev, next) = if self.closed {
            ((i + n - 1) % n, (i + 1) % n)
        } else if i == 0 {
            (0, 1)
        } else if i == n - 1 {
            (n - 2, n - 1)
        } else {
            (i - 1, i + 1)
        };
        (self.points[next] - self.points[prev]).normalize_or_zero()
    }

    /// Compute an initial perpendicular frame for a given tangent.
    fn initial_frame(tangent: Vec3) -> (Vec3, Vec3) {
        let up = if tangent.dot(Vec3::Y).abs() > 0.9 {
            Vec3::X
        } else {
            Vec3::Y
        };
        let normal = tangent.cross(up).normalize_or_zero();
        let binormal = tangent.cross(normal).normalize_or_zero();
        (normal, binormal)
    }

    /// Propagate a frame from one tangent to the next, keeping it smooth.
    /// Rotates (prev_normal, prev_binormal) so they stay perpendicular to new_tangent.
    fn propagate_frame(
        prev_tangent: Vec3,
        prev_normal: Vec3,
        _prev_binormal: Vec3,
        new_tangent: Vec3,
    ) -> (Vec3, Vec3) {
        let dot = prev_tangent.dot(new_tangent).clamp(-1.0, 1.0);
        let rotated_normal = if dot > 0.9999 {
            // Tangents nearly identical — start from previous normal
            prev_normal
        } else {
            let axis = prev_tangent.cross(new_tangent);
            let axis_len = axis.length();
            if axis_len < 1e-8 {
                // Tangents are opposite — fall back to fresh frame
                return Self::initial_frame(new_tangent);
            }
            let axis = axis / axis_len;
            let angle = dot.acos();
            let rot = Quat::from_axis_angle(axis, angle);
            rot.mul_vec3(prev_normal)
        };
        // Always re-orthogonalize against the new tangent
        let normal = (rotated_normal - new_tangent * rotated_normal.dot(new_tangent)).normalize();
        let binormal = new_tangent.cross(normal).normalize();
        (normal, binormal)
    }

    /// Compute propagated frames for all rings, ensuring smooth transitions.
    fn compute_frames(&self) -> Vec<(Vec3, Vec3, Vec3)> {
        let n = self.points.len();
        let num_rings = if self.closed { n + 1 } else { n };
        let mut frames = Vec::with_capacity(num_rings);

        for ring in 0..num_rings {
            let point_idx = ring % n;
            let tangent = self.tangent_at(point_idx);

            if ring == 0 {
                let (normal, binormal) = Self::initial_frame(tangent);
                frames.push((tangent, normal, binormal));
            } else {
                let (prev_t, prev_n, prev_b) = frames[ring - 1];
                let (normal, binormal) =
                    Self::propagate_frame(prev_t, prev_n, prev_b, tangent);
                frames.push((tangent, normal, binormal));
            }
        }

        frames
    }

    /// Build meshes for the tube, chunked to stay within draw call limits.
    fn build_meshes(&self) -> Vec<Mesh> {
        let n = self.points.len();
        if n < 2 {
            return vec![];
        }

        let frames = self.compute_frames();
        let num_rings = frames.len();

        let mut meshes = Vec::new();

        for chunk_start in (0..num_rings).step_by(MAX_RINGS_PER_CHUNK - 1) {
            let chunk_end = (chunk_start + MAX_RINGS_PER_CHUNK).min(num_rings);
            let chunk_rings = chunk_end - chunk_start;

            let verts_per_ring = RING_SEGMENTS + 1;
            let mut vertices = Vec::with_capacity(chunk_rings * verts_per_ring);
            let mut indices = Vec::with_capacity((chunk_rings - 1) * RING_SEGMENTS * 6);

            for (ring, &(_tangent, normal, binormal)) in frames
                .iter()
                .enumerate()
                .take(chunk_end)
                .skip(chunk_start)
            {
                let point_idx = ring % n;

                let u = ring as f32 / (num_rings - 1) as f32;
                let color = self.sample_color(u);

                for j in 0..=RING_SEGMENTS {
                    let angle = (j as f32 / RING_SEGMENTS as f32) * TAU;
                    let cos_a = angle.cos();
                    let sin_a = angle.sin();

                    let offset = normal * cos_a * self.radius + binormal * sin_a * self.radius;
                    let world_pos = self.points[point_idx] + offset + self.position;

                    let vert_normal = (normal * cos_a + binormal * sin_a).normalize_or_zero();

                    vertices.push(Vertex {
                        position: world_pos,
                        uv: vec2(u, j as f32 / RING_SEGMENTS as f32),
                        color,
                        normal: vec4(vert_normal.x, vert_normal.y, vert_normal.z, 0.0),
                    });
                }
            }

            // Indices connecting adjacent rings within this chunk
            for i in 0..(chunk_rings - 1) {
                let row_len = verts_per_ring as u16;
                for j in 0..RING_SEGMENTS {
                    let cur = (i as u16) * row_len + (j as u16);
                    let next_row = cur + row_len;

                    indices.push(cur);
                    indices.push(next_row);
                    indices.push(cur + 1);

                    indices.push(cur + 1);
                    indices.push(next_row);
                    indices.push(next_row + 1);
                }
            }

            meshes.push(Mesh {
                vertices,
                indices,
                texture: None,
            });
        }

        meshes
    }
}

impl SceneObject for Tube {
    fn draw(&self) {
        for mesh in self.build_meshes() {
            draw_mesh(&mesh);
        }
    }

    fn bounding_box(&self) -> BoundingBox {
        if self.points.is_empty() {
            return BoundingBox {
                min: self.position,
                max: self.position,
            };
        }
        let mut min = Vec3::splat(f32::MAX);
        let mut max = Vec3::splat(f32::MIN);
        for p in &self.points {
            let world = *p + self.position;
            min = min.min(world - Vec3::splat(self.radius));
            max = max.max(world + Vec3::splat(self.radius));
        }
        BoundingBox { min, max }
    }
}

impl Animatable for Tube {
    fn get(&self, property_name: &str) -> Option<AnimValue> {
        match property_name {
            "position" => Some(AnimValue::Vec3(self.position)),
            "radius" => Some(AnimValue::Float(self.radius)),
            "closed" => Some(AnimValue::Bool(self.closed)),
            _ => None,
        }
    }

    fn set(&mut self, property_name: &str, value: AnimValue) {
        match (property_name, value) {
            ("position", AnimValue::Vec3(v)) => self.position = v,
            ("radius", AnimValue::Float(v)) => self.radius = v,
            ("closed", AnimValue::Bool(v)) => self.closed = v,
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

    fn make_straight_tube() -> Tube {
        Tube::new(
            vec![vec3(0.0, 0.0, 0.0), vec3(1.0, 0.0, 0.0), vec3(2.0, 0.0, 0.0)],
            0.5,
            WHITE,
        )
    }

    fn make_triangle_tube() -> Tube {
        Tube::new(
            vec![vec3(0.0, 0.0, 0.0), vec3(1.0, 0.0, 0.0), vec3(0.5, 1.0, 0.0)],
            0.2,
            WHITE,
        )
        .closed(true)
    }

    #[test]
    fn property_round_trip() {
        let mut tube = make_straight_tube();
        for name in Tube::PROPERTY_NAMES {
            let val = tube.get(name).unwrap();
            tube.set(name, val.clone());
            assert_eq!(tube.get(name).unwrap(), val, "round-trip failed for {name}");
        }
    }

    #[test]
    fn set_radius() {
        let mut tube = make_straight_tube();
        tube.set("radius", AnimValue::Float(2.0));
        assert_eq!(tube.radius, 2.0);
    }

    #[test]
    fn set_closed() {
        let mut tube = make_straight_tube();
        assert!(!tube.closed);
        tube.set("closed", AnimValue::Bool(true));
        assert!(tube.closed);
    }

    #[test]
    fn unknown_property_returns_none() {
        let tube = make_straight_tube();
        assert!(tube.get("nonexistent").is_none());
    }

    #[test]
    fn default_closed_is_false() {
        let tube = make_straight_tube();
        assert!(!tube.closed);
    }

    #[test]
    fn default_position_is_zero() {
        let tube = make_straight_tube();
        assert_eq!(tube.position, Vec3::ZERO);
    }

    #[test]
    fn property_names_complete() {
        let tube = make_straight_tube();
        let names = tube.property_names();
        assert_eq!(names.len(), 3);
        assert!(names.contains(&"position"));
        assert!(names.contains(&"radius"));
        assert!(names.contains(&"closed"));
    }

    #[test]
    fn bounding_box_correct() {
        let tube = make_straight_tube();
        let bb = tube.bounding_box();
        assert_eq!(bb.min, vec3(-0.5, -0.5, -0.5));
        assert_eq!(bb.max, vec3(2.5, 0.5, 0.5));
    }

    #[test]
    fn bounding_box_empty_points() {
        let tube = Tube::new(vec![], 1.0, WHITE);
        let bb = tube.bounding_box();
        assert_eq!(bb.min, Vec3::ZERO);
        assert_eq!(bb.max, Vec3::ZERO);
    }

    #[test]
    fn bounding_box_with_position_offset() {
        let mut tube = make_straight_tube();
        tube.position = vec3(10.0, 0.0, 0.0);
        let bb = tube.bounding_box();
        assert_eq!(bb.min, vec3(9.5, -0.5, -0.5));
        assert_eq!(bb.max, vec3(12.5, 0.5, 0.5));
    }

    #[test]
    fn mesh_vertex_count_open() {
        let tube = make_straight_tube();
        let meshes = tube.build_meshes();
        let total_verts: usize = meshes.iter().map(|m| m.vertices.len()).sum();
        // 3 rings × (RING_SEGMENTS + 1) vertices per ring
        assert_eq!(total_verts, 3 * (RING_SEGMENTS + 1));
    }

    #[test]
    fn mesh_index_count_open() {
        let tube = make_straight_tube();
        let meshes = tube.build_meshes();
        let total_indices: usize = meshes.iter().map(|m| m.indices.len()).sum();
        // 2 segments × RING_SEGMENTS × 6 indices per quad
        assert_eq!(total_indices, 2 * RING_SEGMENTS * 6);
    }

    #[test]
    fn mesh_vertex_count_closed() {
        let tube = make_triangle_tube();
        let meshes = tube.build_meshes();
        let total_verts: usize = meshes.iter().map(|m| m.vertices.len()).sum();
        // 4 rings (3 points + 1 wrap) × (RING_SEGMENTS + 1) vertices per ring
        assert_eq!(total_verts, 4 * (RING_SEGMENTS + 1));
    }

    #[test]
    fn mesh_index_count_closed() {
        let tube = make_triangle_tube();
        let meshes = tube.build_meshes();
        let total_indices: usize = meshes.iter().map(|m| m.indices.len()).sum();
        // 3 segments × RING_SEGMENTS × 6 indices per quad
        assert_eq!(total_indices, 3 * RING_SEGMENTS * 6);
    }

    #[test]
    fn empty_points_produces_no_mesh() {
        let tube = Tube::new(vec![], 1.0, WHITE);
        assert!(tube.build_meshes().is_empty());
    }

    #[test]
    fn single_point_produces_no_mesh() {
        let tube = Tube::new(vec![Vec3::ZERO], 1.0, WHITE);
        assert!(tube.build_meshes().is_empty());
    }

    #[test]
    fn two_point_path_works() {
        let tube = Tube::new(vec![Vec3::ZERO, Vec3::X], 0.5, WHITE);
        let meshes = tube.build_meshes();
        let total_verts: usize = meshes.iter().map(|m| m.vertices.len()).sum();
        assert_eq!(total_verts, 2 * (RING_SEGMENTS + 1));
    }

    #[test]
    fn straight_path_rings_are_circular() {
        let tube = Tube::new(
            vec![vec3(0.0, 0.0, 0.0), vec3(1.0, 0.0, 0.0)],
            1.0,
            WHITE,
        );
        let meshes = tube.build_meshes();
        let verts = &meshes[0].vertices;
        // Check first ring: all vertices should be distance 1.0 from the path point (0,0,0)
        for j in 0..RING_SEGMENTS {
            let pos = verts[j].position;
            let dist_from_axis = vec2(pos.y, pos.z).length();
            assert!(
                (dist_from_axis - 1.0).abs() < 1e-5,
                "ring vertex {j} distance from axis: {dist_from_axis}"
            );
        }
    }

    #[test]
    fn closed_builder_method() {
        let tube = Tube::new(vec![Vec3::ZERO, Vec3::X, Vec3::Y], 0.5, WHITE).closed(true);
        assert!(tube.closed);
    }
}
