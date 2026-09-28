//! How chunk meshes look

/// Ambient occlusion: how bright a face's corner is, by how many of the 3
/// blocks around it (two sides and the diagonal) are solid: all of them (or
/// both sides) first, none last. Multiplies the texture colour, in linear
/// light, so 0.5 looks about 73% as bright, not half
pub const AO_BRIGHTNESS: [f32; 4] = [0.3, 0.5, 0.72, 1.0];

/// Sky light: every block with open sky above it gets the full level, and each
/// block the light travels from there (into caves, under overhangs, down
/// through water) loses one. So light reaches this many blocks (half a metre
/// each) into the dark; beyond that it's pitch black
pub const SKY_LIGHT_MAX: u8 = 15;

/// How sky light fades with its level: the light the sky gives a surface is
/// (level / max) to this power. 1 would fade evenly; higher darkens faster
/// away from the opening, the way indirect light really falls off
pub const SKY_LIGHT_POWER: f32 = 2.0;
