// Block surfaces: bevy's standard lit material, with one change. The mesher
// merges matching faces into large rectangles (`mesher.rs`), so the texture
// has to repeat once per block across a rectangle. Each vertex carries:
//   uv:   its position across the face, in blocks (1.0 = one block)
//   uv_b: the top-left corner of the block's tile in the texture atlas
// The fraction of `uv` is where we are within one block, which picks the pixel
// within that tile. Ambient occlusion arrives as the vertex colour, which the
// standard material already multiplies the texture by.

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

    // from here on, exactly what the standard material does
    var pbr_input = pbr_input_from_standard_material(tiled, is_front);
    pbr_input.material.base_color = alpha_discard(pbr_input.material, pbr_input.material.base_color);
    var out: FragmentOutput;
    out.color = apply_pbr_lighting(pbr_input);
    out.color = main_pass_post_lighting_processing(pbr_input, out.color);
    return out;
}
