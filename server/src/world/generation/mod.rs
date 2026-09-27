//! Rolling world generation: chunks around players (and spawn) are generated in
//! the background, further out than anyone can see, so walking forward finds
//! them already made.
//!
//! - `settings.rs`: what the world looks like (tweak here)
//! - `terrain.rs`: turns settings + a chunk position into blocks
//! - `config.rs`: how far ahead and how many at once
//! - `logic.rs`: the background task plumbing

use bevy::prelude::*;

pub mod config;
mod logic;
pub mod settings;
pub mod terrain;

pub struct GenerationPlugin;

impl Plugin for GenerationPlugin {
    fn build(&self, app: &mut App) {
        // the generator is built from the settings once, not per chunk
        let settings = settings::WorldGenSettings::default();
        app.insert_resource(terrain::Terrain::new(&settings));
        app.insert_resource(settings);
        app.init_resource::<logic::Generating>();
        app.add_systems(
            FixedUpdate,
            (logic::start_generating, logic::finish_generating).chain(),
        );
    }
}
