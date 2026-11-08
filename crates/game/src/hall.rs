//! The entrance hall: architecture, furnishings, doors and lighting.
//!
//! Light intensities are carried over from the Three.js scene: candela
//! become lumens (× 4π) and rectangular luminance becomes lumens (× area × π).
//! The camera's exposure is set so the two renderers agree.

use std::f32::consts::{FRAC_PI_2, PI, TAU};

use bevy::light::NotShadowCaster;
use bevy::math::Affine2;
use bevy::prelude::*;

use crate::meshes::{self, MeshData};
use crate::textures::{self, Rng};
use crate::winter;

/// Converts a Three.js candela intensity to Bevy's lumens.
pub const CANDELA: f32 = 4.0 * PI;

/// Three.js scaled every pixel by its exposure (1.15) over 0.6 before ACES
/// tone mapping. The camera's exposure applies this to lit surfaces; unlit
/// ones are multiplied by it directly.
pub const DISPLAY_GAIN: f32 = 1.15 / 0.6;

pub fn hex(code: &str) -> Color {
    Color::Srgba(Srgba::hex(code).expect("valid hex color"))
}

/// An unlit surface color, brightened to match lit surfaces on screen.
pub fn unlit(code: &str, alpha: f32) -> Color {
    let color = LinearRgba::from(Srgba::hex(code).expect("valid hex color"));
    Color::LinearRgba(LinearRgba::new(
        color.red * DISPLAY_GAIN,
        color.green * DISPLAY_GAIN,
        color.blue * DISPLAY_GAIN,
        alpha,
    ))
}

pub fn glow(code: &str, strength: f32) -> LinearRgba {
    let color = LinearRgba::from(Srgba::hex(code).expect("valid hex color"));
    LinearRgba::rgb(color.red * strength, color.green * strength, color.blue * strength)
}

/// Gives a mesh flat, per-triangle normals, for the low-poly look.
pub fn facet(mesh: Mesh) -> Mesh {
    mesh.with_duplicated_vertices().with_computed_flat_normals()
}

/// The runner rug's footprint (see `setup`), where footsteps fall soft.
const RUG: (f32, f32, f32, f32) = (0.0, -1.2, 3.0, 9.0);

/// Half the room's width (x) and depth (z), and the height of its walls.
pub const HALF_WIDTH: f32 = 7.0;
pub const HALF_DEPTH: f32 = 9.0;
pub const HEIGHT: f32 = 5.0;
/// Height of the roof's ridge, which runs front to back. The roof's logs
/// hang below this line.
pub const RIDGE: f32 = 8.0;
/// Radius of the logs laid side by side to form the roof.
const ROOF_LOG: f32 = 0.2;
/// Centre height of the ridge beam, just under the roof logs.
const RIDGE_BEAM: f32 = RIDGE - 2.0 * ROOF_LOG - 0.3;
/// How far the eaves reach past the side walls.
const OVERHANG: f32 = 1.0;

pub fn on_rug(x: f32, z: f32) -> bool {
    let (cx, cz, width, depth) = RUG;
    (x - cx).abs() < width / 2.0 && (z - cz).abs() < depth / 2.0
}

/// An axis-aligned footprint the visitor can't walk into.
#[derive(Clone, Copy)]
pub struct Collider {
    pub x: f32,
    pub z: f32,
    pub half_width: f32,
    pub half_depth: f32,
}

#[derive(Resource, Default)]
pub struct Colliders(pub Vec<Collider>);

impl Colliders {
    /// Keeps the visitor's eye about a shoulder's width from furniture.
    fn block(&mut self, x: f32, z: f32, width: f32, depth: f32) {
        self.0.push(Collider { x, z, half_width: width / 2.0 + 0.23, half_depth: depth / 2.0 + 0.23 });
    }

    pub fn allows(&self, x: f32, z: f32) -> bool {
        x.abs() < HALF_WIDTH - 0.5
            && z.abs() < HALF_DEPTH - 0.6
            && !self
                .0
                .iter()
                .any(|c| (x - c.x).abs() < c.half_width && (z - c.z).abs() < c.half_depth)
    }
}

#[derive(Clone, Debug)]
pub enum Action {
    Door(&'static str),
    Lamp,
    Note,
}

/// Something the visitor can use when facing it. `half` is the half-size of
/// the box, in the entity's local space, that the view ray must hit.
#[derive(Component, Clone)]
pub struct Target {
    pub label: String,
    pub action: Action,
    pub half: Vec3,
}

/// A light that belongs to the table lamp, with its power when switched on.
#[derive(Component)]
pub struct LampLight(pub f32);

#[derive(Resource)]
pub struct Lamp {
    pub on: bool,
    pub shade: Handle<StandardMaterial>,
    pub shade_glow: LinearRgba,
    pub bulb: Entity,
}

/// Spawning helpers shared by the hall and the winter exterior.
pub struct Kit<'a, 'w, 's> {
    pub commands: &'a mut Commands<'w, 's>,
    pub meshes: &'a mut Assets<Mesh>,
    pub materials: &'a mut Assets<StandardMaterial>,
    pub images: &'a mut Assets<Image>,
}

