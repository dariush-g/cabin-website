//! The entrance hall: architecture, furnishings, doors and lighting.
//!
//! Light intensities are carried over from the Three.js scene: candela
//! become lumens (× 4π) and rectangular luminance becomes lumens (× area × π).
//! The camera's exposure is set so the two renderers agree.

use std::f32::consts::{FRAC_PI_2, PI};

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
        x > -4.5
            && x < 4.5
            && z > -5.4
            && z < 5.4
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
        self.meshes.add(data.build())
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
        let mesh = self.mesh(meshes::frustum(top, bottom, height, 32, false));
        self.spawn(mesh, material, Transform::from_translation(at.into()), None)
    }

    pub fn sphere(&mut self, radius: f32, material: &Handle<StandardMaterial>, at: [f32; 3]) -> Entity {
        let mesh = self.meshes.add(Sphere::new(radius).mesh().uv(20, 14));
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

    let wood = kit.mat("#4b3020", 0.8, 0.0);
    let dark_wood = kit.mat("#291e15", 0.8, 0.0);
    let trim = kit.mat("#393e32", 0.8, 0.0);
    let wall = kit.mat("#737966", 0.8, 0.0);
    let brass = kit.mat("#ad8a49", 0.32, 0.7);
    let cream = kit.mat("#d4c9aa", 0.8, 0.0);
    let panel_moulding = kit.mat("#4b5040", 0.8, 0.0);

    // Floor, ceiling and walls.
    let floor_image = kit.image(textures::floor(&mut rng), true);
    let floor = kit.materials.add(StandardMaterial {
        base_color_texture: Some(floor_image),
        perceptual_roughness: 0.62,
        uv_transform: Affine2::from_scale(Vec2::new(3.0, 3.5)),
        ..default()
    });
    kit.cuboid([10.0, 0.15, 12.0], &floor, [0.0, -0.08, 0.0]);
    let ceiling = kit.mat("#b0ad94", 0.8, 0.0);
    kit.cuboid([10.0, 0.2, 12.0], &ceiling, [0.0, 3.9, 0.0]);
    kit.cuboid([10.0, 3.9, 0.18], &wall, [0.0, 1.95, -6.0]);
    kit.cuboid([10.0, 3.9, 0.18], &wall, [0.0, 1.95, 6.0]);
    kit.cuboid([0.18, 3.9, 12.0], &wall, [5.0, 1.95, 0.0]);
    // The left wall has a real opening for the winter window.
    kit.cuboid([0.18, 3.9, 3.0], &wall, [-5.0, 1.95, -4.5]);
    kit.cuboid([0.18, 3.9, 4.0], &wall, [-5.0, 1.95, 4.0]);
    kit.cuboid([0.18, 0.9, 5.0], &wall, [-5.0, 0.45, -0.5]);
    kit.cuboid([0.18, 0.65, 5.0], &wall, [-5.0, 3.575, -0.5]);

    // Wainscoting and architectural trim.
    for x in [-4.87f32, 4.87] {
        kit.cuboid([0.12, 1.05, 11.8], &trim, [x, 0.525, 0.0]);
        kit.cuboid([0.18, 0.07, 11.8], &dark_wood, [x, 1.08, 0.0]);
        kit.cuboid([0.19, 0.16, 11.8], &dark_wood, [x, 0.08, 0.0]);
        kit.cuboid([0.23, 0.13, 11.8], &trim, [x, 3.75, 0.0]);
        let inset = if x < 0.0 { 0.07 } else { -0.07 };
        let mut z = -5.7;
        while z < 6.0 {
            kit.cuboid([0.16, 0.85, 0.035], &panel_moulding, [x + inset, 0.54, z]);
            z += 0.85;
        }
    }
    for z in [-5.88f32, 5.88] {
        kit.cuboid([9.8, 1.05, 0.1], &trim, [0.0, 0.525, z]);
        kit.cuboid([9.8, 0.07, 0.17], &dark_wood, [0.0, 1.08, z]);
        kit.cuboid([9.8, 0.16, 0.16], &dark_wood, [0.0, 0.08, z]);
        kit.cuboid([9.8, 0.13, 0.23], &trim, [0.0, 3.75, z]);
        let mut x = -4.5;
        while x < 5.0 {
            kit.cuboid([0.035, 0.85, 0.17], &panel_moulding, [x, 0.54, z]);
            x += 0.85;
        }
    }

    // Window: cold glass, mullions, sill, and the winter forest outdoors.
    let glass = kit.materials.add(StandardMaterial {
        base_color: hex("#c1d4dc").with_alpha(0.12),
        perceptual_roughness: 0.16,
        clearcoat: 1.0,
        alpha_mode: AlphaMode::Blend,
        ..default()
    });
    let pane = kit.cuboid([0.035, 2.3, 4.9], &glass, [-5.0, 2.05, -0.5]);
    kit.no_shadow(pane);
    for z in [-3.0f32, 2.0] {
        kit.cuboid([0.25, 2.48, 0.12], &cream, [-4.87, 2.05, z]);
    }
    for y in [0.9f32, 3.2] {
        kit.cuboid([0.25, 0.12, 5.15], &cream, [-4.87, y, -0.5]);
    }
    for z in [-1.35f32, 0.35] {
        kit.cuboid([0.16, 2.3, 0.065], &cream, [-4.85, 2.05, z]);
    }
    kit.cuboid([0.16, 0.065, 5.0], &cream, [-4.84, 2.0, -0.5]);
    kit.cuboid([0.48, 0.1, 5.35], &cream, [-4.76, 0.92, -0.5]);
    if !crate::flag("noforest") {
        winter::spawn(&mut kit);
    }
    let frost_image = kit.image(textures::frost(&mut rng), false);
    let frost = kit.materials.add(StandardMaterial {
        base_color: unlit("#ffffff", 0.75),
        base_color_texture: Some(frost_image),
        unlit: true,
        alpha_mode: AlphaMode::Blend,
        ..default()
    });
    let frost_mesh = kit.mesh(meshes::quad(4.9, 2.3));
    let frost_pane = kit.spawn(
        frost_mesh,
        &frost,
        Transform::from_xyz(-4.97, 2.05, -0.5).with_rotation(Quat::from_rotation_y(FRAC_PI_2)),
        None,
    );
    kit.no_shadow(frost_pane);
    let outer_sill = kit.mat("#d9e3e7", 0.8, 0.0);
    kit.cuboid([0.28, 0.075, 5.12], &outer_sill, [-5.16, 0.96, -0.5]);
    let curtain = kit.mat("#8c8a70", 0.8, 0.0);
    for z in [-3.1f32, 2.1] {
        for fold in 0..8 {
            kit.cylinder(0.09, 0.12, 2.7, &curtain, [-4.55, 1.92, z + (fold as f32 - 3.5) * 0.055]);
        }
    }
    let rod = kit.cylinder(0.035, 0.035, 5.8, &brass, [-4.54, 3.36, -0.5]);
    kit.place(rod, [-4.54, 3.36, -0.5], Quat::from_rotation_x(FRAC_PI_2));

    // Doors to the rooms still to come.
    let doors = Doors { wood: &wood, dark_wood: &dark_wood, cream: &cream, brass: &brass };
    doors.spawn(&mut kit, "LIVING ROOM", -2.5, -5.75, 0.0);
    doors.spawn(&mut kit, "KITCHEN", 1.0, -5.75, 0.0);
    doors.spawn(&mut kit, "STUDY", 4.75, -2.0, -FRAC_PI_2);
    doors.spawn(&mut kit, "FRONT DOOR", 0.8, 5.75, PI);

    // Runner rug and its fringes.
    let rug_image = kit.image(textures::rug(&mut rng), false);
    let rug = kit.materials.add(StandardMaterial {
        base_color_texture: Some(rug_image),
        perceptual_roughness: 1.0,
        ..default()
    });
    kit.cuboid([2.5, 0.025, 6.0], &rug, [0.15, 0.02, -0.7]);
    let fringe = kit.mat("#9b8e6e", 0.8, 0.0);
    let fringe_mesh = kit.mesh(meshes::cuboid(0.013, 0.009, 0.13));
    for i in 0..60 {
        let x = -1.08 + i as f32 * 0.041;
        for z in [2.36f32, -3.76] {
            kit.spawn(fringe_mesh.clone(), &fringe, Transform::from_xyz(x, 0.025, z), None);
        }
    }

    // Console table on the right-hand wall.
    kit.cuboid([0.67, 0.13, 2.4], &wood, [4.37, 0.85, 1.6]);
    kit.cuboid([0.59, 0.22, 2.25], &dark_wood, [4.37, 0.69, 1.6]);
    for x in [4.12f32, 4.6] {
        for z in [0.55f32, 2.64] {
            kit.cuboid([0.075, 0.72, 0.075], &wood, [x, 0.36, z]);
        }
    }
    kit.cuboid([0.5, 0.07, 2.1], &wood, [4.37, 0.22, 1.6]);
    colliders.block(4.37, 1.6, 0.8, 2.5);
    for z in [1.03f32, 2.1] {
        kit.sphere(0.035, &brass, [4.04, 0.69, z]);
    }
    // A framed mirror above the console.
    kit.cuboid([0.1, 1.55, 1.2], &brass, [4.82, 2.12, 1.6]);
    let mirror = kit.mat("#78918b", 0.22, 0.85);
    kit.cuboid([0.06, 1.4, 1.05], &mirror, [4.75, 2.12, 1.6]);

    spawn_lamp(&mut kit);

    // Books, a ceramic bowl, and a small note.
    for (i, color) in ["#626950", "#6d4031", "#b0a07b"].into_iter().enumerate() {
        let cover = kit.mat(color, 0.8, 0.0);
        kit.cuboid([0.34, 0.055, 0.5], &cover, [4.28, 0.965 + i as f32 * 0.055, 0.85]);
    }
    let ceramic = kit.materials.add(StandardMaterial {
        base_color: hex("#88958c"),
        perceptual_roughness: 0.8,
        double_sided: true,
        cull_mode: None,
        ..default()
    });
    let bowl_mesh = kit.mesh(meshes::sphere_segment(0.17, 24, 12, PI * 0.52));
    let bowl = kit.spawn(
        bowl_mesh,
        &ceramic,
        Transform::from_xyz(4.27, 1.09, 1.4).with_rotation(Quat::from_rotation_x(PI)),
        None,
    );
    kit.no_shadow(bowl);
    let paper = kit.mat("#d4c4a0", 0.8, 0.0);
    let note = kit.cuboid([0.28, 0.008, 0.35], &paper, [4.22, 0.926, 1.76]);
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
        Transform::from_xyz(4.22, 0.9305, 1.76).with_rotation(facing_reader),
        None,
    );
    kit.no_shadow(page);

    // Upholstered window bench.
    kit.cuboid([0.88, 0.16, 2.0], &wood, [-4.02, 0.43, -0.5]);
    let upholstery = kit.mat("#8e896e", 0.8, 0.0);
    kit.cuboid([0.92, 0.15, 2.06], &upholstery, [-4.02, 0.575, -0.5]);
    for z in [-1.32f32, 0.32] {
        for x in [-4.32f32, -3.72] {
            kit.cuboid([0.07, 0.4, 0.07], &wood, [x, 0.2, z]);
        }
    }
    colliders.block(-4.02, -0.5, 1.0, 2.2);
    let cushion_fabric = kit.mat("#a99b78", 0.8, 0.0);
    let cushion = kit.cuboid([0.57, 0.18, 0.56], &cushion_fabric, [-4.05, 0.73, -1.07]);
    kit.place(cushion, [-4.05, 0.73, -1.07], Quat::from_rotation_y(0.2));

    // Coat stand and a hanging wool coat.
    kit.cylinder(0.26, 0.32, 0.09, &dark_wood, [3.7, 0.05, 4.4]);
    kit.cylinder(0.045, 0.065, 2.15, &wood, [3.7, 1.1, 4.4]);
    for i in 0..4 {
        let angle = i as f32 * FRAC_PI_2;
        let at = [3.7 + angle.cos() * 0.17, 1.94, 4.4 + angle.sin() * 0.17];
        let hook = kit.cuboid([0.5, 0.035, 0.035], &brass, at);
        kit.place(hook, at, Quat::from_rotation_y(-angle));
    }
    let wool = kit.mat("#545a4c", 0.8, 0.0);
    kit.cuboid([0.42, 0.95, 0.22], &wool, [3.82, 1.34, 4.4]);
    colliders.block(3.7, 4.4, 0.8, 0.8);

    // Umbrella pot.
    let pot = kit.mat("#625f4c", 0.8, 0.0);
    kit.cylinder(0.2, 0.15, 0.52, &pot, [-3.7, 0.26, 4.6]);
    colliders.block(-3.7, 4.6, 0.45, 0.45);
    for i in 0..3 {
        let x = -3.75 + i as f32 * 0.06;
        let umbrella = kit.cylinder(0.025, 0.025, 1.03, &dark_wood, [x, 0.68, 4.6]);
        kit.place(umbrella, [x, 0.68, 4.6], Quat::from_rotation_z((i as f32 - 1.0) * 0.12));
        kit.sphere(0.045, &brass, [x, 1.2, 4.6]);
    }

    // A landscape painting beside the doors.
    kit.cuboid([1.4, 1.02, 0.09], &brass, [3.22, 2.1, -5.76]);
    let painting_image = kit.image(textures::painting(), false);
    let painting = kit.materials.add(StandardMaterial {
        base_color_texture: Some(painting_image),
        perceptual_roughness: 0.8,
        ..default()
    });
    kit.cuboid([1.23, 0.85, 0.03], &painting, [3.22, 2.1, -5.7]);

    spawn_pendant(&mut kit, &brass, &dark_wood);

    // Overcast daylight through the window, and a broad fill standing in for
    // light bounced off the ceiling.
    if !crate::flag("norect") { commands.spawn((
        RectLight {
            color: hex("#ccdfef"),
            intensity: 5.0 * 4.8 * 2.2 * PI,
            width: 4.8,
            height: 2.2,
            range: 14.0,
        },
        Transform::from_xyz(-4.77, 2.05, -0.5).looking_at(Vec3::new(0.0, 1.2, -0.5), Vec3::Y),
    ));
    commands.spawn((
        RectLight {
            color: hex("#ead7b3"),
            intensity: 0.8 * 5.0 * 6.0 * PI,
            width: 5.0,
            height: 6.0,
            range: 10.0,
        },
        Transform::from_xyz(0.0, 3.7, -0.8).looking_at(Vec3::new(0.0, 0.0, -0.8), Vec3::Z),
    )); }
    commands.insert_resource(colliders);
}

