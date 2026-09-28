//! Reading the player's actions through their key bindings. Game code asks
//! "is Jump held?", never "is Space held?", so rebinding changes everything

use bevy::{ecs::system::SystemParam, prelude::*};

use super::definition::{Action, Binding, Settings};

/// The keyboard and mouse, read through the bindings
#[derive(SystemParam)]
pub struct ActionInput<'w> {
    keys: Res<'w, ButtonInput<KeyCode>>,
    mouse: Res<'w, ButtonInput<MouseButton>>,
    settings: Res<'w, Settings>,
}

impl ActionInput<'_> {
    /// Held down right now
    pub fn pressed(&self, action: Action) -> bool {
        match self.settings.controls.binding(action) {
            Binding::Key(key) => self.keys.pressed(key),
            Binding::Mouse(button) => self.mouse.pressed(button),
        }
    }

    /// Went down this frame
    pub fn just_pressed(&self, action: Action) -> bool {
        match self.settings.controls.binding(action) {
            Binding::Key(key) => self.keys.just_pressed(key),
            Binding::Mouse(button) => self.mouse.just_pressed(button),
        }
    }
}