impl Kit<'_, '_, '_> {
    pub fn mat(&mut self, color: &str, roughness: f32, metallic: f32) -> Handle<StandardMaterial> {
        self.materials.add(StandardMaterial {
            base_color: hex(color),
            perceptual_roughness: roughness,
            metallic,
            ..default()
        })
    }

    pub fn image(&mut self, canvas: textures::Canvas, repeat: bool) -> Handle<Image> {
        self.images.add(canvas.into_image(repeat))
    }

    pub fn mesh(&mut self, data: MeshData) -> Handle<Mesh> {
        self.meshes.add(facet(data.build()))
    }

    pub fn spawn(
        &mut self,
        mesh: Handle<Mesh>,
        material: &Handle<StandardMaterial>,
        transform: Transform,
        parent: Option<Entity>,
    ) -> Entity {
        let mut entity = self
            .commands
            .spawn((Mesh3d(mesh), MeshMaterial3d(material.clone()), transform));
        if let Some(parent) = parent {
            entity.insert(ChildOf(parent));
        }
        entity.id()
    }

    pub fn cuboid(&mut self, size: [f32; 3], material: &Handle<StandardMaterial>, at: [f32; 3]) -> Entity {
        let mesh = self.mesh(meshes::cuboid(size[0], size[1], size[2]));
        self.spawn(mesh, material, Transform::from_translation(at.into()), None)
    }

    fn cuboid_in(&mut self, parent: Entity, size: [f32; 3], material: &Handle<StandardMaterial>, at: [f32; 3]) -> Entity {
        let mesh = self.mesh(meshes::cuboid(size[0], size[1], size[2]));
        self.spawn(mesh, material, Transform::from_translation(at.into()), Some(parent))
    }

    pub fn cylinder(&mut self, top: f32, bottom: f32, height: f32, material: &Handle<StandardMaterial>, at: [f32; 3]) -> Entity {
        let mesh = self.mesh(meshes::frustum(top, bottom, height, 7, false));
        self.spawn(mesh, material, Transform::from_translation(at.into()), None)
    }

    pub fn sphere(&mut self, radius: f32, material: &Handle<StandardMaterial>, at: [f32; 3]) -> Entity {
        let mesh = self.meshes.add(facet(Sphere::new(radius).mesh().uv(8, 6)));
        self.spawn(mesh, material, Transform::from_translation(at.into()), None)
    }

    /// Replaces an entity's placement, for parts that also need turning.
    fn place(&mut self, entity: Entity, at: [f32; 3], rotation: Quat) {
        self.commands
            .entity(entity)
            .insert(Transform::from_translation(at.into()).with_rotation(rotation));
    }

    fn no_shadow(&mut self, entity: Entity) {
        self.commands.entity(entity).insert(NotShadowCaster);
    }

    fn target(&mut self, entity: Entity, label: String, action: Action, size: [f32; 3]) {
        let half = Vec3::from(size) / 2.0;
        self.commands.entity(entity).insert(Target { label, action, half });
    }
}

