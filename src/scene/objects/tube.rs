use std::f32::consts::TAU;

use macroquad::prelude::*;

use crate::scene::mesh::MeshBuilder;
use crate::scene::traits::{BoundingBox, SceneObject, animatable};

const RING_SEGMENTS: usize = 16;

pub struct Tube {
    pub position: Vec3,
    pub points: Vec<Vec3>,
    pub radius: f32,
    pub colors: Vec<Vec4>,
    pub closed: bool,
    pub scale: f32,
}

impl Tube {
    pub fn new(points: Vec<Vec3>, radius: f32, color: Color) -> Self {
        Self {
            position: Vec3::ZERO,
            points,
            radius,
            colors: vec![vec4(color.r, color.g, color.b, color.a)],
            closed: false,
            scale: 1.0,
        }
    }

    pub fn with_colors(mut self, colors: Vec<Color>) -> Self {
        self.colors = colors.into_iter().map(|c| vec4(c.r, c.g, c.b, c.a)).collect();
        self
    }

    pub fn with_closed(mut self, val: bool) -> Self {
        self.closed = val;
        self
    }

    /// Sample the gradient color at position `u` (0..1 along the tube).
    /// Colors are evenly spaced and wrap back to the first color.
    fn sample_color(&self, u: f32) -> [u8; 4] {
        let n = self.colors.len();
        if n == 1 {
            return Color::new(self.colors[0].x, self.colors[0].y, self.colors[0].z, self.colors[0].w).into();
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
        let up = if tangent.dot(Vec3::Y).abs() > 0.9 { Vec3::X } else { Vec3::Y };
        let normal = tangent.cross(up).normalize_or_zero();
        let binormal = tangent.cross(normal).normalize_or_zero();
        (normal, binormal)
    }

    /// Propagate a frame from one tangent to the next, keeping it smooth.
    /// Rotates (prev_normal, prev_binormal) so they stay perpendicular to new_tangent.
    fn propagate_frame(prev_tangent: Vec3, prev_normal: Vec3, _prev_binormal: Vec3, new_tangent: Vec3) -> (Vec3, Vec3) {
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
                let (normal, binormal) = Self::propagate_frame(prev_t, prev_n, prev_b, tangent);
                frames.push((tangent, normal, binormal));
            }
        }

        frames
    }

    /// Build the tube surface: one vertex ring per frame, consecutive rings
    /// connected by quad strips. `MeshBuilder` handles draw-call chunking.
    fn build(&self, mb: &mut MeshBuilder) {
        let n = self.points.len();
        if n < 2 {
            return;
        }

        let frames = self.compute_frames();
        let num_rings = frames.len();

        let rings: Vec<Vec<Vertex>> = frames
            .iter()
            .enumerate()
            .map(|(ring, &(_tangent, normal, binormal))| {
                let point_idx = ring % n;

                let u = ring as f32 / (num_rings - 1) as f32;
                let color = self.sample_color(u);

                (0..=RING_SEGMENTS)
                    .map(|j| {
                        let angle = (j as f32 / RING_SEGMENTS as f32) * TAU;
                        let cos_a = angle.cos();
                        let sin_a = angle.sin();

                        let r = self.radius * self.scale;
                        let offset = normal * cos_a * r + binormal * sin_a * r;
                        let world_pos = self.points[point_idx] * self.scale + offset + self.position;

                        let vert_normal = (normal * cos_a + binormal * sin_a).normalize_or_zero();

                        Vertex {
                            position: world_pos,
                            uv: vec2(u, j as f32 / RING_SEGMENTS as f32),
                            color,
                            normal: vec4(vert_normal.x, vert_normal.y, vert_normal.z, 0.0),
                        }
                    })
                    .collect()
            })
            .collect();

        for pair in rings.windows(2) {
            mb.strip(&pair[0], &pair[1]);
        }
    }
}

impl SceneObject for Tube {
    fn draw(&self) {
        let mut mb = MeshBuilder::new();
        self.build(&mut mb);
        mb.draw();
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
        let r = self.radius * self.scale;
        for p in &self.points {
            let world = *p * self.scale + self.position;
            min = min.min(world - Vec3::splat(r));
            max = max.max(world + Vec3::splat(r));
        }
        BoundingBox { min, max }
    }
}

