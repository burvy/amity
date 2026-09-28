// Block surfaces: bevy's standard lit material, with two changes. The mesher
// merges matching faces into large rectangles (`mesher.rs`), so first, the texture
// has to repeat once per block across a rectangle. Each vertex carries:
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
//        in it still lights it fully. The sun is left to its shadows

#import bevy_pbr::{
    forward_io::{VertexOutput, FragmentOutput},
    pbr_fragment::pbr_input_from_standard_material,
    pbr_functions::{alpha_discard, apply_pbr_lighting, main_pass_post_lighting_processing},
}

// xy: a tile's size in the atlas. z: how far in from a tile's edges to sample
// (a share of the tile), so neighbouring tiles never bleed in
@group(#{MATERIAL_BIND_GROUP}) @binding(100) var<uniform> tiling: vec4<f32>;

@fragment
fn fragment(in: VertexOutput, @builtin(front_facing) is_front: bool) -> FragmentOutput {
    var tiled = in;
    let within = clamp(fract(in.uv), vec2(tiling.z), vec2(1.0 - tiling.z));
    tiled.uv = in.uv_b + within * tiling.xy;
    // the alpha is sky light, not see-through-ness: the texture's alpha is
    let sky = in.color.a;
    tiled.color = vec4(in.color.rgb, 1.0);

    // from here on, what the standard material does, with the sky's light
    // turned down by the sky light
    var pbr_input = pbr_input_from_standard_material(tiled, is_front);
    pbr_input.diffuse_occlusion *= sky;
    pbr_input.specular_occlusion *= sky;
    pbr_input.material.base_color = alpha_discard(pbr_input.material, pbr_input.material.base_color);
    var out: FragmentOutput;
    out.color = apply_pbr_lighting(pbr_input);
    out.color = main_pass_post_lighting_processing(pbr_input, out.color);
    return out;
}