pub fn setup(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    mut images: ResMut<Assets<Image>>,
) {
    let mut kit = Kit {
        commands: &mut commands,
        meshes: &mut meshes,
        materials: &mut materials,
        images: &mut images,
    };
    let mut colliders = Colliders::default();
    // Interior textures need variety, not a particular sequence.
    let mut rng = Rng::new(20_261_105);
    let (w, d, h) = (HALF_WIDTH, HALF_DEPTH, HEIGHT);

    let wood = kit.mat("#4b3020", 0.8, 0.0);
    let dark_wood = kit.mat("#291e15", 0.8, 0.0);
    let wall = kit.mat("#a3926f", 0.95, 0.0);
    let brass = kit.mat("#ad8a49", 0.32, 0.7);
    // Hand-hewn casings around the window and doors.
    let casing = kit.mat("#6a4527", 0.8, 0.0);

    // Floor; the board ceiling follows the roof (see `spawn_roof`).
    let floor_image = kit.image(textures::floor(&mut rng), true);
    let floor = kit.materials.add(StandardMaterial {
        base_color_texture: Some(floor_image),
        perceptual_roughness: 0.62,
        uv_transform: Affine2::from_scale(Vec2::new(w * 0.6, d * 0.58)),
        ..default()
    });
    kit.cuboid([w * 2.0, 0.15, d * 2.0], &floor, [0.0, -0.08, 0.0]);

    // Chinking: the pale mortar behind the logs, showing in the seams.
    kit.cuboid([w * 2.0, h, 0.18], &wall, [0.0, h / 2.0, -d]);
    kit.cuboid([w * 2.0, h, 0.18], &wall, [0.0, h / 2.0, d]);
    kit.cuboid([0.18, h, d * 2.0], &wall, [w, h / 2.0, 0.0]);
    // The left wall has a real opening for the winter window (z -3 to 2,
    // y 0.9 to 3.25).
    kit.cuboid([0.18, h, d - 3.0], &wall, [-w, h / 2.0, -(d + 3.0) / 2.0]);
    kit.cuboid([0.18, h, d - 2.0], &wall, [-w, h / 2.0, (d + 2.0) / 2.0]);
    kit.cuboid([0.18, 0.9, 5.0], &wall, [-w, 0.45, -0.5]);
    kit.cuboid([0.18, h - 3.25, 5.0], &wall, [-w, (h + 3.25) / 2.0, -0.5]);
    let gable = kit.mesh(meshes::gable(w, RIDGE - h));
    for z in [-d, d] {
        kit.spawn(gable.clone(), &wall, Transform::from_xyz(0.0, h, z), None);
    }

    spawn_logs(&mut kit, &mut rng);
    spawn_roof(&mut kit, &mut rng);

    // Window: cold glass, mullions, sill, and the winter forest outdoors.
    let glass = kit.materials.add(StandardMaterial {
        base_color: hex("#c1d4dc").with_alpha(0.12),
        perceptual_roughness: 0.16,
        clearcoat: 1.0,
        alpha_mode: AlphaMode::Blend,
        ..default()
    });
    let pane = kit.cuboid([0.035, 2.3, 4.9], &glass, [-w, 2.05, -0.5]);
    kit.no_shadow(pane);
    for z in [-3.0f32, 2.0] {
        kit.cuboid([0.25, 2.48, 0.12], &casing, [-w + 0.24, 2.05, z]);
    }
    for y in [0.9f32, 3.2] {
        kit.cuboid([0.25, 0.12, 5.15], &casing, [-w + 0.24, y, -0.5]);
    }
    for z in [-1.35f32, 0.35] {
        kit.cuboid([0.16, 2.3, 0.065], &casing, [-w + 0.28, 2.05, z]);
    }
    kit.cuboid([0.16, 0.065, 5.0], &casing, [-w + 0.28, 2.0, -0.5]);
    kit.cuboid([0.48, 0.1, 5.35], &casing, [-w + 0.36, 0.92, -0.5]);
    if !crate::flag("noforest") {
        winter::spawn(&mut kit);
    }
    let frost_image = kit.image(textures::frost(&mut rng), false);
    let frost = kit.materials.add(StandardMaterial {
        base_color: unlit("#9fb2d6", 0.45),
        base_color_texture: Some(frost_image),
        unlit: true,
        alpha_mode: AlphaMode::Blend,
        ..default()
    });
    let frost_mesh = kit.mesh(meshes::quad(4.9, 2.3));
    let frost_pane = kit.spawn(
        frost_mesh,
        &frost,
        Transform::from_xyz(-w + 0.03, 2.05, -0.5).with_rotation(Quat::from_rotation_y(FRAC_PI_2)),
        None,
    );
    kit.no_shadow(frost_pane);
    let outer_sill = kit.mat("#d9e3e7", 0.8, 0.0);
    kit.cuboid([0.28, 0.075, 5.12], &outer_sill, [-w - 0.16, 0.96, -0.5]);
    let curtain = kit.mat("#8c8a70", 0.8, 0.0);
    for z in [-3.1f32, 2.1] {
        for fold in 0..8 {
            kit.cylinder(0.09, 0.12, 2.7, &curtain, [-w + 0.5, 1.92, z + (fold as f32 - 3.5) * 0.055]);
        }
    }
    let rod = kit.cylinder(0.035, 0.035, 5.8, &brass, [-w + 0.5, 3.36, -0.5]);
    kit.place(rod, [-w + 0.5, 3.36, -0.5], Quat::from_rotation_x(FRAC_PI_2));

    // Doors to the rooms still to come.
    let doors = Doors { wood: &wood, dark_wood: &dark_wood, casing: &casing, brass: &brass };
    doors.spawn(&mut kit, "LIVING ROOM", -4.6, -d + 0.43, 0.0);
    doors.spawn(&mut kit, "KITCHEN", 4.6, -d + 0.43, 0.0);
    doors.spawn(&mut kit, "STUDY", w - 0.43, -4.0, -FRAC_PI_2);
    doors.spawn(&mut kit, "FRONT DOOR", 0.0, d - 0.43, PI);

    spawn_painting(&mut kit, &mut rng, Vec3::new(0.0, 2.25, -d + 0.36), &brass);

    // Runner rug and its fringes, from the front door toward the back wall.
    let rug_image = kit.image(textures::rug(&mut rng), false);
    let rug = kit.materials.add(StandardMaterial {
        base_color_texture: Some(rug_image),
        perceptual_roughness: 1.0,
        ..default()
    });
    let (rug_x, rug_z, rug_width, rug_depth) = RUG;
    kit.cuboid([rug_width, 0.025, rug_depth], &rug, [rug_x, 0.02, rug_z]);
    let fringe = kit.mat("#9b8e6e", 0.8, 0.0);
    let fringe_mesh = kit.mesh(meshes::cuboid(0.013, 0.009, 0.13));
    let tassels = ((rug_width - 0.1) / 0.041) as usize;
    for i in 0..tassels {
        let x = rug_x - (tassels - 1) as f32 * 0.041 / 2.0 + i as f32 * 0.041;
        for z in [rug_z + rug_depth / 2.0 + 0.06, rug_z - rug_depth / 2.0 - 0.06] {
            kit.spawn(fringe_mesh.clone(), &fringe, Transform::from_xyz(x, 0.025, z), None);
        }
    }

    // Console table on the right-hand wall.
    let (cx, cz) = (w - 0.7, 3.0);
    kit.cuboid([0.67, 0.13, 2.4], &wood, [cx, 0.85, cz]);
    kit.cuboid([0.59, 0.22, 2.25], &dark_wood, [cx, 0.69, cz]);
    for dx in [-0.25f32, 0.23] {
        for dz in [-1.05f32, 1.04] {
            kit.cuboid([0.075, 0.72, 0.075], &wood, [cx + dx, 0.36, cz + dz]);
        }
    }
    kit.cuboid([0.5, 0.07, 2.1], &wood, [cx, 0.22, cz]);
    colliders.block(cx, cz, 0.8, 2.5);
    for dz in [-0.57f32, 0.5] {
        kit.sphere(0.035, &brass, [cx - 0.33, 0.69, cz + dz]);
    }
    // A framed mirror above the console.
    kit.cuboid([0.1, 1.55, 1.2], &brass, [w - 0.4, 2.12, cz]);
    let mirror = kit.mat("#78918b", 0.22, 0.85);
    kit.cuboid([0.06, 1.4, 1.05], &mirror, [w - 0.47, 2.12, cz]);

    spawn_lamp(&mut kit, Vec3::new(cx - 0.05, 1.58, cz + 0.63));

    // Books, a ceramic bowl, and a small note.
    for (i, color) in ["#626950", "#6d4031", "#b0a07b"].into_iter().enumerate() {
        let cover = kit.mat(color, 0.8, 0.0);
        kit.cuboid([0.34, 0.055, 0.5], &cover, [cx - 0.09, 0.965 + i as f32 * 0.055, cz - 0.75]);
    }
    let ceramic = kit.materials.add(StandardMaterial {
        base_color: hex("#88958c"),
        perceptual_roughness: 0.8,
        double_sided: true,
        cull_mode: None,
        ..default()
    });
    let bowl_mesh = kit.mesh(meshes::sphere_segment(0.17, 8, 4, PI * 0.52));
    let bowl = kit.spawn(
        bowl_mesh,
        &ceramic,
        Transform::from_xyz(cx - 0.1, 1.09, cz - 0.2).with_rotation(Quat::from_rotation_x(PI)),
        None,
    );
    kit.no_shadow(bowl);
    let paper = kit.mat("#d4c4a0", 0.8, 0.0);
    let note = kit.cuboid([0.28, 0.008, 0.35], &paper, [cx - 0.15, 0.926, cz + 0.16]);
    kit.target(
        note,
        "Welcome note · E to read".into(),
        Action::Note,
        [0.28, 0.04, 0.35],
    );
    // The handwriting faces whoever stands at the console.
    let note_image = kit.image(textures::note(), false);
    let writing = kit.materials.add(StandardMaterial {
        base_color_texture: Some(note_image),
        perceptual_roughness: 0.8,
        ..default()
    });
    let writing_mesh = kit.mesh(meshes::quad(0.35, 0.28));
    let facing_reader = Quat::from_mat3(&Mat3::from_cols(Vec3::Z, Vec3::X, Vec3::Y));
    let page = kit.spawn(
        writing_mesh,
        &writing,
        Transform::from_xyz(cx - 0.15, 0.9305, cz + 0.16).with_rotation(facing_reader),
        None,
    );
    kit.no_shadow(page);

    // Upholstered window seat: a panelled wooden base on tapered feet, two
    // seat cushions, back cushions leaning on the wall under the sill, and a
    // throw pillow.
    let sx = -w + 0.98;
    kit.cuboid([0.86, 0.3, 2.0], &wood, [sx, 0.29, -0.5]);
    kit.cuboid([0.9, 0.035, 2.04], &dark_wood, [sx, 0.457, -0.5]);
    for z in [-0.95f32, -0.05] {
        // Raised panels on the front of the base.
        kit.cuboid([0.015, 0.035, 0.8], &dark_wood, [sx + 0.435, 0.39, z]);
        kit.cuboid([0.015, 0.035, 0.8], &dark_wood, [sx + 0.435, 0.19, z]);
        for dz in [-0.4f32, 0.4] {
            kit.cuboid([0.015, 0.235, 0.035], &dark_wood, [sx + 0.435, 0.29, z + dz]);
        }
    }
    let foot = kit.mesh(meshes::frustum(0.035, 0.024, 0.14, 6, false));
    for z in [-1.44f32, 0.44] {
        for dx in [-0.38f32, 0.38] {
            kit.spawn(foot.clone(), &dark_wood, Transform::from_xyz(sx + dx, 0.07, z), None);
        }
    }
    colliders.block(sx, -0.5, 1.0, 2.2);
    let upholstery = kit.mat("#8e896e", 0.95, 0.0);
    let seat = kit.meshes.add(meshes::cushion(Vec3::new(0.84, 0.15, 0.98), 0.05, 0.022, 8).build());
    let back = kit.meshes.add(meshes::cushion(Vec3::new(0.14, 0.3, 0.96), 0.05, 0.02, 8).build());
    for z in [-0.995f32, -0.005] {
        kit.spawn(seat.clone(), &upholstery, Transform::from_xyz(sx + 0.03, 0.55, z), None);
        // Leaning back toward the wall, clear of the sill above.
        kit.spawn(back.clone(), &upholstery, Transform::from_xyz(sx - 0.33, 0.77, z).with_rotation(Quat::from_rotation_z(0.16)), None);
    }
    let cushion_fabric = kit.mat("#a99b78", 0.95, 0.0);
    let pillow = kit.meshes.add(meshes::cushion(Vec3::new(0.12, 0.4, 0.4), 0.05, 0.035, 8).build());
    kit.spawn(
        pillow,
        &cushion_fabric,
        Transform::from_xyz(sx - 0.13, 0.8, -1.2).with_rotation(Quat::from_rotation_y(0.3) * Quat::from_rotation_z(0.3) * Quat::from_rotation_x(0.1)),
        None,
    );

    // Coat stand and a hanging wool coat, by the front door.
    let (sx, sz) = (w - 1.3, d - 1.4);
    kit.cylinder(0.26, 0.32, 0.09, &dark_wood, [sx, 0.05, sz]);
    kit.cylinder(0.045, 0.065, 2.15, &wood, [sx, 1.1, sz]);
    for i in 0..4 {
        let angle = i as f32 * FRAC_PI_2;
        let at = [sx + angle.cos() * 0.17, 1.94, sz + angle.sin() * 0.17];
        let hook = kit.cuboid([0.5, 0.035, 0.035], &brass, at);
        kit.place(hook, at, Quat::from_rotation_y(-angle));
    }
    let wool = kit.mat("#545a4c", 0.8, 0.0);
    kit.cuboid([0.42, 0.95, 0.22], &wool, [sx + 0.12, 1.34, sz]);
    colliders.block(sx, sz, 0.8, 0.8);

    // Umbrella pot on the other side of the door.
    let (px, pz) = (-w + 1.3, d - 1.4);
    let pot = kit.mat("#625f4c", 0.8, 0.0);
    kit.cylinder(0.2, 0.15, 0.52, &pot, [px, 0.26, pz]);
    colliders.block(px, pz, 0.45, 0.45);
    for i in 0..3 {
        let x = px - 0.05 + i as f32 * 0.06;
        let umbrella = kit.cylinder(0.025, 0.025, 1.03, &dark_wood, [x, 0.68, pz]);
        kit.place(umbrella, [x, 0.68, pz], Quat::from_rotation_z((i as f32 - 1.0) * 0.12));
        kit.sphere(0.045, &brass, [x, 1.2, pz]);
    }

    spawn_chandelier(&mut kit, &mut rng);

    // Night: a faint glow off the snow through the window, and a weak warm
    // fill standing in for candlelight bounced off the ceiling. The Three.js
    // scene used rectangular area lights; on WebGL 2 those cost so much per
    // pixel that the frame rate fell to about a third. Wide, shadowless
    // spotlights at the same places stand in for them: matching on-axis
    // intensity, an area light of luminance L and area A becomes a spot of
    // 4π·L·A lumens, and a zero inner angle approximates the area light's
    // cosine falloff. A large radius keeps highlights broad, like
    // reflections of a big light. `?rect` restores the area lights for
    // comparison.
    let area_lights = crate::flag("rect");
    //
    // An area light also grazes whatever sits beside it (the window's bench,
    // sill and curtains), which a spot in the same plane misses. Each spot
    // therefore sits `setback` metres behind its light, brightened by the
    // inverse-square loss at the room's centre, so those fall inside its cone.
    for (color, luminance, width, height, range, at, target, up, setback, middle) in [
        ("#7f95bf", 0.9, 4.8, 2.2, 16.0, Vec3::new(-w + 0.23, 2.05, -0.5), Vec3::new(0.0, 1.2, -0.5), Vec3::Y, 1.0f32, w - 0.2),
        ("#e8c9a0", 0.12, 10.0, 14.0, 14.0, Vec3::new(0.0, RIDGE - 1.2, -1.0), Vec3::new(0.0, 0.0, -1.0), Vec3::Z, 0.0, 3.5),
    ] {
        let transform = Transform::from_translation(at).looking_at(target, up);
        if area_lights {
            commands.spawn((
                RectLight { color: hex(color), intensity: luminance * width * height * PI, width, height, range },
                transform,
            ));
        } else {
            let gain = ((middle + setback) / middle).powi(2);
            commands.spawn((
                SpotLight {
                    color: hex(color),
                    intensity: 4.0 * PI * luminance * width * height * gain,
                    range: range + setback,
                    radius: width.min(height) / 2.0,
                    outer_angle: 1.5,
                    inner_angle: 0.0,
                    shadow_maps_enabled: false,
                    ..default()
                },
                transform.with_translation(at - transform.forward() * setback),
            ));
        }
    }
    commands.insert_resource(colliders);
}

