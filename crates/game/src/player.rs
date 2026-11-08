//! The visitor: looking, walking, and using what they face.

use bevy::prelude::*;

use crate::bridge::{self, Command, Move};
use crate::hall::{self, Action, Colliders, Lamp, LampLight, Target};

const START: Vec3 = Vec3::new(0.8, 1.65, 6.6);
const START_YAW: f32 = 0.11;
const START_PITCH: f32 = -0.035;
const WALK_SPEED: f32 = 2.0;
/// Holding Shift: a brisk walk, not a run.
const HURRY_SPEED: f32 = 3.1;
/// How quickly speed eases toward the target, per second.
const ACCELERATION: f32 = 9.0;
/// Metres per footstep (an unhurried two steps a second at walking speed);
/// the head dips and rises once per step.
const STRIDE: f32 = 1.0;
const REACH: f32 = 2.8;

#[derive(Resource)]
pub struct Visitor {
    yaw: f32,
    pitch: f32,
    active: bool,
    moves: [bool; 4],
    hurrying: bool,
    wants_to_interact: bool,
    target: Option<Entity>,
    /// Eye position without head bob; collisions use this.
    eye: Vec3,
    /// Current walking velocity on the floor plane (x, z).
    velocity: Vec2,
    /// Advances by π per footstep.
    stride_phase: f32,
    /// 0 standing still, 1 walking: fades the bob in and out.
    bob_weight: f32,
    bob: bool,
}

impl Visitor {
    /// `bob` is off for visitors who prefer reduced motion.
    pub fn new(bob: bool) -> Self {
        Self {
            yaw: START_YAW,
            pitch: START_PITCH,
            active: false,
            moves: [false; 4],
            hurrying: false,
            wants_to_interact: false,
            target: None,
            eye: START,
            velocity: Vec2::ZERO,
            stride_phase: 0.0,
            bob_weight: 0.0,
            bob,
        }
    }

    fn held(&self, direction: Move) -> f32 {
        let index = match direction {
            Move::Forward => 0,
            Move::Backward => 1,
            Move::Left => 2,
            Move::Right => 3,
        };
        if self.moves[index] { 1.0 } else { 0.0 }
    }

    fn set(&mut self, direction: Move, pressed: bool) {
        let index = match direction {
            Move::Forward => 0,
            Move::Backward => 1,
            Move::Left => 2,
            Move::Right => 3,
        };
        self.moves[index] = pressed;
    }
}

pub fn camera_transform() -> Transform {
    Transform::from_translation(START).with_rotation(view_rotation(START_YAW, START_PITCH))
}

fn view_rotation(yaw: f32, pitch: f32) -> Quat {
    Quat::from_euler(EulerRot::YXZ, yaw, pitch, 0.0)
}

/// Applies input the browser forwarded since the last frame.
pub fn receive_commands(mut visitor: ResMut<Visitor>) {
    for command in bridge::drain() {
        match command {
            Command::Active(active) => {
                visitor.active = active;
                if !active {
                    visitor.moves = [false; 4];
                    visitor.hurrying = false;
                }
            }
            Command::Move(direction, pressed) => visitor.set(direction, pressed),
            Command::ClearMoves => {
                visitor.moves = [false; 4];
                visitor.hurrying = false;
            }
            Command::Hurry(hurrying) => visitor.hurrying = hurrying,
            Command::Look { yaw, pitch } => {
                visitor.yaw += yaw;
                visitor.pitch = (visitor.pitch + pitch).clamp(-1.25, 1.25);
            }
            Command::Interact => visitor.wants_to_interact = true,
            Command::Reset => {
                visitor.yaw = START_YAW;
                visitor.pitch = START_PITCH;
                visitor.eye = START;
                visitor.velocity = Vec2::ZERO;
            }
        }
    }
}

