// Block surfaces: bevy's standard lit material, with three changes.
//
// First, the mesher merges matching faces into large rectangles (`mesher.rs`),
// so the texture has to repeat once per block across a rectangle. Each vertex
// carries:
//   uv:   its position across the face, in blocks (1.0 = one block)
//   uv_b: the top-left corner of the block's tile in the texture atlas
// The fraction of `uv` is where we are within one block, which picks the pixel
// within that tile.
//
// Second, the vertex colour carries the shading worked out when meshing:
//   rgb: ambient occlusion, which the standard material multiplies the
//        texture by
//   a:   sky light (0 to 1): how much of the sky's light reaches here
//        (`light.rs`). It turns down only the light coming from the sky, its
//        ambient light and reflections, so a cave is black but a flashlight
//        in it still lights it fully.
//
// Third, sun and moon light is taken away where it can't reach: behind the
// skyline (horizon shadows, `world/horizon`), and where the sky can't reach
// either (deep in caves, where the shadow map may not cover). Lights add up,
// so after the standard lighting, the hidden part of each one is worked out
// the same way bevy does and subtracted.

#import bevy_pbr::{
    forward_io::{VertexOutput, FragmentOutput},
    lighting,
    lighting::LAYER_BASE,
    mesh_types::MESH_FLAGS_SHADOW_RECEIVER_BIT,
    mesh_view_bindings as view_bindings,
    mesh_view_types,
    pbr_fragment::pbr_input_from_standard_material,
    pbr_functions,
    pbr_functions::{alpha_discard, apply_pbr_lighting, main_pass_post_lighting_processing},
    pbr_types::PbrInput,
    shadows,
}

const TAU: f32 = 6.28318530718;
const HALF_PI: f32 = 1.57079632679;