/// Log walls, gables and roof trusses, merged into one smooth-shaded mesh.
///
/// Logs sit centred on the wall planes, so the chinking behind them shows
/// only in the seams. The side walls' courses are half a log higher than the
/// end walls', the way notched corners interlock, and line up with the
/// window so that cut logs end at its casings. The end walls carry on up
/// into the gables, each course shorter under the roof's pitch.
fn spawn_logs(kit: &mut Kit, rng: &mut Rng) {
    const COURSE: f32 = 0.6;
    const RADIUS: f32 = 0.32;
    const TILE: f32 = 3.2;
    let (w, d, h) = (HALF_WIDTH, HALF_DEPTH, HEIGHT);
    let rise = RIDGE - h;
    let pitch = rise.atan2(w);
    let mut logs = MeshData::default();
    let mut add = |rng: &mut Rng, length: f32, radius: f32, at: Vec3, rotation: Quat| {
        let radius = radius * (0.95 + rng.f32() * 0.1);
        let data = meshes::log(length, radius, 16, TILE, rng.f32() * 4.0);
        // Turn each log about its own axis so knots and checks don't line up.
        let rotation = rotation * Quat::from_rotation_x(rng.f32() * TAU);
        let jitter = Vec3::new(0.0, (rng.f32() - 0.5) * 0.015, 0.0);
        let transform = Mat4::from_rotation_translation(rotation, at + jitter);
        logs.append(data.merge_transformed(&[transform]));
    };
    let along_z = Quat::from_rotation_y(FRAC_PI_2);
    let mut y = 0.28;
    while y < RIDGE {
        // Under the roof, a log runs until its underside meets the roof logs.
        let above = (y + 2.0 * ROOF_LOG - RADIUS - h).max(0.0);
        let half = if above == 0.0 { w + 0.25 } else { w * (1.0 - above / rise) };
        if half > 0.3 {
            for z in [-d, d] {
                add(rng, half * 2.0, RADIUS, Vec3::new(0.0, y, z), Quat::IDENTITY);
            }
        }
        y += COURSE;
    }
    // A course tops out at the sill (0.9) and another just above the head.
    let mut y = -0.02;
    while y < h + 0.1 {
        let full = d * 2.0 + 0.5;
        add(rng, full, RADIUS, Vec3::new(w, y, 0.0), along_z);
        // The window opening runs from z = -3 to 2, y = 0.9 to 3.2.
        if y < 0.9 || y > 3.25 {
            add(rng, full, RADIUS, Vec3::new(-w, y, 0.0), along_z);
        } else {
            let (back, front) = (d + 0.25 - 3.0, d + 0.25 - 2.0);
            add(rng, back, RADIUS, Vec3::new(-w, y, -3.0 - back / 2.0), along_z);
            add(rng, front, RADIUS, Vec3::new(-w, y, 2.0 + front / 2.0), along_z);
        }
        y += COURSE;
    }

    // The roof: logs laid side by side from ridge to eave, their ends
    // running out past the walls.
    let roof_length = (w + OVERHANG) / pitch.cos();
    let mut z = -d - 0.4;
    while z <= d + 0.4 {
        for side in [1.0f32, -1.0] {
            let slope = if side > 0.0 { Quat::IDENTITY } else { Quat::from_rotation_y(PI) } * Quat::from_rotation_z(-pitch);
            let (down, normal) = (slope * Vec3::X, slope * Vec3::Y);
            let at = Vec3::new(0.0, RIDGE, z) + down * roof_length / 2.0 - normal * ROOF_LOG;
            add(rng, roof_length, ROOF_LOG, at, slope);
        }
        z += ROOF_LOG * 2.0;
    }

    // King-post trusses, clear of the chandelier's chain, and a ridge beam.
    for z in [-5.2f32, 2.2, 6.4] {
        let tie = h - 0.26;
        add(rng, w * 2.0, 0.26, Vec3::new(0.0, tie, z), Quat::IDENTITY);
        add(rng, RIDGE_BEAM - tie, 0.2, Vec3::new(0.0, (RIDGE_BEAM + tie) / 2.0, z), Quat::from_rotation_z(FRAC_PI_2));
    }
    add(rng, d * 2.0, 0.26, Vec3::new(0.0, RIDGE_BEAM, 0.0), along_z);

    let image = kit.image(textures::log(rng), true);
    let material = kit.materials.add(StandardMaterial {
        base_color_texture: Some(image),
        perceptual_roughness: 0.82,
        ..default()
    });
    let mesh = kit.meshes.add(logs.build());
    kit.spawn(mesh, &material, Transform::IDENTITY, None);
}

