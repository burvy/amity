// Block surfaces in the depth-only passes: the shadow maps (and a depth
// prepass, if one is ever turned on). Solid blocks need nothing from here; for
// see-through blocks like leaves, it cuts the same holes the main pass does
// (`block.wgsl`), so their shadows are dappled rather than solid squares.
//
// Bevy's own version (`pbr_prepass.wgsl`) would look the texture up at the
// raw UV, which here is a position across a merged rectangle in blocks, not a
// spot in the atlas. So first the UV is tiled the same way `block.wgsl` does
// it, then bevy's alpha check runs as normal

#import bevy_pbr::{
    pbr_prepass_functions,
    prepass_io,
}

// xy: a tile's size in the atlas. z: how far in from a tile's edges to sample
@group(#{MATERIAL_BIND_GROUP}) @binding(100) var<uniform> tiling: vec4<f32>;

// The UV the texture is looked up at: the tile's corner, plus how far across
// one block this fragment is
fn tiled(in: prepass_io::VertexOutput) -> prepass_io::VertexOutput {
    var out = in;
#ifdef VERTEX_UVS_B
    let within = clamp(fract(in.uv), vec2(tiling.z), vec2(1.0 - tiling.z));
    out.uv = in.uv_b + within * tiling.xy;
#endif
    return out;
}

#ifdef PREPASS_FRAGMENT
@fragment
fn fragment(in: prepass_io::VertexOutput) -> prepass_io::FragmentOutput {
    pbr_prepass_functions::prepass_alpha_discard(tiled(in));

    var out: prepass_io::FragmentOutput;
#ifdef UNCLIPPED_DEPTH_ORTHO_EMULATION
    out.frag_depth = in.unclipped_depth;
#endif
#ifdef NORMAL_PREPASS
    out.normal = vec4(in.world_normal * 0.5 + vec3(0.5), 1.0);
#endif
#ifdef MOTION_VECTOR_PREPASS
    out.motion_vector = pbr_prepass_functions::calculate_motion_vector(
        in.world_position,
        in.previous_world_position,
    );
#endif
    return out;
}
#else
@fragment
fn fragment(in: prepass_io::VertexOutput) {
    pbr_prepass_functions::prepass_alpha_discard(tiled(in));
}
#endif
