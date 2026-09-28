//! How chunk meshes look

/// Ambient occlusion: how bright a face's corner is, by how many of the 3
/// blocks around it (two sides and the diagonal) are solid: all of them (or
/// both sides) first, none last. Multiplies the texture colour, in linear
/// light, so 0.5 looks about 73% as bright, not half
pub const AO_BRIGHTNESS: [f32; 4] = [0.3, 0.5, 0.72, 1.0];
