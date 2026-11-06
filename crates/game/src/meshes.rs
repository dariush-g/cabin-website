//! Procedural mesh builders.
//!
//! Bevy's primitives cover spheres and tori, but the hall also needs cones
//! with open ends, partial spheres, and boxes whose faces map a whole texture
//! upright, the way the canvas-drawn signs and rug expect. Texture
//! coordinates follow Bevy's convention: v = 0 is the top of the image.

use std::f32::consts::TAU;

use bevy::asset::RenderAssetUsages;
use bevy::mesh::{Indices, PrimitiveTopology};
use bevy::prelude::*;

#[derive(Default, Clone)]
pub struct MeshData {
    pub positions: Vec<[f32; 3]>,
    pub normals: Vec<[f32; 3]>,
    pub uvs: Vec<[f32; 2]>,
    pub indices: Vec<u32>,
}

impl MeshData {
    fn vertex(&mut self, position: Vec3, normal: Vec3, uv: Vec2) -> u32 {
        self.positions.push(position.to_array());
        self.normals.push(normal.to_array());
        self.uvs.push(uv.to_array());
        (self.positions.len() - 1) as u32
    }

    /// Adds a rectangle. Seen from the side `normal` points to, `u` runs left
    /// to right across the image and `v` runs from its top edge to the bottom.
    fn face(&mut self, center: Vec3, u: Vec3, v: Vec3, normal: Vec3) {
        let corners = [(0.0, 0.0), (1.0, 0.0), (1.0, 1.0), (0.0, 1.0)];
        let first = self.positions.len() as u32;
        for (s, t) in corners {
            self.vertex(center + u * (s - 0.5) + v * (t - 0.5), normal, Vec2::new(s, t));
        }
        self.indices
            .extend([first, first + 3, first + 2, first, first + 2, first + 1]);
    }

    /// Copies `self` once per transform into a single mesh, so thousands of
    /// branches render in one draw call.
    pub fn merge_transformed(&self, transforms: &[Mat4]) -> MeshData {
        let mut merged = MeshData::default();
        for transform in transforms {
            let offset = merged.positions.len() as u32;
            let normal_matrix = Mat3::from_mat4(*transform).inverse().transpose();
            merged.positions.extend(
                self.positions
                    .iter()
                    .map(|p| transform.transform_point3(Vec3::from(*p)).to_array()),
            );
            merged.normals.extend(
                self.normals
                    .iter()
                    .map(|n| (normal_matrix * Vec3::from(*n)).normalize_or_zero().to_array()),
            );
            merged.uvs.extend_from_slice(&self.uvs);
            merged.indices.extend(self.indices.iter().map(|i| i + offset));
        }
        merged
    }

    pub fn build(self) -> Mesh {
        Mesh::new(PrimitiveTopology::TriangleList, RenderAssetUsages::default())
            .with_inserted_attribute(Mesh::ATTRIBUTE_POSITION, self.positions)
            .with_inserted_attribute(Mesh::ATTRIBUTE_NORMAL, self.normals)
            .with_inserted_attribute(Mesh::ATTRIBUTE_UV_0, self.uvs)
            .with_inserted_indices(Indices::U32(self.indices))
    }
}

/// A box whose every face shows the full texture upright. The top face reads
/// with the image's top edge toward -Z.
pub fn cuboid(width: f32, height: f32, depth: f32) -> MeshData {
    let mut data = MeshData::default();
    let (w, h, d) = (width, height, depth);
    let half = Vec3::new(w, h, d) / 2.0;
    data.face(Vec3::Z * half.z, Vec3::X * w, Vec3::NEG_Y * h, Vec3::Z);
    data.face(Vec3::NEG_Z * half.z, Vec3::NEG_X * w, Vec3::NEG_Y * h, Vec3::NEG_Z);
    data.face(Vec3::X * half.x, Vec3::NEG_Z * d, Vec3::NEG_Y * h, Vec3::X);
    data.face(Vec3::NEG_X * half.x, Vec3::Z * d, Vec3::NEG_Y * h, Vec3::NEG_X);
    data.face(Vec3::Y * half.y, Vec3::X * w, Vec3::Z * d, Vec3::Y);
    data.face(Vec3::NEG_Y * half.y, Vec3::X * w, Vec3::NEG_Z * d, Vec3::NEG_Y);
    data
}

/// A flat rectangle in the XY plane facing +Z.
pub fn quad(width: f32, height: f32) -> MeshData {
    let mut data = MeshData::default();
    data.face(Vec3::ZERO, Vec3::X * width, Vec3::NEG_Y * height, Vec3::Z);
    data
}

/// A cylinder or cone along Y, centered on the origin, optionally without caps.
pub fn frustum(radius_top: f32, radius_bottom: f32, height: f32, segments: u32, open: bool) -> MeshData {
    let mut data = MeshData::default();
    let slope = (radius_bottom - radius_top) / height;
    for row in 0..=1u32 {
        let v = row as f32;
        let radius = radius_top + v * (radius_bottom - radius_top);
        let y = height / 2.0 - v * height;
        for column in 0..=segments {
            let u = column as f32 / segments as f32;
            let (sin, cos) = (u * TAU).sin_cos();
            data.vertex(
                Vec3::new(radius * sin, y, radius * cos),
                Vec3::new(sin, slope, cos).normalize(),
                Vec2::new(u, v),
            );
        }
    }
    let stride = segments + 1;
    for column in 0..segments {
        let (a, d) = (column, column + 1);
        let (b, c) = (a + stride, d + stride);
        data.indices.extend([a, b, d, b, c, d]);
    }
    if !open {
        for (top, radius) in [(true, radius_top), (false, radius_bottom)] {
            let y = if top { height / 2.0 } else { -height / 2.0 };
            let normal = if top { Vec3::Y } else { Vec3::NEG_Y };
            let center = data.vertex(Vec3::new(0.0, y, 0.0), normal, Vec2::splat(0.5));
            for column in 0..=segments {
                let (sin, cos) = (column as f32 / segments as f32 * TAU).sin_cos();
                data.vertex(
                    Vec3::new(radius * sin, y, radius * cos),
                    normal,
                    Vec2::new(0.5 + sin * 0.5, 0.5 - cos * 0.5),
                );
            }
            for column in 0..segments {
                let (i, j) = (center + 1 + column, center + 2 + column);
                if top {
                    data.indices.extend([center, i, j]);
                } else {
                    data.indices.extend([center, j, i]);
                }
            }
        }
    }
    data
}

/// A sphere from the top pole down to `theta_length` radians.
pub fn sphere_segment(radius: f32, width_segments: u32, height_segments: u32, theta_length: f32) -> MeshData {
    let mut data = MeshData::default();
    for row in 0..=height_segments {
        let v = row as f32 / height_segments as f32;
        let theta = v * theta_length;
        for column in 0..=width_segments {
            let u = column as f32 / width_segments as f32;
            let phi = u * TAU;
            let normal = Vec3::new(-phi.cos() * theta.sin(), theta.cos(), phi.sin() * theta.sin());
            data.vertex(normal * radius, normal, Vec2::new(u, v));
        }
    }
    let stride = width_segments + 1;
    for row in 0..height_segments {
        for column in 0..width_segments {
            let a = row * stride + column + 1;
            let b = row * stride + column;
            let c = (row + 1) * stride + column;
            let d = (row + 1) * stride + column + 1;
            if row > 0 {
                data.indices.extend([a, b, d]);
            }
            if row < height_segments - 1 || theta_length < std::f32::consts::PI {
                data.indices.extend([b, c, d]);
            }
        }
    }
    data
}