struct Doors<'m> {
    wood: &'m Handle<StandardMaterial>,
    dark_wood: &'m Handle<StandardMaterial>,
    cream: &'m Handle<StandardMaterial>,
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
            kit.cuboid_in(door, [0.15, 2.95, 0.28], self.cream, [dx, 1.475, 0.06]);
        }
        kit.cuboid_in(door, [1.85, 0.16, 0.28], self.cream, [0.0, 2.9, 0.06]);
        kit.cuboid_in(door, [1.48, 0.035, 0.35], self.brass, [0.0, 0.025, 0.14]);
        kit.cuboid_in(door, [0.13, 0.3, 0.04], self.brass, [0.48, 1.2, 0.25]);
        let knob = kit.meshes.add(Sphere::new(0.065).mesh().uv(20, 14));
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
fn spawn_lamp(kit: &mut Kit) {
    let brass = kit.mat("#ad8a49", 0.32, 0.7);
    let center = Vec3::new(4.32, 1.58, 2.23);
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
    let shade_mesh = kit.mesh(meshes::frustum(0.19, 0.33, 0.42, 64, true));
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
        let torus = kit.meshes.add(Torus { minor_radius: 0.007, major_radius: radius }.mesh().minor_resolution(8).major_resolution(64));
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
        Transform::from_xyz(4.1, 1.55, center.z),
        LampLight(1.7 * CANDELA),
    ));
    commands.insert_resource(Lamp { on: true, shade: shade_material, shade_glow, bulb });
}

