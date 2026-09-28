//! The one font all UI text uses. To use your own: put the .ttf or .otf in
//! assets/fonts/ and set `FONT_FILE` below, e.g.
//! `Some(include_bytes!("../../assets/fonts/m5x7.ttf"))`. It's embedded in the
//! build like the block atlas, so native and web load it the same way

use bevy::prelude::*;

/// The font file, or None for Bevy's built-in font
const FONT_FILE: Option<&[u8]> = Some(include_bytes!("../../assets/fonts/PixelOperatorSC.ttf"));

/// The UI font. Every text in the UI takes its `TextFont::font` from here
#[derive(Resource)]
pub struct UiFont(pub Handle<Font>);

impl FromWorld for UiFont {
    fn from_world(world: &mut World) -> Self {
        match FONT_FILE {
            Some(bytes) => {
                let font = Font::from_bytes(bytes.to_vec());
                Self(world.resource_mut::<Assets<Font>>().add(font))
            }
            // the default handle is Bevy's built-in font
            None => Self(Handle::default()),
        }
    }
}