/// The pitched roof over the log roof: decking, and cedar shingles that run
/// out past the walls as eaves.
fn spawn_roof(kit: &mut Kit, rng: &mut Rng) {
    const BOARDS: f32 = 0.08;
    let (w, d, h) = (HALF_WIDTH, HALF_DEPTH, HEIGHT);
    let rise = RIDGE - h;
    let pitch = rise.atan2(w);
    let slope = (w + OVERHANG) / pitch.cos();
    let length = d * 2.0 + 1.2;

    // Dark decking, glimpsed in the seams between the roof logs.
    let decking = kit.mat("#1c140e", 0.9, 0.0);
    let shingle_image = kit.image(textures::shingles(rng), true);
    let shingles = kit.materials.add(StandardMaterial {
        base_color_texture: Some(shingle_image),
        perceptual_roughness: 0.95,
        ..default()
    });
    let boards = kit.mesh(meshes::cuboid(slope, BOARDS, length));
    let shingled = kit.mesh(meshes::tiled_cuboid(slope + 0.05, 0.12, length + 0.2, 1.2));
    for side in [1.0f32, -1.0] {
        // Local +x runs down the slope on both sides.
        let rotation = if side > 0.0 { Quat::IDENTITY } else { Quat::from_rotation_y(PI) } * Quat::from_rotation_z(-pitch);
        let down = rotation * Vec3::X;
        let normal = rotation * Vec3::Y;
        let ridge = Vec3::new(0.0, RIDGE, 0.0);
        let at = ridge + down * slope / 2.0 + normal * BOARDS / 2.0;
        kit.spawn(boards.clone(), &decking, Transform::from_translation(at).with_rotation(rotation), None);
        let at = ridge + down * (slope / 2.0 + 0.02) + normal * (BOARDS + 0.06);
        kit.spawn(shingled.clone(), &shingles, Transform::from_translation(at).with_rotation(rotation), None);
    }
    let cap = kit.mat("#2b2119", 0.9, 0.0);
    kit.cuboid([0.4, 0.1, length + 0.2], &cap, [0.0, RIDGE + BOARDS + 0.17, 0.0]);
}