/// Walks relative to the view direction, sliding along furniture and walls,
/// with a gentle head bob while moving.
pub fn walk(time: Res<Time>, mut visitor: ResMut<Visitor>, colliders: Res<Colliders>, mut camera: Single<&mut Transform, With<Camera3d>>) {
    let dt = time.delta_secs().min(0.05);
    let mut forward = visitor.held(Move::Forward) - visitor.held(Move::Backward);
    let mut side = visitor.held(Move::Right) - visitor.held(Move::Left);
    let length = forward.hypot(side);
    if length > 0.0 {
        forward /= length;
        side /= length;
    }
    let speed = if visitor.hurrying { HURRY_SPEED } else { WALK_SPEED };
    let (sin, cos) = visitor.yaw.sin_cos();
    let wanted = if visitor.active {
        Vec2::new(-sin * forward + cos * side, -cos * forward - sin * side) * speed
    } else {
        Vec2::ZERO
    };
    visitor.velocity = visitor.velocity.lerp(wanted, 1.0 - (-ACCELERATION * dt).exp());

    // Slide along obstacles one axis at a time; only real movement counts
    // toward footsteps, so walking into a wall doesn't bob.
    let before = visitor.eye;
    let step = visitor.velocity * dt;
    if colliders.allows(visitor.eye.x + step.x, visitor.eye.z) {
        visitor.eye.x += step.x;
    } else {
        visitor.velocity.x = 0.0;
    }
    if colliders.allows(visitor.eye.x, visitor.eye.z + step.y) {
        visitor.eye.z += step.y;
    } else {
        visitor.velocity.y = 0.0;
    }
    let moved = (visitor.eye - before).length();
    let footfalls_before = footfalls(visitor.stride_phase);
    visitor.stride_phase += moved / STRIDE * std::f32::consts::PI;
    let speed_ratio = moved / dt.max(1e-4) / WALK_SPEED;
    if footfalls(visitor.stride_phase) != footfalls_before {
        bridge::step(speed_ratio.clamp(0.6, 1.4), hall::on_rug(visitor.eye.x, visitor.eye.z));
    }
    // Full bob at walking pace, slightly more when hurrying.
    let pace = speed_ratio.min(1.2);
    let weight_target = if visitor.bob { pace } else { 0.0 };
    visitor.bob_weight += (weight_target - visitor.bob_weight) * (1.0 - (-6.0 * dt).exp());

    // A gentle dip at each footfall and a faint sway toward the planted foot.
    let phase = visitor.stride_phase;
    let weight = visitor.bob_weight;
    let rise = -0.012 * weight * (1.0 - (2.0 * phase).cos()) / 2.0;
    let sway = 0.005 * weight * phase.sin();
    let right = Vec3::new(cos, 0.0, -sin);
    camera.translation = visitor.eye + Vec3::Y * rise + right * sway;
    camera.rotation = view_rotation(visitor.yaw, visitor.pitch);
}

/// Footfalls taken by a given stride phase. A foot lands at the bottom of
/// each head dip, a quarter of a cycle into every step.
fn footfalls(phase: f32) -> i32 {
    ((phase - std::f32::consts::FRAC_PI_2) / std::f32::consts::PI).floor() as i32
}

/// Distance along the ray to a box of half-size `half` in `to_local` space.
fn ray_box(origin: Vec3, direction: Vec3, to_local: Mat4, half: Vec3) -> Option<f32> {
    let o = to_local.transform_point3(origin);
    let d = to_local.transform_vector3(direction);
    let (mut near, mut far) = (f32::NEG_INFINITY, f32::INFINITY);
    for axis in 0..3 {
        if d[axis].abs() < 1e-8 {
            if o[axis].abs() > half[axis] {
                return None;
            }
            continue;
        }
        let a = (-half[axis] - o[axis]) / d[axis];
        let b = (half[axis] - o[axis]) / d[axis];
        near = near.max(a.min(b));
        far = far.min(a.max(b));
    }
    // The direction is transformed without renormalizing, so `near` is
    // already measured in world units.
    (near <= far && far >= 0.0).then_some(near.max(0.0))
}

#[allow(clippy::too_many_arguments)]
pub fn aim_and_interact(
    mut visitor: ResMut<Visitor>,
    camera: Single<&GlobalTransform, With<Camera3d>>,
    targets: Query<(Entity, &GlobalTransform, &Target)>,
    mut lamp: ResMut<Lamp>,
    mut lights: Query<(&LampLight, Option<&mut PointLight>, Option<&mut SpotLight>, Option<&mut RectLight>)>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    mut visibility: Query<&mut Visibility>,
) {
    let wants_to_interact = std::mem::take(&mut visitor.wants_to_interact);
    let faced = if visitor.active {
        let origin = camera.translation();
        let direction = camera.forward().as_vec3();
        targets
            .iter()
            .filter_map(|(entity, transform, target)| {
                let to_local = transform.affine().inverse().into();
                ray_box(origin, direction, to_local, target.half)
                    .filter(|distance| *distance <= REACH)
                    .map(|distance| (entity, distance))
            })
            .min_by(|a, b| a.1.total_cmp(&b.1))
            .map(|(entity, _)| entity)
    } else {
        None
    };
    if faced != visitor.target {
        visitor.target = faced;
        let label = faced.and_then(|entity| targets.get(entity).ok()).map(|(_, _, t)| t.label.as_str());
        bridge::target(label);
    }
    let Some((_, _, target)) = faced.filter(|_| wants_to_interact).and_then(|e| targets.get(e).ok()) else {
        return;
    };
    match target.action {
        Action::Door(name) => bridge::toast(&format!(
            "{name} — this room isn’t open yet. For now, make yourself at home in the entrance hall."
        )),
        Action::Note => bridge::toast("“Welcome home. Leave the weather outside. There’s no hurry here.”"),
        Action::Lamp => {
            let message = hall::toggle_lamp(&mut lamp, &mut lights, &mut materials, &mut visibility);
            bridge::toast(message);
        }
    }
}

/// Tells the page the hall is ready once a few frames have rendered.
pub fn announce_ready(mut frames: Local<u32>) {
    *frames += 1;
    if *frames == 3 {
        bridge::ready();
    }
}