animatable!(Tube {
    position: Vec3,
    radius: Float,
    closed: Bool,
    scale: Float,
});

#[cfg(test)]
mod tests {
    use super::*;
    use crate::scene::traits::test_support::assert_property_roundtrip;

    fn make_straight_tube() -> Tube {
        Tube::new(vec![vec3(0.0, 0.0, 0.0), vec3(1.0, 0.0, 0.0), vec3(2.0, 0.0, 0.0)], 0.5, WHITE)
    }

    fn make_triangle_tube() -> Tube {
        Tube::new(vec![vec3(0.0, 0.0, 0.0), vec3(1.0, 0.0, 0.0), vec3(0.5, 1.0, 0.0)], 0.2, WHITE).with_closed(true)
    }

    fn build_meshes(tube: &Tube) -> Vec<Mesh> {
        let mut mb = MeshBuilder::new();
        tube.build(&mut mb);
        mb.build()
    }

    #[test]
    fn property_round_trip() {
        assert_property_roundtrip(&mut make_straight_tube());
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
        let total_verts: usize = build_meshes(&tube).iter().map(|m| m.vertices.len()).sum();
        // 2 strips (3 rings) × RING_SEGMENTS quads × 4 vertices per quad
        assert_eq!(total_verts, 2 * RING_SEGMENTS * 4);
    }

    #[test]
    fn mesh_index_count_open() {
        let tube = make_straight_tube();
        let total_indices: usize = build_meshes(&tube).iter().map(|m| m.indices.len()).sum();
        // 2 segments × RING_SEGMENTS × 6 indices per quad
        assert_eq!(total_indices, 2 * RING_SEGMENTS * 6);
    }

    #[test]
    fn mesh_vertex_count_closed() {
        let tube = make_triangle_tube();
        let total_verts: usize = build_meshes(&tube).iter().map(|m| m.vertices.len()).sum();
        // 3 strips (3 points + 1 wrap ring) × RING_SEGMENTS quads × 4 vertices per quad
        assert_eq!(total_verts, 3 * RING_SEGMENTS * 4);
    }

    #[test]
    fn mesh_index_count_closed() {
        let tube = make_triangle_tube();
        let total_indices: usize = build_meshes(&tube).iter().map(|m| m.indices.len()).sum();
        // 3 segments × RING_SEGMENTS × 6 indices per quad
        assert_eq!(total_indices, 3 * RING_SEGMENTS * 6);
    }

    #[test]
    fn empty_points_produces_no_mesh() {
        let tube = Tube::new(vec![], 1.0, WHITE);
        assert!(build_meshes(&tube).is_empty());
    }

    #[test]
    fn single_point_produces_no_mesh() {
        let tube = Tube::new(vec![Vec3::ZERO], 1.0, WHITE);
        assert!(build_meshes(&tube).is_empty());
    }

    #[test]
    fn two_point_path_works() {
        let tube = Tube::new(vec![Vec3::ZERO, Vec3::X], 0.5, WHITE);
        let total_verts: usize = build_meshes(&tube).iter().map(|m| m.vertices.len()).sum();
        // 1 strip × RING_SEGMENTS quads × 4 vertices per quad
        assert_eq!(total_verts, RING_SEGMENTS * 4);
    }

    #[test]
    fn straight_path_rings_are_circular() {
        let tube = Tube::new(vec![vec3(0.0, 0.0, 0.0), vec3(1.0, 0.0, 0.0)], 1.0, WHITE);
        let meshes = build_meshes(&tube);
        // A straight tube along X: every vertex should be distance 1.0 from the axis.
        for (j, vert) in meshes.iter().flat_map(|m| m.vertices.iter()).enumerate() {
            let pos = vert.position;
            let dist_from_axis = vec2(pos.y, pos.z).length();
            assert!(
                (dist_from_axis - 1.0).abs() < 1e-5,
                "vertex {j} distance from axis: {dist_from_axis}"
            );
        }
    }

    #[test]
    fn with_closed_builder_method() {
        let tube = Tube::new(vec![Vec3::ZERO, Vec3::X, Vec3::Y], 0.5, WHITE).with_closed(true);
        assert!(tube.closed);
    }
}
