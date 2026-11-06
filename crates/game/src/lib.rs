//! Dariush Ghassemieh's entrance hall: a walkable hall during a snowstorm, rendered with Bevy.
//!
//! The web page owns the overlay, pointer capture and ambient sound; see
//! [`bridge`] for how the two sides talk.

mod bridge;
mod hall;
mod meshes;
mod player;
mod textures;
mod winter;

use bevy::camera::Exposure;
use bevy::light::PointLightShadowMap;
use bevy::pbr::{DistanceFog, FogFalloff};
use bevy::post_process::effect_stack::ChromaticAberration;
use bevy::prelude::*;
use bevy::render::view::Tonemapping;
use bevy::window::WindowResolution;
use wasm_bindgen::prelude::*;

pub use bridge::{clear_moves, interact, look, reset_view, set_active, set_hurry, set_move};

const SKY: &str = "#1c2433";

// TEMPORARY bisect flags from the URL query.
pub fn flag(name: &str) -> bool {
    web_sys::window().and_then(|w| w.location().search().ok()).is_some_and(|q| q.contains(name))
}

/// Starts the hall in `#house-canvas`. The page caps the canvas's resolution
/// and scales it up smoothly, which keeps fill-rate low on large screens.
/// Callbacks: `on_ready()` once the first
/// frames have rendered, `on_target(label | null)` when the object in view
/// changes, `on_toast(message)`, `on_step(loudness, onRug)` at each footfall,
/// and `on_error(message)` if the app panics.
///
/// This hands control to the browser's event loop by throwing; the caller
/// should ignore the "Using exceptions for control flow" error.
#[wasm_bindgen]
pub fn run(
    on_ready: js_sys::Function,
    on_target: js_sys::Function,
    on_toast: js_sys::Function,
    on_step: js_sys::Function,
    on_error: js_sys::Function,
    reduced_motion: bool,
) {
    bridge::install(bridge::Callbacks { on_ready, on_target, on_toast, on_step, on_error });

    let mut app = App::new();
    app.add_plugins(DefaultPlugins.set(WindowPlugin {
        primary_window: Some(Window {
            canvas: Some("#house-canvas".into()),
            fit_canvas_to_parent: true,
            prevent_default_event_handling: false,
            resolution: WindowResolution::default().with_scale_factor_override(1.0),
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
        .insert_resource(PointLightShadowMap { size: 512 })
        .insert_resource(player::Visitor::new(!reduced_motion))
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
        // Smooth polygon edges; cheap now that the canvas resolution is capped.
        Msaa::Sample4,
        Exposure { ev100 },
        // Snowy night swallows the forest a few trees out.
        DistanceFog {
            color: hall::hex(SKY),
            falloff: FogFalloff::Linear { start: 7.0, end: 30.0 },
            ..default()
        },
        // Just enough cool night light to find the corners; Bevy's ambient
        // term omits the Lambert 1/π, hence the scale.
        AmbientLight {
            color: hall::hex("#5a6680"),
            brightness: 0.18 / std::f32::consts::PI,
            ..default()
        },
        player::camera_transform(),
    )).id();
    // A faint colour fringe toward the edges of the frame, like an old lens.
    // TEMPORARY: `?noca` turns it off for comparison.
    if !flag("noca") {
        commands.entity(camera).insert(ChromaticAberration { intensity: 0.006, ..default() });
    }
    if flag("hw2x2") {
        commands.entity(camera).insert(bevy::light::ShadowFilteringMethod::Hardware2x2);
    }
}
