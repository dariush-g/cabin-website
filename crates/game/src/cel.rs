//! Cel shading: every lit surface uses the standard material with its
//! lighting snapped to flat bands (see `cel.wesl`).

use bevy::asset::embedded_asset;
use bevy::pbr::{ExtendedMaterial, MaterialExtension};
use bevy::prelude::*;
use bevy::render::render_resource::AsBindGroup;
use bevy::shader::ShaderRef;

/// The material every surface in the hall uses.
pub type Toon = ExtendedMaterial<StandardMaterial, Cel>;

#[derive(Asset, AsBindGroup, Reflect, Debug, Clone, Default)]
pub struct Cel {}

impl MaterialExtension for Cel {
    fn fragment_shader() -> ShaderRef {
        "embedded://game/cel.wesl".into()
    }
}

/// Wraps a standard material so it is drawn cel shaded.
pub fn toon(base: StandardMaterial) -> Toon {
    Toon { base, extension: Cel {} }
}

pub struct CelPlugin;

impl Plugin for CelPlugin {
    fn build(&self, app: &mut App) {
        embedded_asset!(app, "cel.wesl");
        app.add_plugins(MaterialPlugin::<Toon>::default());
    }
}