/// An opal glass pendant: the glass diffuses the bulb, and the bulb's point
/// light casts the room's main shadows.
fn spawn_pendant(kit: &mut Kit, brass: &Handle<StandardMaterial>, dark_wood: &Handle<StandardMaterial>) {
    kit.cylinder(0.07, 0.07, 0.08, brass, [0.0, 3.76, -0.8]);
    kit.cylinder(0.016, 0.016, 0.6, dark_wood, [0.0, 3.45, -0.8]);
    let diffusion = kit.image(textures::diffusion(), false);
    let opal = kit.materials.add(StandardMaterial {
        base_color: hex("#eee2c9"),
        emissive: glow("#ffe1ae", 1.35),
        emissive_texture: Some(diffusion),
        emissive_exposure_weight: 1.0,
        perceptual_roughness: 0.38,
        clearcoat: 0.3,
        clearcoat_perceptual_roughness: 0.3,
        ..default()
    });
    let globe = kit.meshes.add(Sphere::new(0.28).mesh().uv(40, 24));
    let pendant = kit.spawn(globe, &opal, Transform::from_xyz(0.0, 3.08, -0.8), None);
    kit.no_shadow(pendant);
    kit.commands.spawn((
        PointLight {
            color: hex("#ffdfb3"),
            intensity: 22.0 * CANDELA,
            range: 11.0,
            shadow_maps_enabled: !crate::flag("noshadow"),
            shadow_map_near_z: 0.12,
            ..default()
        },
        Transform::from_xyz(0.0, 3.08, -0.8),
    ));
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
