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

    /// Adds another mesh's triangles to this one.
    pub fn append(&mut self, other: MeshData) {
        let offset = self.positions.len() as u32;
        self.positions.extend(other.positions);
        self.normals.extend(other.normals);
        self.uvs.extend(other.uvs);
        self.indices.extend(other.indices.into_iter().map(|i| i + offset));
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

/// A box whose texture repeats every `tile` metres on every face, so stone
/// keeps the same scale however the box is proportioned.
pub fn tiled_cuboid(width: f32, height: f32, depth: f32, tile: f32) -> MeshData {
    let mut data = cuboid(width, height, depth);
    let spans = [(width, height), (width, height), (depth, height), (depth, height), (width, depth), (width, depth)];
    for (face, (a, b)) in spans.into_iter().enumerate() {
        for uv in &mut data.uvs[face * 4..face * 4 + 4] {
            uv[0] *= a / tile;
            uv[1] *= b / tile;
        }
    }
    data
}

/// A box for moulding: the texture repeats every `tile` metres along its
/// length (x) but spans its full width, so a profile drawn across the image
/// stays intact.
pub fn moulding(length: f32, width: f32, depth: f32, tile: f32) -> MeshData {
    let mut data = cuboid(length, width, depth);
    for face in [0, 1, 4, 5] {
        for uv in &mut data.uvs[face * 4..face * 4 + 4] {
            uv[0] *= length / tile;
        }
    }
    data
}

/// A triangle in the XY plane, base on y = 0 and apex at (0, rise), facing
/// both ways: the gable end of a pitched roof.
pub fn gable(half_width: f32, rise: f32) -> MeshData {
    let mut data = MeshData::default();
    for normal in [Vec3::Z, Vec3::NEG_Z] {
        let first = data.positions.len() as u32;
        data.vertex(Vec3::new(-half_width, 0.0, 0.0), normal, Vec2::new(0.0, 1.0));
        data.vertex(Vec3::new(half_width, 0.0, 0.0), normal, Vec2::new(1.0, 1.0));
        data.vertex(Vec3::new(0.0, rise, 0.0), normal, Vec2::new(0.5, 0.0));
        if normal.z > 0.0 {
            data.indices.extend([first, first + 1, first + 2]);
        } else {
            data.indices.extend([first, first + 2, first + 1]);
        }
    }
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

/// A round log lying along X, centred on the origin, with smooth normals and
/// capped ends. `u` runs along its length, one texture repeat every `tile`
/// metres starting at `u0`; `v` wraps once around it.
pub fn log(length: f32, radius: f32, segments: u32, tile: f32, u0: f32) -> MeshData {
    let mut data = MeshData::default();
    let half = length / 2.0;
    let (u_start, u_end) = (u0, u0 + length / tile);
    for column in 0..=segments {
        let v = column as f32 / segments as f32;
        let (sin, cos) = (v * TAU).sin_cos();
        let normal = Vec3::new(0.0, sin, cos);
        data.vertex(Vec3::new(-half, radius * sin, radius * cos), normal, Vec2::new(u_start, v));
        data.vertex(Vec3::new(half, radius * sin, radius * cos), normal, Vec2::new(u_end, v));
    }
    for column in 0..segments {
        let (a0, a1, b0, b1) = (column * 2, column * 2 + 1, column * 2 + 2, column * 2 + 3);
        data.indices.extend([a0, a1, b1, a0, b1, b0]);
    }
    // End caps sample one spot of the texture: they sit inside corners and
    // casings, where nobody looks at them closely.
    for (x, normal) in [(half, Vec3::X), (-half, Vec3::NEG_X)] {
        let uv = Vec2::new(u_start, 0.5);
        let center = data.vertex(Vec3::new(x, 0.0, 0.0), normal, uv);
        for column in 0..=segments {
            let (sin, cos) = (column as f32 / segments as f32 * TAU).sin_cos();
            data.vertex(Vec3::new(x, radius * sin, radius * cos), normal, uv);
        }
        for column in 0..segments {
            let (i, j) = (center + 1 + column, center + 2 + column);
            if x > 0.0 {
                data.indices.extend([center, j, i]);
            } else {
                data.indices.extend([center, i, j]);
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

/// A box with rounded edges and corners, for cushions. Faces that point along
/// the box's thinnest axis puff out by up to `bulge` at their centre, the way
/// stuffed fabric does. Normals are smooth, so it reads as soft.
pub fn cushion(size: Vec3, radius: f32, bulge: f32, segments: u32) -> MeshData {
    let mut data = MeshData::default();
    let half = size / 2.0;
    let radius = radius.min(half.min_element());
    let inner = half - Vec3::splat(radius);
    // The axis the cushion is thin along, and the two it spreads across.
    let thin = (0..3).min_by(|&a, &b| half[a].total_cmp(&half[b])).unwrap();
    let (across, along) = ((thin + 1) % 3, (thin + 2) % 3);
    // (u, v, normal) per face, matching `cuboid`'s orientation.
    let faces = [
        (Vec3::X, Vec3::NEG_Y, Vec3::Z),
        (Vec3::NEG_X, Vec3::NEG_Y, Vec3::NEG_Z),
        (Vec3::NEG_Z, Vec3::NEG_Y, Vec3::X),
        (Vec3::Z, Vec3::NEG_Y, Vec3::NEG_X),
        (Vec3::X, Vec3::Z, Vec3::Y),
        (Vec3::X, Vec3::NEG_Z, Vec3::NEG_Y),
    ];
    let stride = segments + 1;
    for (u, v, normal) in faces {
        let first = data.positions.len() as u32;
        for j in 0..=segments {
            for i in 0..=segments {
                let (s, t) = (i as f32 / segments as f32, j as f32 / segments as f32);
                let on_box = normal * half + u * half * (2.0 * s - 1.0) + v * half * (2.0 * t - 1.0);
                let core = on_box.clamp(-inner, inner);
                let mut n = (on_box - core).try_normalize().unwrap_or(normal);
                let mut position = core + n * radius;
                // Puff the broad faces out: height h = bulge·(1 − a²)(1 − b²).
                let a = (position[across] / half[across]).clamp(-1.0, 1.0);
                let b = (position[along] / half[along]).clamp(-1.0, 1.0);
                let puff = bulge * (1.0 - a * a) * (1.0 - b * b);
                position[thin] += position[thin].signum() * puff;
                let facing = n[thin].abs();
                n[across] += facing * bulge * 2.0 * a / half[across] * (1.0 - b * b);
                n[along] += facing * bulge * 2.0 * b / half[along] * (1.0 - a * a);
                data.vertex(position, n.normalize(), Vec2::new(s, t));
            }
        }
        for j in 0..segments {
            for i in 0..segments {
                let a = first + j * stride + i;
                let (b, c, d) = (a + 1, a + stride + 1, a + stride);
                data.indices.extend([a, d, c, a, c, b]);
            }
        }
    }
    data
}