// xy: a tile's size in the atlas. z: how far in from a tile's edges to sample
// (a share of the tile), so neighbouring tiles never bleed in
@group(#{MATERIAL_BIND_GROUP}) @binding(100) var<uniform> tiling: vec4<f32>;
// The horizon map: per 2 m cell, how high the skyline is in 16 directions (0
// is flat, 1 straight up), 4 per layer: directions 0-3 in layer 0, 4-7 in
// layer 1, and so on. Direction k points k * 22.5 degrees from +X towards +Z.
// It repeats across the world: a cell's texel is its position wrapped to the
// map's size
@group(#{MATERIAL_BIND_GROUP}) @binding(101) var horizon_map: texture_2d_array<f32>;
@group(#{MATERIAL_BIND_GROUP}) @binding(102) var horizon_sampler: sampler;
// x: 1 / the map's width in metres. y: how far light fades as it sets behind
// the skyline, in radians. z: how far the shadow map reaches, in metres (0
// when off). w: the share of that over which its shadows fade into the
// horizon's
@group(#{MATERIAL_BIND_GROUP}) @binding(103) var<uniform> horizon: vec4<f32>;

@fragment
fn fragment(in: VertexOutput, @builtin(front_facing) is_front: bool) -> FragmentOutput {
    var tiled = in;
    let within = clamp(fract(in.uv), vec2(tiling.z), vec2(1.0 - tiling.z));
    tiled.uv = in.uv_b + within * tiling.xy;
    // the alpha is sky light, not see-through-ness: the texture's alpha is
    let sky = in.color.a;
    tiled.color = vec4(in.color.rgb, 1.0);

    // the standard material, with the sky's light turned down by the sky light
    var pbr_input = pbr_input_from_standard_material(tiled, is_front);
    pbr_input.diffuse_occlusion *= sky;
    pbr_input.specular_occlusion *= sky;
    pbr_input.material.base_color = alpha_discard(pbr_input.material, pbr_input.material.base_color);
    var out: FragmentOutput;
    out.color = apply_pbr_lighting(pbr_input);

    // minus the sun and moon light that can't get here
    let hidden = hidden_light(pbr_input, sky);
    out.color = vec4(max(out.color.rgb - view_bindings::view.exposure * hidden, vec3(0.0)), out.color.a);

    out.color = main_pass_post_lighting_processing(pbr_input, out.color);
    return out;
}

// The directional light (sun, moon) that apply_pbr_lighting added here, less
// what should reach here. Bevy lit it as much as its shadow map allows (`shadow`
// below); what should reach is that, faded out towards the shadow map's reach,
// times what the skyline and the sky leave (`visible`). Negative where the
// fade gives back light the shadow map took (the fade band)
fn hidden_light(in: PbrInput, sky: f32) -> vec3<f32> {
    // the skyline around here, in the 16 directions
    let uv = in.world_position.xz * horizon.x;
    let a = textureSampleLevel(horizon_map, horizon_sampler, uv, 0, 0.0);
    let b = textureSampleLevel(horizon_map, horizon_sampler, uv, 1, 0.0);
    let c = textureSampleLevel(horizon_map, horizon_sampler, uv, 2, 0.0);
    let d = textureSampleLevel(horizon_map, horizon_sampler, uv, 3, 0.0);
    var skyline = array<f32, 16>(
        a.r, a.g, a.b, a.a, b.r, b.g, b.b, b.a,
        c.r, c.g, c.b, c.a, d.r, d.g, d.b, d.a,
    );
    // where the sky can't reach (deep in a cave) the sun can't either. Fades
    // in over the darkest quarter of sky light
    let open = smoothstep(0.0, 0.25, sky);

    // the same lighting input apply_pbr_lighting builds
    let base = in.material.base_color;
    let perceptual_roughness = in.material.perceptual_roughness;
    let NdotV = max(dot(in.N, in.V), 0.0001);
    var input: lighting::LightingInput;
    input.layers[LAYER_BASE].NdotV = NdotV;
    input.layers[LAYER_BASE].N = in.N;
    input.layers[LAYER_BASE].R = reflect(-in.V, in.N);
    input.layers[LAYER_BASE].perceptual_roughness = perceptual_roughness;
    input.layers[LAYER_BASE].roughness = lighting::perceptualRoughnessToRoughness(perceptual_roughness);
    input.P = in.world_position.xyz;
    input.V = in.V;
    input.diffuse_color = pbr_functions::calculate_diffuse_color(
        base.rgb,
        in.material.metallic,
        in.material.specular_transmission,
        in.material.diffuse_transmission,
    );
    input.metallic = in.material.metallic;
    input.F0_dielectric = pbr_functions::calculate_F0_dielectric(in.material.reflectance);
    input.F0_metallic = base.rgb;
    input.F_ab = lighting::F_AB(perceptual_roughness, NdotV);

    let view_z = dot(vec4<f32>(
        view_bindings::view.view_from_world[0].z,
        view_bindings::view.view_from_world[1].z,
        view_bindings::view.view_from_world[2].z,
        view_bindings::view.view_from_world[3].z
    ), in.world_position);

    // 0 where the shadow map is trusted, rising to 1 at its reach, beyond
    // which there's only the horizon
    let reach = horizon.z;
    var fade = 1.0;
    if reach > 0.0 {
        fade = smoothstep(reach * (1.0 - horizon.w), reach, -view_z);
    }

    var hidden = vec3(0.0);
    for (var i: u32 = 0u; i < view_bindings::lights.n_directional_lights; i = i + 1u) {
        let light = &view_bindings::lights.directional_lights[i];
        let to_light = (*light).direction_to_light;

        // how much of this light the skyline and the lack of sky leave
        let elevation = asin(clamp(to_light.y, -1.0, 1.0));
        let turn = fract(atan2(to_light.z, to_light.x) / TAU) * 16.0;
        let first = u32(floor(turn)) % 16u;
        let height = mix(skyline[first], skyline[(first + 1u) % 16u], fract(turn)) * HALF_PI;
        let visible = smoothstep(height - horizon.y, height + horizon.y, elevation) * open;
        // in full view, and not in the fade band (where the shadow map is
        // trusted, or beyond its reach, where it lit everything): as bevy lit it
        if visible >= 0.999 && (fade <= 0.0 || fade >= 1.0) {
            continue;
        }

        // facing away from the light, it adds nothing: nothing to take away,
        // and no need for the (costly) shadow map lookup
        let light_contrib = lighting::directional_light(i, &input, true);
        if all(light_contrib <= vec3(0.0)) {
            continue;
        }
        var shadow = 1.0;
        if ((in.flags & MESH_FLAGS_SHADOW_RECEIVER_BIT) != 0u
                && ((*light).flags & mesh_view_types::DIRECTIONAL_LIGHT_FLAGS_SHADOWS_ENABLED_BIT) != 0u) {
            shadow = shadows::fetch_directional_shadow(i, in.world_position, in.world_normal, view_z, in.frag_coord.xy);
        }
        let reaching = mix(shadow, 1.0, fade) * visible;
        hidden += light_contrib * (shadow - reaching);
    }
    return hidden;
}
