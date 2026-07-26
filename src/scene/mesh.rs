//! Shared mesh construction for scene objects.
//!
//! macroquad silently clamps any draw call exceeding its buffer limits
//! (10,000 vertices / 5,000 indices), which drops geometry with only a
//! console warning. `MeshBuilder` owns those limits in one place: append
//! primitives and it splits into multiple meshes automatically, at
//! primitive granularity, whenever the next primitive wouldn't fit.

use macroquad::models::Vertex;
use macroquad::prelude::*;

pub const MAX_VERTICES_PER_MESH: usize = 10_000;
pub const MAX_INDICES_PER_MESH: usize = 5_000;

/// Normal for flat scene objects on the XY plane.
pub const FLAT_NORMAL: Vec4 = Vec4::new(0.0, 0.0, 1.0, 0.0);

/// Convert a `Vec4` color (as stored by scene objects) to vertex color bytes.
pub fn color_bytes(color: Vec4) -> [u8; 4] {
    Color::new(color.x, color.y, color.z, color.w).into()
}

/// Build a vertex for a flat XY-plane object.
pub fn flat_vertex(position: Vec3, uv: Vec2, color: [u8; 4]) -> Vertex {
    Vertex {
        position,
        uv,
        color,
        normal: FLAT_NORMAL,
    }
}

pub struct MeshBuilder {
    texture: Option<Texture2D>,
    vertices: Vec<Vertex>,
    indices: Vec<u16>,
    meshes: Vec<Mesh>,
}

impl MeshBuilder {
    pub fn new() -> Self {
        Self {
            texture: None,
            vertices: Vec::new(),
            indices: Vec::new(),
            meshes: Vec::new(),
        }
    }

    pub fn with_texture(texture: Texture2D) -> Self {
        Self {
            texture: Some(texture),
            ..Self::new()
        }
    }

    /// Append a primitive: vertices plus indices relative to the primitive's
    /// first vertex. The primitive lands in a single mesh; a new mesh is
    /// started first if it wouldn't fit in the current one.
    ///
    /// # Panics
    /// Panics if a single primitive alone exceeds the per-draw-call limits.
    pub fn primitive(&mut self, vertices: &[Vertex], rel_indices: &[u16]) {
        assert!(
            vertices.len() <= MAX_VERTICES_PER_MESH && rel_indices.len() <= MAX_INDICES_PER_MESH,
            "primitive exceeds macroquad draw-call limits ({} vertices / {} indices)",
            vertices.len(),
            rel_indices.len(),
        );
        if self.vertices.len() + vertices.len() > MAX_VERTICES_PER_MESH || self.indices.len() + rel_indices.len() > MAX_INDICES_PER_MESH {
            self.flush();
        }
        let base = self.vertices.len() as u16;
        self.vertices.extend_from_slice(vertices);
        self.indices.extend(rel_indices.iter().map(|i| base + i));
    }

    /// Append a quad from four corners in winding order (two triangles).
    pub fn quad(&mut self, corners: [Vertex; 4]) {
        self.primitive(&corners, &[0, 1, 2, 0, 2, 3]);
    }

    /// Append a triangle fan: center connected to consecutive rim vertices.
    /// `closed` connects the last rim vertex back to the first.
    pub fn fan(&mut self, center: Vertex, rim: &[Vertex], closed: bool) {
        if rim.len() < 2 {
            return;
        }
        let mut vertices = Vec::with_capacity(rim.len() + 1);
        vertices.push(center);
        vertices.extend_from_slice(rim);

        let segments = if closed { rim.len() } else { rim.len() - 1 };
        let mut indices = Vec::with_capacity(segments * 3);
        for i in 0..segments {
            let next = (i + 1) % rim.len();
            indices.extend_from_slice(&[0, (i + 1) as u16, (next + 1) as u16]);
        }
        self.primitive(&vertices, &indices);
    }

    /// Append quads connecting two vertex rows of equal length: row a to
    /// row b, one quad per adjacent column pair.
    pub fn strip(&mut self, row_a: &[Vertex], row_b: &[Vertex]) {
        assert_eq!(row_a.len(), row_b.len(), "strip rows must have equal length");
        for j in 0..row_a.len().saturating_sub(1) {
            self.quad([row_a[j], row_a[j + 1], row_b[j + 1], row_b[j]]);
        }
    }

