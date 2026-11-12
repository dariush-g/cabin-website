// Cel shading for the hall's materials: the standard lighting is computed as
// usual, then its brightness is snapped to a few flat bands.

import bevy_pbr::render::{
    forward_io::{VertexOutput, FragmentOutput},
    mesh_view_bindings::view,
    pbr_fragment::pbr_input_from_standard_material,
    pbr_functions::{alpha_discard, apply_pbr_lighting, main_pass_post_lighting_processing},
    pbr_types::STANDARD_MATERIAL_FLAGS_UNLIT_BIT,
};

/// Bands per unit of perceptual brightness.
const BANDS: f32 = 4.0;
/// Brightness is banded in roughly perceptual space so the steps look even.
const GAMMA: f32 = 2.2;

/// Snaps a brightness to the middle of its band, with a pixel-wide blend at
/// each edge so the steps do not shimmer.
fn band(light: f32) -> f32 {
    let x = pow(light, 1.0 / GAMMA) * BANDS - 0.5;
    let edge = clamp(fwidth(x), 0.001, 0.5);
    let step = floor(x) + smoothstep(0.5 - edge, 0.5 + edge, fract(x)) + 0.5;
    return pow(step / BANDS, GAMMA);
}

@fragment
fn fragment(vertex_output: VertexOutput, @builtin(front_facing) is_front: bool) -> FragmentOutput {
    var pbr_input = pbr_input_from_standard_material(vertex_output, is_front);
    pbr_input.material.base_color = alpha_discard(
        pbr_input.material.flags,
        pbr_input.material.alpha_cutoff,
        pbr_input.material.base_color,
    );
    let base = pbr_input.material.base_color;

    var out: FragmentOutput;
    if (pbr_input.material.flags & STANDARD_MATERIAL_FLAGS_UNLIT_BIT) == 0u {
        // Light the surface as if it were white and did not glow, so only the
        // light is banded and the surface's own texture stays intact.
        let emissive = pbr_input.material.emissive;
        pbr_input.material.base_color = vec4(1.0, 1.0, 1.0, base.a);
        pbr_input.material.emissive = vec4(0.0);
        let light = apply_pbr_lighting(pbr_input);
        pbr_input.material.base_color = base;
        pbr_input.material.emissive = emissive;

        let brightness = dot(light.rgb, vec3(0.2126, 0.7152, 0.0722));
        // Keep the light's tint; fall back to neutral where there is none.
        let tint = mix(vec3(1.0), light.rgb / max(brightness, 0.0001), saturate(brightness * 2000.0));
        let glow = emissive.rgb * light.a * mix(1.0, view.exposure, emissive.a);
        out.color = vec4(base.rgb * tint * band(brightness) + glow, light.a);
    } else {
        out.color = base;
    }
    out.color = main_pass_post_lighting_processing(pbr_input, out.color);
    return out;
}
