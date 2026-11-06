//! The House: a walkable entrance hall during a snowstorm, rendered with Bevy.
//!
//! The web page owns the overlay, pointer capture and ambient sound; see
//! [`bridge`] for how the two sides talk.

mod bridge;
mod hall;
mod meshes;
mod player;
mod textures;
mod winter;

use bevy::anti_alias::fxaa::Fxaa;
use bevy::camera::Exposure;
use bevy::light::PointLightShadowMap;
use bevy::pbr::{DistanceFog, FogFalloff};
use bevy::prelude::*;
use bevy::render::view::Tonemapping;
use bevy::window::WindowResolution;
use wasm_bindgen::prelude::*;

pub use bridge::{clear_moves, interact, look, reset_view, set_active, set_move};

const SKY: &str = "#a6b4bf";

// TEMPORARY bisect flags from the URL query.
pub fn flag(name: &str) -> bool {
    web_sys::window().and_then(|w| w.location().search().ok()).is_some_and(|q| q.contains(name))
}

/// Starts the hall in `#house-canvas`. Callbacks: `on_ready()` once the first
/// frames have rendered, `on_target(label | null)` when the object in view
/// changes, `on_toast(message)`, and `on_error(message)` if the app panics.
///
/// This hands control to the browser's event loop by throwing; the caller
/// should ignore the "Using exceptions for control flow" error.
#[wasm_bindgen]
pub fn run(
    on_ready: js_sys::Function,
    on_target: js_sys::Function,
    on_toast: js_sys::Function,
    on_error: js_sys::Function,
    reduced_motion: bool,
    pixel_ratio: f32,
) {
    bridge::install(bridge::Callbacks { on_ready, on_target, on_toast, on_error });

    let mut app = App::new();
    app.add_plugins(DefaultPlugins.set(WindowPlugin {
        primary_window: Some(Window {
            canvas: Some("#house-canvas".into()),
            fit_canvas_to_parent: true,
            prevent_default_event_handling: false,
            // High-density screens gain little past this, at a large fill-rate cost.
            resolution: WindowResolution::default().with_scale_factor_override(if flag("lowres") { 1.0 } else { pixel_ratio.clamp(1.0, 1.75) }),
            ..default()
        }),
        ..default()
    }));

    // Keep the console report Bevy's log plugin installs, and tell the page.
    let console_hook = std::panic::take_hook();
    std::panic::set_hook(Box::new(move |info| {
        console_hook(info);
        bridge::fail(&info.to_string());
    }));

    app.insert_resource(ClearColor(hall::hex(SKY)))
        .insert_resource(PointLightShadowMap { size: 1024 })
        .insert_resource(player::Visitor::default())
        .insert_resource(winter::Storm { time: 0.0, animate: !reduced_motion })
        .add_systems(Startup, (spawn_camera, hall::setup))
        .add_systems(
            Update,
            (
                player::receive_commands,
                player::walk,
                player::aim_and_interact,
                winter::storm,
                player::announce_ready,
            )
                .chain(),
        )
        .run();
}

fn spawn_camera(mut commands: Commands) {
    // Matches Three.js's pre-tone-mapping scale: 2^-ev / 1.2 = DISPLAY_GAIN.
    let ev100 = (1.0 / (1.2 * hall::DISPLAY_GAIN)).log2();
    let camera = commands.spawn((
        Camera3d::default(),
        Projection::from(PerspectiveProjection {
            fov: 65f32.to_radians(),
            near: 0.05,
            far: 100.0,
            ..default()
        }),
        Tonemapping::AcesFitted,
        // FXAA instead of 4× MSAA: the lighting is fill-rate heavy on WebGL 2.
        Msaa::Off,
        Fxaa::default(),
        Exposure { ev100 },
        // Distance haze belongs beyond the windows; keep the entrance air clear.
        DistanceFog {
            color: hall::hex(SKY),
            falloff: FogFalloff::Linear { start: 9.0, end: 42.0 },
            ..default()
        },
        // Stands in for the cool sky and warm floor bounce of a hemisphere
        // light; Bevy's ambient term omits the Lambert 1/π, hence the scale.
        AmbientLight {
            color: hall::hex("#a7aaa4"),
            brightness: 0.6 / std::f32::consts::PI,
            ..default()
        },
        player::camera_transform(),
    )).id();
    if flag("hw2x2") {
        commands.entity(camera).insert(bevy::light::ShadowFilteringMethod::Hardware2x2);
    }
}