    fn flush(&mut self) {
        if self.vertices.is_empty() {
            return;
        }
        self.meshes.push(Mesh {
            vertices: std::mem::take(&mut self.vertices),
            indices: std::mem::take(&mut self.indices),
            texture: self.texture.clone(),
        });
    }

    /// Finish building and return the accumulated meshes.
    pub fn build(mut self) -> Vec<Mesh> {
        self.flush();
        self.meshes
    }

    /// Finish building and draw all meshes.
    pub fn draw(self) {
        for mesh in self.build() {
            draw_mesh(&mesh);
        }
    }
}

impl Default for MeshBuilder {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn v(x: f32) -> Vertex {
        flat_vertex(vec3(x, 0.0, 0.0), vec2(0.0, 0.0), [255; 4])
    }

    #[test]
    fn single_quad_is_one_mesh() {
        let mut mb = MeshBuilder::new();
        mb.quad([v(0.0), v(1.0), v(2.0), v(3.0)]);
        let meshes = mb.build();
        assert_eq!(meshes.len(), 1);
        assert_eq!(meshes[0].vertices.len(), 4);
        assert_eq!(meshes[0].indices, vec![0, 1, 2, 0, 2, 3]);
    }

    #[test]
    fn empty_builder_builds_no_meshes() {
        assert!(MeshBuilder::new().build().is_empty());
    }

    #[test]
    fn chunks_when_indices_exceed_limit() {
        // Each quad is 4 vertices / 6 indices; 5000/6 = 833 quads per mesh.
        let mut mb = MeshBuilder::new();
        for _ in 0..834 {
            mb.quad([v(0.0), v(1.0), v(2.0), v(3.0)]);
        }
        let meshes = mb.build();
        assert_eq!(meshes.len(), 2);
        assert_eq!(meshes[0].indices.len(), 833 * 6);
        assert_eq!(meshes[1].vertices.len(), 4);
    }

    #[test]
    fn chunks_when_vertices_exceed_limit() {
        // 100-vertex primitives with few indices: 10000/100 = 100 per mesh.
        let vertices: Vec<Vertex> = (0..100).map(|i| v(i as f32)).collect();
        let mut mb = MeshBuilder::new();
        for _ in 0..101 {
            mb.primitive(&vertices, &[0, 1, 2]);
        }
        let meshes = mb.build();
        assert_eq!(meshes.len(), 2);
        assert_eq!(meshes[0].vertices.len(), 10_000);
        assert_eq!(meshes[1].vertices.len(), 100);
    }

    #[test]
    fn indices_are_rebased_per_mesh() {
        let mut mb = MeshBuilder::new();
        mb.quad([v(0.0), v(1.0), v(2.0), v(3.0)]);
        mb.quad([v(4.0), v(5.0), v(6.0), v(7.0)]);
        let meshes = mb.build();
        assert_eq!(meshes[0].indices, vec![0, 1, 2, 0, 2, 3, 4, 5, 6, 4, 6, 7]);
    }

    #[test]
    fn fan_closed_wraps() {
        let mut mb = MeshBuilder::new();
        mb.fan(v(0.0), &[v(1.0), v(2.0), v(3.0)], true);
        let meshes = mb.build();
        assert_eq!(meshes[0].vertices.len(), 4);
        // 3 triangles, last one wraps back to rim[0]
        assert_eq!(meshes[0].indices, vec![0, 1, 2, 0, 2, 3, 0, 3, 1]);
    }

    #[test]
    fn fan_open_does_not_wrap() {
        let mut mb = MeshBuilder::new();
        mb.fan(v(0.0), &[v(1.0), v(2.0), v(3.0)], false);
        let meshes = mb.build();
        assert_eq!(meshes[0].indices, vec![0, 1, 2, 0, 2, 3]);
    }

    #[test]
    fn strip_connects_rows() {
        let mut mb = MeshBuilder::new();
        mb.strip(&[v(0.0), v(1.0), v(2.0)], &[v(3.0), v(4.0), v(5.0)]);
        let meshes = mb.build();
        // 2 quads of 4 vertices each
        assert_eq!(meshes[0].vertices.len(), 8);
        assert_eq!(meshes[0].indices.len(), 12);
    }

    #[test]
    #[should_panic(expected = "primitive exceeds macroquad draw-call limits")]
    fn oversized_primitive_panics() {
        let vertices: Vec<Vertex> = (0..10_001).map(|i| v(i as f32)).collect();
        MeshBuilder::new().primitive(&vertices, &[0, 1, 2]);
    }
}
