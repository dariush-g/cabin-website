//! The snowy forest beyond the window, and the storm blowing through it.
//!
//! Branches and fronds are merged into single meshes, so the forest draws in
//! a handful of calls instead of one per needle cluster.

use std::f32::consts::{FRAC_PI_2, TAU};

use bevy::asset::RenderAssetUsages;
use bevy::camera::visibility::NoFrustumCulling;
use bevy::light::NotShadowCaster;
use bevy::math::Affine2;
use bevy::mesh::{Indices, PrimitiveTopology};
use bevy::prelude::*;

use crate::hall::{Kit, glow, hex, unlit};
use crate::meshes;
use crate::textures::{self, Rng};

/// A band of falling snow. Flakes are camera-facing quads rebuilt each frame.
#[derive(Component)]
pub struct SnowLayer {
    near: f32,
    far: f32,
    width: f32,
    height: f32,
    /// Edge length of a flake, in meters.
    size: f32,
    speed: f32,
    flakes: Vec<Vec3>,
    phases: Vec<f32>,
}

#[derive(Resource)]
pub struct Storm {
    pub time: f32,
    pub animate: bool,
}

pub fn spawn(kit: &mut Kit) {
    // One seeded sequence drives the whole exterior, so the forest is the same
    // on every visit.
    let mut rng = Rng::new(941);
    let bark = kit.image(textures::bark(&mut rng), true);
    let trunk_material = kit.materials.add(StandardMaterial {
        base_color_texture: Some(bark),
        perceptual_roughness: 1.0,
        uv_transform: Affine2::from_scale(Vec2::new(2.0, 3.0)),
        ..default()
    });
    let needles = kit.image(textures::needles(&mut rng), false);
    let foliage_material = kit.materials.add(StandardMaterial {
        base_color_texture: Some(needles.clone()),
        alpha_mode: AlphaMode::Mask(0.32),
        double_sided: true,
        cull_mode: None,
        perceptual_roughness: 1.0,
        emissive: glow("#5d6f8f", 0.05),
        emissive_texture: Some(needles),
        emissive_exposure_weight: 1.0,
        ..default()
    });
    let twig_material = kit.mat("#5b5c50", 1.0, 0.0);

    const TREES: usize = 30;
    const TIERS: usize = 14;
    const ARMS: usize = 7;
    let trunk = kit.mesh(meshes::frustum(0.08, 0.2, 1.0, 5, false));
    let mut branches = Vec::with_capacity(TREES * TIERS * ARMS);
    let mut fronds = Vec::with_capacity(TREES * TIERS * ARMS * 2);
    for tree in 0..TREES {
        let x = if tree < 6 { -10.0 - rng.f32() * 5.0 } else { -17.0 - rng.f32() * 22.0 };
        let z = if tree < 6 { -9.0 + tree as f32 * 3.4 } else { -24.0 + rng.f32() * 48.0 };
        let height = 5.5 + rng.f32() * 5.0;
        let width = 1.5 + rng.f32() * 0.8;
        let offset = rng.f32() * TAU;
        // Keep the tapered trunk below the upper needle clusters.
        let trunk_entity = kit.spawn(
            trunk.clone(),
            &trunk_material,
            Transform::from_xyz(x, height * 0.45, z)
                .with_rotation(Quat::from_rotation_y(offset))
                .with_scale(Vec3::new(1.0, height * 0.9, 1.0)),
            None,
        );
        kit.commands.entity(trunk_entity).insert(NotShadowCaster);
        for tier in 0..TIERS {
            let fraction = tier as f32 / TIERS as f32;
            let y = 0.9 + fraction * (height - 0.95) + (rng.f32() - 0.5) * 0.22;
            let radius = width * (1.0 - fraction).powf(0.8) * (0.85 + rng.f32() * 0.3);
            for arm in 0..ARMS {
                let angle = offset + arm as f32 * TAU / ARMS as f32 + tier as f32 * 0.61;
                let length = radius * (0.72 + rng.f32() * 0.35);
                let dip = 0.12 + length * 0.14;
                let (dz, dx) = angle.sin_cos();
                let direction = Vec3::new(dx * length, -dip, dz * length);
                branches.push(Mat4::from_scale_rotation_translation(
                    Vec3::new(1.0, direction.length(), 1.0),
                    Quat::from_rotation_arc(Vec3::Y, direction.normalize()),
                    Vec3::new(x + dx * length / 2.0, y - dip / 2.0, z + dz * length / 2.0),
                ));
                for tilt in [-0.42f32, 0.42] {
                    let rotation = Quat::from_rotation_y(-angle)
                        * Quat::from_rotation_x(-FRAC_PI_2 + tilt + (rng.f32() - 0.5) * 0.2)
                        * Quat::from_rotation_y(-0.12);
                    fronds.push(Mat4::from_scale_rotation_translation(
                        Vec3::new(length * 1.35, 0.48 + length * 0.42, 1.0),
                        rotation,
                        Vec3::new(x + dx * length * 0.58, y - dip * 0.6, z + dz * length * 0.58),
                    ));
                }
            }
        }
    }
    // The daylight is overcast, so the exterior needs no hard shadows.
    let branch_mesh = kit.mesh(meshes::frustum(0.008, 0.025, 1.0, 5, false).merge_transformed(&branches));
    let branch_entity = kit.spawn(branch_mesh, &twig_material, Transform::IDENTITY, None);
    let frond_mesh = kit.mesh(meshes::quad(1.0, 1.0).merge_transformed(&fronds));
    let frond_entity = kit.spawn(frond_mesh, &foliage_material, Transform::IDENTITY, None);
    kit.commands.entity(branch_entity).insert(NotShadowCaster);
    kit.commands.entity(frond_entity).insert(NotShadowCaster);

    let snow_texture = kit.image(textures::snow_ground(&mut rng), true);
    let ground_material = kit.materials.add(StandardMaterial {
        base_color_texture: Some(snow_texture.clone()),
        perceptual_roughness: 1.0,
        emissive: glow("#7f93b8", 0.12),
        emissive_texture: Some(snow_texture),
        emissive_exposure_weight: 1.0,
        uv_transform: Affine2::from_scale(Vec2::new(12.0, 16.0)),
        ..default()
    });
    let ground_mesh = kit.meshes.add(Plane3d::default().mesh().size(55.0, 70.0));
    let ground = kit.spawn(ground_mesh, &ground_material, Transform::from_xyz(-32.0, -0.03, 0.0), None);
    kit.commands.entity(ground).insert(NotShadowCaster);

    // Drifts break up the flat ground at the forest edge.
    let drift_material = kit.materials.add(StandardMaterial {
        base_color: hex("#dce5e7"),
        perceptual_roughness: 1.0,
        emissive: glow("#7f93b8", 0.12),
        emissive_exposure_weight: 1.0,
        ..default()
    });
    let drift_mesh = kit.meshes.add(crate::hall::facet(Sphere::new(1.0).mesh().uv(7, 5)));
    for _ in 0..16 {
        let at = Vec3::new(-7.0 - rng.f32() * 22.0, -0.12, -15.0 + rng.f32() * 30.0);
        let scale = Vec3::new(1.2 + rng.f32() * 2.0, 0.2 + rng.f32() * 0.25, 1.5 + rng.f32() * 3.0);
        let drift = kit.spawn(drift_mesh.clone(), &drift_material, Transform::from_translation(at).with_scale(scale), None);
        kit.commands.entity(drift).insert(NotShadowCaster);
    }

    let flake = kit.image(textures::flake(), false);
    let flake_material = kit.materials.add(StandardMaterial {
        base_color: unlit("#c9d6ee", 0.7),
        base_color_texture: Some(flake),
        unlit: true,
        alpha_mode: AlphaMode::Blend,
        ..default()
    });
    // Close, small flakes; a deep distant band; and a few large flakes
    // blowing right past the glass.
    let layers = [
        (2400, -15.0, -5.3, 27.0, 11.0, 0.075, 1.2),
        (3800, -43.0, -14.0, 62.0, 17.0, 0.1, 0.9),
        (450, -12.0, -5.25, 22.0, 9.0, 0.14, 1.65),
    ];
    for (count, near, far, width, height, size, speed) in layers.into_iter().filter(|_| !crate::flag("nosnow")) {
        let mut flakes = Vec::with_capacity(count);
        let mut phases = Vec::with_capacity(count);
        for _ in 0..count {
            let x = near + rng.f32() * (far - near);
            let y = rng.f32() * height;
            let z = (rng.f32() - 0.5) * width;
            flakes.push(Vec3::new(x, y, z));
            phases.push(rng.f32() * TAU);
        }
        let mesh = kit.meshes.add(flake_quads(count));
        let layer = SnowLayer {
            near,
            far,
            width,
            height,
            // Matches the on-screen size of Three.js attenuated points at a
            // 65° field of view: size × tan(fov / 2).
            size: size * 0.637,
            speed,
            flakes,
            phases,
        };
        kit.commands.spawn((
            Mesh3d(mesh),
            MeshMaterial3d(flake_material.clone()),
            Transform::IDENTITY,
            layer,
            NotShadowCaster,
            NoFrustumCulling,
        ));
    }
}