/// Two crossed quads standing on their bottom edge, so a flame scales up
/// from its base.
fn flame_mesh() -> MeshData {
    let quad = meshes::quad(1.0, 1.0);
    let lift = Vec3::new(0.0, 0.5, 0.0);
    quad.merge_transformed(&[
        Mat4::from_translation(lift),
        Mat4::from_rotation_translation(Quat::from_rotation_y(FRAC_PI_2), lift),
    ])
}

fn flame_material(kit: &mut Kit, strength: f32) -> Handle<StandardMaterial> {
    let image = kit.image(textures::flame(), false);
    kit.materials.add(StandardMaterial {
        base_color: Color::LinearRgba(LinearRgba::new(strength, strength, strength, 1.0)),
        base_color_texture: Some(image),
        unlit: true,
        alpha_mode: AlphaMode::Add,
        double_sided: true,
        cull_mode: None,
        ..default()
    })
}

/// A two-tier wrought-iron chandelier of candles, hung on a long chain from
/// the ridge beam. Its central light casts the room's main shadows, though
/// not the chandelier's.
fn spawn_chandelier(kit: &mut Kit, rng: &mut Rng) {
    let (x, z) = (0.0f32, -1.4f32);
    let iron = kit.mat("#2b2724", 0.55, 0.65);
    let wax = kit.mat("#e8dcc0", 0.7, 0.0);
    let (low, high) = (3.3f32, 3.8f32);
    // The underside of the ridge beam.
    let hook = RIDGE_BEAM - 0.26;

    // Chain and central stem.
    let mut parts = vec![kit.sphere(0.05, &iron, [x, hook, z])];
    let link = kit.meshes.add(facet(Torus { minor_radius: 0.012, major_radius: 0.05 }.mesh().minor_resolution(4).major_resolution(8).build()));
    let mut y = hook - 0.05;
    let mut i = 0;
    while y > high + 0.25 {
        let turn = if i % 2 == 0 { 0.0 } else { FRAC_PI_2 };
        parts.push(kit.spawn(
            link.clone(),
            &iron,
            Transform::from_xyz(x, y, z).with_rotation(Quat::from_rotation_y(turn) * Quat::from_rotation_x(FRAC_PI_2)),
            None,
        ));
        y -= 0.085;
        i += 1;
    }
    let stem_top = y + 0.03;
    let stem_bottom = low - 0.15;
    parts.push(kit.cylinder(0.035, 0.035, stem_top - stem_bottom, &iron, [x, (stem_top + stem_bottom) / 2.0, z]));
    for collar in [stem_top, high, low] {
        parts.push(kit.sphere(0.06, &iron, [x, collar, z]));
    }
    let finial = kit.mesh(meshes::frustum(0.07, 0.0, 0.22, 7, false));
    parts.push(kit.spawn(finial, &iron, Transform::from_xyz(x, stem_bottom - 0.08, z), None));

    // Two rings of candles on spokes.
    let mut candles = Vec::new();
    let mut flames = Vec::new();
    for (radius, ring_y, count, turn) in [(1.0f32, low, 8, 0.0f32), (0.55, high, 6, 0.26)] {
        let ring = kit.meshes.add(facet(Torus { minor_radius: 0.025, major_radius: radius }.mesh().minor_resolution(5).major_resolution(28).build()));
        parts.push(kit.spawn(ring, &iron, Transform::from_xyz(x, ring_y, z), None));
        for k in 0..count {
            let angle = turn + k as f32 * TAU / count as f32;
            let (sin, cos) = angle.sin_cos();
            let spoke = kit.cuboid([radius, 0.028, 0.028], &iron, [x + cos * radius / 2.0, ring_y, z + sin * radius / 2.0]);
            parts.push(spoke);
            kit.place(spoke, [x + cos * radius / 2.0, ring_y, z + sin * radius / 2.0], Quat::from_rotation_y(-angle));
            let (cx, cz) = (x + cos * radius, z + sin * radius);
            parts.push(kit.cylinder(0.055, 0.035, 0.05, &iron, [cx, ring_y + 0.045, cz]));
            let height = 0.16 + rng.f32() * 0.08;
            candles.push(Mat4::from_scale_rotation_translation(
                Vec3::new(1.0, height, 1.0),
                Quat::IDENTITY,
                Vec3::new(cx, ring_y + 0.07 + height / 2.0, cz),
            ));
            flames.push(Mat4::from_scale_rotation_translation(
                Vec3::new(0.06, 0.13, 0.06),
                Quat::from_rotation_y(rng.f32()),
                Vec3::new(cx, ring_y + 0.07 + height + 0.005, cz),
            ));
        }
    }
    let candle_mesh = kit.mesh(meshes::frustum(0.026, 0.028, 1.0, 8, false).merge_transformed(&candles));
    let candle_entity = kit.spawn(candle_mesh, &wax, Transform::IDENTITY, None);
    kit.no_shadow(candle_entity);
    let flame_mesh = kit.meshes.add(flame_mesh().merge_transformed(&flames).build());
    let candlelight = flame_material(kit, 2.2);
    let flame_entity = kit.spawn(flame_mesh, &candlelight, Transform::IDENTITY, None);
    kit.no_shadow(flame_entity);
    // The candles themselves give the light, so the ironwork shouldn't
    // throw a giant silhouette of itself across the ceiling and floor.
    for part in parts {
        kit.no_shadow(part);
    }

    kit.commands.spawn((
        PointLight {
            color: hex("#ffd7a3"),
            intensity: 34.0 * CANDELA,
            range: 18.0,
            radius: 0.8,
            shadow_maps_enabled: !crate::flag("noshadow"),
            shadow_map_near_z: 0.3,
            ..default()
        },
        Transform::from_xyz(x, low + 0.25, z),
    ));
}