/// Builds the static parts of a flake mesh: texture coordinates and indices.
fn flake_quads(count: usize) -> Mesh {
    let mut uvs = Vec::with_capacity(count * 4);
    let mut indices = Vec::with_capacity(count * 6);
    for i in 0..count as u32 {
        uvs.extend([[0.0, 0.0], [1.0, 0.0], [1.0, 1.0], [0.0, 1.0]]);
        let first = i * 4;
        indices.extend([first, first + 3, first + 2, first, first + 2, first + 1]);
    }
    Mesh::new(PrimitiveTopology::TriangleList, RenderAssetUsages::default())
        .with_inserted_attribute(Mesh::ATTRIBUTE_POSITION, vec![[0.0f32; 3]; count * 4])
        .with_inserted_attribute(Mesh::ATTRIBUTE_NORMAL, vec![[0.0f32, 0.0, 1.0]; count * 4])
        .with_inserted_attribute(Mesh::ATTRIBUTE_UV_0, uvs)
        .with_inserted_indices(Indices::U32(indices))
}

/// Blows the snow on gusting wind and turns every flake toward the camera.
pub fn storm(
    time: Res<Time>,
    mut storm: ResMut<Storm>,
    camera: Single<&GlobalTransform, With<Camera3d>>,
    mut layers: Query<(&mut SnowLayer, &Mesh3d)>,
    mut meshes: ResMut<Assets<Mesh>>,
) {
    let dt = time.delta_secs().min(0.05);
    let animate = storm.animate;
    if animate {
        storm.time += dt;
    }
    let t = storm.time;
    let gust = 1.7 + (t * 0.63).sin() * 0.9 + (t * 1.41).sin() * 0.35;
    let right = camera.right().as_vec3();
    let up = camera.up().as_vec3();
    for (mut layer, mesh) in &mut layers {
        let layer = &mut *layer;
        if animate {
            for (flake, phase) in layer.flakes.iter_mut().zip(&layer.phases) {
                let flutter = (t * 1.8 + phase).sin();
                flake.x += dt * (gust * 0.75 + flutter * 0.3) * layer.speed;
                flake.y -= dt * (1.3 + phase.cos() * 0.4) * layer.speed;
                flake.z += dt * (gust * 1.8 + flutter * 0.7) * layer.speed;
                if flake.x > layer.far {
                    flake.x = layer.near;
                }
                if flake.y < 0.0 {
                    flake.y += layer.height;
                }
                if flake.z > layer.width / 2.0 {
                    flake.z -= layer.width;
                }
            }
        }
        let (r, u) = (right * layer.size / 2.0, up * layer.size / 2.0);
        let mut positions = Vec::with_capacity(layer.flakes.len() * 4);
        for &flake in &layer.flakes {
            positions.extend([
                (flake - r + u).to_array(),
                (flake + r + u).to_array(),
                (flake + r - u).to_array(),
                (flake - r - u).to_array(),
            ]);
        }
        if let Some(mut mesh) = meshes.get_mut(&mesh.0) {
            mesh.insert_attribute(Mesh::ATTRIBUTE_POSITION, positions);
        }
    }
}