/// An oil landscape in a carved gilt frame, hung on the back wall with a
/// brass picture light above it. `wall` is the centre of the frame's back.
fn spawn_painting(kit: &mut Kit, rng: &mut Rng, wall: Vec3, brass: &Handle<StandardMaterial>) {
    let (width, height) = (1.5f32, 1.05f32);
    let image = kit.image(textures::painting(rng), false);
    let canvas = kit.materials.add(StandardMaterial {
        base_color_texture: Some(image),
        // A coat of varnish: a faint sheen over matte paint.
        perceptual_roughness: 0.7,
        clearcoat: 0.35,
        clearcoat_perceptual_roughness: 0.45,
        ..default()
    });
    kit.cuboid([width, height, 0.03], &canvas, (wall + Vec3::new(0.0, 0.0, 0.05)).into());

    let gilt = |kit: &mut Kit, rng: &mut Rng, leaves: bool| {
        let image = kit.image(textures::gilt(rng, leaves), true);
        kit.materials.add(StandardMaterial {
            base_color_texture: Some(image),
            metallic: 0.55,
            perceptual_roughness: 0.42,
            ..default()
        })
    };
    let beads = gilt(kit, rng, false);
    let leaves = gilt(kit, rng, true);
    let liner = kit.mat("#cbbf9f", 0.95, 0.0);
    // Rings of moulding from the canvas outward: (width, depth, tilt, material, tile).
    // Tilted rings slope back toward the canvas, like a carved cove.
    let rings = [
        (0.06f32, 0.075f32, 0.0f32, &liner, 1.0f32),
        (0.025, 0.095, 0.0, &beads, 0.14),
        (0.1, 0.12, 0.42, &leaves, 0.42),
        (0.05, 0.15, 0.0, &beads, 0.2),
    ];
    let (mut inner_w, mut inner_h) = (width, height);
    for (ring, depth, tilt, material, tile) in rings {
        for (horizontal, sign) in [(true, 1.0f32), (true, -1.0), (false, 1.0), (false, -1.0)] {
            let length = if horizontal { inner_w + ring * 2.0 } else { inner_h };
            let mesh = kit.mesh(meshes::moulding(length, ring, depth, tile));
            // Each piece is built lying along x with its inner edge at -y once
            // turned into place, then tipped back about its length.
            let (offset, turn) = if horizontal {
                (Vec3::new(0.0, sign * (inner_h + ring) / 2.0, 0.0), if sign > 0.0 { 0.0 } else { PI })
            } else {
                (Vec3::new(sign * (inner_w + ring) / 2.0, 0.0, 0.0), if sign > 0.0 { -FRAC_PI_2 } else { FRAC_PI_2 })
            };
            let rotation = Quat::from_rotation_z(turn) * Quat::from_rotation_x(tilt);
            let at = wall + offset + Vec3::new(0.0, 0.0, depth / 2.0);
            kit.spawn(mesh, material, Transform::from_translation(at).with_rotation(rotation), None);
        }
        inner_w += ring * 2.0;
        inner_h += ring * 2.0;
    }

    // Picture light: a brass hood on an arm, washing the canvas from above.
    let top = wall.y + inner_h / 2.0;
    let arm = kit.cuboid([0.03, 0.03, 0.28], brass, [wall.x, top + 0.12, wall.z + 0.14]);
    kit.no_shadow(arm);
    let hood = kit.cylinder(0.045, 0.045, 0.62, brass, [wall.x, top + 0.12, wall.z + 0.3]);
    kit.place(hood, [wall.x, top + 0.12, wall.z + 0.3], Quat::from_rotation_z(FRAC_PI_2));
    kit.no_shadow(hood);
    let bulb = kit.materials.add(StandardMaterial {
        base_color: unlit("#fff0cf", 1.0),
        unlit: true,
        ..default()
    });
    let strip = kit.cuboid([0.56, 0.012, 0.03], &bulb, [wall.x, top + 0.077, wall.z + 0.3]);
    kit.no_shadow(strip);
    kit.commands.spawn((
        SpotLight {
            color: hex("#ffd9a0"),
            intensity: 3.2 * CANDELA,
            range: 3.5,
            radius: 0.3,
            outer_angle: 0.85,
            inner_angle: 0.35,
            shadow_maps_enabled: false,
            ..default()
        },
        // Set out from the hood so the pool spreads evenly down the canvas.
        Transform::from_xyz(wall.x, top + 0.2, wall.z + 0.75).looking_at(wall + Vec3::new(0.0, -0.25, 0.05), Vec3::Y),
    ));
}

struct Doors<'m> {
    wood: &'m Handle<StandardMaterial>,
    dark_wood: &'m Handle<StandardMaterial>,
    casing: &'m Handle<StandardMaterial>,
    brass: &'m Handle<StandardMaterial>,
}

impl Doors<'_> {
    fn spawn(&self, kit: &mut Kit, name: &'static str, x: f32, z: f32, rotation: f32) {
        let door = kit
            .commands
            .spawn((Transform::from_xyz(x, 0.0, z).with_rotation(Quat::from_rotation_y(rotation)), Visibility::default()))
            .id();
        kit.cuboid_in(door, [1.65, 2.8, 0.17], self.dark_wood, [0.0, 1.4, 0.0]);
        let panel = kit.cuboid_in(door, [1.42, 2.63, 0.1], self.wood, [0.0, 1.315, 0.12]);
        for (y, outer, inner) in [(0.61, 0.73, 0.61), (1.65, 1.05, 0.93)] {
            kit.cuboid_in(door, [1.14, outer, 0.05], self.dark_wood, [0.0, y, 0.19]);
            kit.cuboid_in(door, [1.02, inner, 0.055], self.wood, [0.0, y, 0.22]);
        }
        for dx in [-0.84f32, 0.84] {
            kit.cuboid_in(door, [0.15, 2.95, 0.28], self.casing, [dx, 1.475, 0.06]);
        }
        kit.cuboid_in(door, [1.85, 0.16, 0.28], self.casing, [0.0, 2.9, 0.06]);
        kit.cuboid_in(door, [1.48, 0.035, 0.35], self.brass, [0.0, 0.025, 0.14]);
        kit.cuboid_in(door, [0.13, 0.3, 0.04], self.brass, [0.48, 1.2, 0.25]);
        let knob = kit.meshes.add(facet(Sphere::new(0.065).mesh().uv(6, 4)));
        kit.spawn(knob, self.brass, Transform::from_xyz(0.48, 1.24, 0.32), Some(door));
        let sign_image = kit.image(textures::label(name), false);
        let sign_material = kit.materials.add(StandardMaterial {
            base_color_texture: Some(sign_image),
            perceptual_roughness: 0.8,
            ..default()
        });
        let sign = kit.cuboid_in(door, [0.9, 0.265, 0.035], &sign_material, [0.0, 2.38, 0.27]);
        let label = format!("{name} · E to try the door");
        kit.target(panel, label.clone(), Action::Door(name), [1.42, 2.63, 0.1]);
        kit.target(sign, label, Action::Door(name), [0.9, 0.265, 0.035]);
    }
}

/// A linen-shaded table lamp. The shade's glow is an emissive map describing
/// the bulb's light diffusing through fabric; a point light carries that glow
/// into the room and spotlights spill light up and down.
fn spawn_lamp(kit: &mut Kit, center: Vec3) {
    let brass = kit.mat("#ad8a49", 0.32, 0.7);
    kit.cylinder(0.17, 0.22, 0.06, &brass, [center.x, 0.96, center.z]);
    kit.cylinder(0.035, 0.05, 0.48, &brass, [center.x, 1.21, center.z]);

    let linen = kit.image(textures::linen(), false);
    let diffusion = kit.image(textures::diffusion(), false);
    let shade_glow = glow("#ffd29b", 1.3);
    let shade_material = kit.materials.add(StandardMaterial {
        base_color_texture: Some(linen),
        perceptual_roughness: 0.95,
        emissive: shade_glow,
        emissive_texture: Some(diffusion),
        emissive_exposure_weight: 1.0,
        double_sided: true,
        cull_mode: None,
        ..default()
    });
    let shade_mesh = kit.mesh(meshes::frustum(0.19, 0.33, 0.42, 9, true));
    let shade = kit.spawn(shade_mesh, &shade_material, Transform::from_translation(center), None);
    // The fabric transmits light; an opaque shadow from the shade would hide it.
    kit.no_shadow(shade);
    kit.target(shade, "Table lamp · E to switch".into(), Action::Lamp, [0.66, 0.42, 0.66]);

    let filament = kit.materials.add(StandardMaterial {
        base_color: unlit("#fff0cf", 1.0),
        unlit: true,
        ..default()
    });
    let bulb = kit.sphere(0.063, &filament, [center.x, 1.57, center.z]);
    kit.no_shadow(bulb);

    let piping = kit.mat("#b9a683", 0.9, 0.0);
    for (radius, y) in [(0.19f32, 1.79f32), (0.33, 1.37)] {
        let torus = kit.meshes.add(Torus { minor_radius: 0.007, major_radius: radius }.mesh().minor_resolution(4).major_resolution(9));
        let ring = kit.spawn(torus, &piping, Transform::from_xyz(center.x, y, center.z), None);
        kit.no_shadow(ring);
    }

    let commands = &mut kit.commands;
    // Light through the fabric. The Three.js scene used four small area
    // lights around the shade; one point light of the same total power looks
    // nearly identical and keeps per-pixel lighting cheap enough for 60 fps.
    let fabric_power = 4.0 * 9.0 * 0.29 * 0.32 * PI;
    commands.spawn((
        PointLight {
            color: hex("#ffcf96"),
            intensity: fabric_power,
            range: 6.0,
            radius: 0.25,
            ..default()
        },
        Transform::from_translation(center),
        LampLight(fabric_power),
    ));
    for (y, target_y, candela, angle) in [(1.34f32, 0.8f32, 4.2f32, 1.05f32), (1.82, 3.8, 3.2, 0.9)] {
        commands.spawn((
            SpotLight {
                color: hex("#ffce92"),
                intensity: candela * CANDELA,
                range: 5.0,
                outer_angle: angle,
                inner_angle: angle * 0.15,
                shadow_maps_enabled: !crate::flag("noshadow"),
                // Keep the lamp's own stem and base out of its shadow maps.
                shadow_map_near_z: 0.5,
                ..default()
            },
            Transform::from_xyz(center.x, y, center.z).looking_at(Vec3::new(center.x, target_y, center.z), Vec3::Z),
            LampLight(candela * CANDELA),
        ));
    }
    // A small fill approximates light reflected back from the nearby wall.
    commands.spawn((
        PointLight {
            color: hex("#ffd9a6"),
            intensity: 1.7 * CANDELA,
            range: 3.5,
            ..default()
        },
        Transform::from_xyz(center.x - 0.22, 1.55, center.z),
        LampLight(1.7 * CANDELA),
    ));
    commands.insert_resource(Lamp { on: true, shade: shade_material, shade_glow, bulb });
}

/// Switches the table lamp, returning the message to show.
pub fn toggle_lamp(
    lamp: &mut Lamp,
    lights: &mut Query<(&LampLight, Option<&mut PointLight>, Option<&mut SpotLight>, Option<&mut RectLight>)>,
    materials: &mut Assets<StandardMaterial>,
    visibility: &mut Query<&mut Visibility>,
) -> &'static str {
    lamp.on = !lamp.on;
    let scale = if lamp.on { 1.0 } else { 0.0 };
    for (power, point, spot, rect) in lights.iter_mut() {
        if let Some(mut light) = point {
            light.intensity = power.0 * scale;
        }
        if let Some(mut light) = spot {
            light.intensity = power.0 * scale;
        }
        if let Some(mut light) = rect {
            light.intensity = power.0 * scale;
        }
    }
    if let Some(mut shade) = materials.get_mut(&lamp.shade) {
        shade.emissive = if lamp.on { lamp.shade_glow } else { LinearRgba::BLACK };
    }
    if let Ok(mut bulb) = visibility.get_mut(lamp.bulb) {
        *bulb = if lamp.on { Visibility::Inherited } else { Visibility::Hidden };
    }
    if lamp.on { "A little warmth." } else { "The lamp is off." }
}
