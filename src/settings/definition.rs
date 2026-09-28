//! What the player can set: graphics and controls. Saved between sessions (see
//! `storage.rs`); every field has a default, so a settings file from an older
//! version, missing newer fields, still loads

use std::collections::BTreeMap;

use bevy::{light::ShadowFilteringMethod, prelude::*};
use serde::{Deserialize, Serialize};

#[derive(Resource, Clone, PartialEq, Debug, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct Settings {
    pub graphics: Graphics,
    pub controls: Controls,
}

#[derive(Clone, PartialEq, Debug, Serialize, Deserialize)]
#[serde(default)]
pub struct Graphics {
    /// Chunks (16 m each) loaded around you, sideways. The server caps it
    pub render_distance: u8,
    /// Share of the screen's resolution the world is drawn at, 1.0 = full.
    /// Lower is faster (0.75 is about half the pixels) and softer. The UI stays
    /// sharp either way
    pub render_scale: f32,
    /// Vertical field of view, degrees
    pub fov: f32,
    pub anti_aliasing: AntiAliasing,
    pub shadows: ShadowQuality,
    pub sky_light: SkyLightQuality,
    /// A glow around bright light, like the sun
    pub bloom: bool,
    pub frame_limit: FrameLimit,
    /// Size of all UI, 1.0 = as designed
    pub ui_scale: f32,
}

impl Default for Graphics {
    fn default() -> Self {
        Self {
            render_distance: 8,
            render_scale: 1.0,
            fov: 70.0,
            anti_aliasing: AntiAliasing::Fxaa,
            shadows: ShadowQuality::Medium,
            sky_light: SkyLightQuality::High,
            bloom: true,
            frame_limit: FrameLimit::VSync,
            ui_scale: 1.0,
        }
    }
}

/// Smoothing jagged edges. Cheapest to dearest: FXAA (a blur along edges),
/// SMAA (sharper), MSAA (draws edges 4 times over: costly on weak GPUs)
#[derive(Clone, Copy, PartialEq, Eq, Debug, Serialize, Deserialize)]
pub enum AntiAliasing {
    Off,
    Fxaa,
    Smaa,
    Msaa,
}

impl AntiAliasing {
    pub const ALL: [Self; 4] = [Self::Off, Self::Fxaa, Self::Smaa, Self::Msaa];

    pub fn label(self) -> &'static str {
        match self {
            Self::Off => "Off",
            Self::Fxaa => "FXAA",
            Self::Smaa => "SMAA",
            Self::Msaa => "MSAA 4x",
        }
    }
}

/// Sun shadows: how many cascades, how far they reach, and how sharp
#[derive(Clone, Copy, PartialEq, Eq, Debug, Serialize, Deserialize)]
pub enum ShadowQuality {
    Off,
    Low,
    Medium,
    High,
}

impl ShadowQuality {
    pub const ALL: [Self; 4] = [Self::Off, Self::Low, Self::Medium, Self::High];

    /// (cascades, metres they reach, shadow map pixels per side). Each
    /// cascade draws the terrain once more every frame, so cascades cost the
    /// most; the first one (the sharpest) covers only the nearest few metres,
    /// so 2048 pixels is already sharp there. They don't need to reach far:
    /// beyond them, horizon shadows (`world/horizon`) darken the terrain, so
    /// the shadow map is for the detail close by
    pub fn cascades(self) -> Option<(usize, f32, usize)> {
        match self {
            Self::Off => None,
            Self::Low => Some((1, 16.0, 128)),
            Self::Medium => Some((2, 64.0, 1024)),
            Self::High => Some((2, 64.0, 2048)),
        }
    }

    /// Softening shadow edges: High blurs them over many samples (soft, and
    /// dearer per pixel); the rest use the GPU's built-in 2x2 blend
    pub fn filtering(self) -> ShadowFilteringMethod {
        match self {
            Self::High => ShadowFilteringMethod::Gaussian,
            _ => ShadowFilteringMethod::Hardware2x2,
        }
    }
}

/// The sky's lighting of the world: pixels per side of the sky cubemap redone
/// every frame
#[derive(Clone, Copy, PartialEq, Eq, Debug, Serialize, Deserialize)]
pub enum SkyLightQuality {
    Low,
    High,
}

impl SkyLightQuality {
    pub const ALL: [Self; 2] = [Self::Low, Self::High];

    pub fn resolution(self) -> u32 {
        match self {
            Self::Low => 64,
            Self::High => 128,
        }
    }
}

#[derive(Clone, Copy, PartialEq, Eq, Debug, Serialize, Deserialize)]
pub enum FrameLimit {
    /// As fast as the screen refreshes, without tearing
    VSync,
    Fps30,
    Fps60,
    Fps120,
    Unlimited,
}

impl FrameLimit {
    pub const ALL: [Self; 5] = [
        Self::VSync,
        Self::Fps30,
        Self::Fps60,
        Self::Fps120,
        Self::Unlimited,
    ];

    /// The frame rate cap, if it's a number
    pub fn fps(self) -> Option<f64> {
        match self {
            Self::Fps30 => Some(30.0),
            Self::Fps60 => Some(60.0),
            Self::Fps120 => Some(120.0),
            Self::VSync | Self::Unlimited => None,
        }
    }
}

#[derive(Clone, PartialEq, Debug, Serialize, Deserialize)]
#[serde(default)]
pub struct Controls {
    /// Mouse look speed, 1.0 = default
    pub sensitivity: f32,
    /// Moving the mouse up looks down
    pub invert_y: bool,
    /// What each action is bound to. Actions missing here (added in a newer
    /// version) use their default
    pub bindings: BTreeMap<Action, Binding>,
}

impl Default for Controls {
    fn default() -> Self {
        Self {
            sensitivity: 1.0,
            invert_y: false,
            bindings: Action::ALL
                .into_iter()
                .map(|action| (action, action.default_binding()))
                .collect(),
        }
    }
}

impl Controls {
    /// What `action` is bound to
    pub fn binding(&self, action: Action) -> Binding {
        self.bindings
            .get(&action)
            .copied()
            .unwrap_or_else(|| action.default_binding())
    }
}

/// A key or mouse button
#[derive(Clone, Copy, PartialEq, Eq, Debug, Serialize, Deserialize)]
pub enum Binding {
    Key(KeyCode),
    Mouse(MouseButton),
}

/// Everything the player can do with a key or button. Esc (pause) is fixed
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Debug, Serialize, Deserialize)]
pub enum Action {
    Forward,
    Back,
    Left,
    Right,
    Jump,
    Sprint,
    Crouch,
    Break,
    Place,
    Interact,
    Inventory,
    Slot1,
    Slot2,
    Slot3,
    Slot4,
    Slot5,
    Debug,
}

impl Action {
    /// In the order the controls screen lists them
    pub const ALL: [Action; 17] = [
        Self::Forward,
        Self::Back,
        Self::Left,
        Self::Right,
        Self::Jump,
        Self::Sprint,
        Self::Crouch,
        Self::Break,
        Self::Place,
        Self::Interact,
        Self::Inventory,
        Self::Slot1,
        Self::Slot2,
        Self::Slot3,
        Self::Slot4,
        Self::Slot5,
        Self::Debug,
    ];

    /// The hotbar slot keys, in slot order
    pub const SLOTS: [Action; 5] = [
        Self::Slot1,
        Self::Slot2,
        Self::Slot3,
        Self::Slot4,
        Self::Slot5,
    ];

    pub fn label(self) -> &'static str {
        match self {
            Self::Forward => "Move forward",
            Self::Back => "Move back",
            Self::Left => "Move left",
            Self::Right => "Move right",
            Self::Jump => "Jump / fly up (double-tap: fly)",
            Self::Sprint => "Sprint / keep placing",
            Self::Crouch => "Crouch / fly down",
            Self::Break => "Break",
            Self::Place => "Place",
            Self::Interact => "Interact",
            Self::Inventory => "Inventory (hold)",
            Self::Slot1 => "Hotbar slot 1",
            Self::Slot2 => "Hotbar slot 2",
            Self::Slot3 => "Hotbar slot 3",
            Self::Slot4 => "Hotbar slot 4",
            Self::Slot5 => "Hotbar slot 5",
            Self::Debug => "Debug info",
        }
    }

    // Not Ctrl for anything: Ctrl+W closes the browser tab on the web
    pub fn default_binding(self) -> Binding {
        use Binding::{Key, Mouse};
        match self {
            Self::Forward => Key(KeyCode::KeyW),
            Self::Back => Key(KeyCode::KeyS),
            Self::Left => Key(KeyCode::KeyA),
            Self::Right => Key(KeyCode::KeyD),
            Self::Jump => Key(KeyCode::Space),
            Self::Sprint => Key(KeyCode::ShiftLeft),
            Self::Crouch => Key(KeyCode::KeyC),
            Self::Break => Mouse(MouseButton::Left),
            Self::Place => Mouse(MouseButton::Right),
            Self::Interact => Key(KeyCode::KeyE),
            Self::Inventory => Key(KeyCode::Tab),
            Self::Slot1 => Key(KeyCode::Digit1),
            Self::Slot2 => Key(KeyCode::Digit2),
            Self::Slot3 => Key(KeyCode::Digit3),
            Self::Slot4 => Key(KeyCode::Digit4),
            Self::Slot5 => Key(KeyCode::Digit5),
            Self::Debug => Key(KeyCode::F3),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn settings_round_trip_through_ron() {
        let mut settings = Settings::default();
        settings.graphics.fov = 85.0;
        settings
            .controls
            .bindings
            .insert(Action::Jump, Binding::Mouse(MouseButton::Middle));
        let text = ron::to_string(&settings).unwrap();
        assert_eq!(ron::from_str::<Settings>(&text).unwrap(), settings);
    }

    #[test]
    fn missing_fields_take_defaults() {
        let settings: Settings = ron::from_str("(graphics: (fov: 100.0))").unwrap();
        assert_eq!(settings.graphics.fov, 100.0);
        assert_eq!(settings.graphics.render_distance, 8);
        assert_eq!(
            settings.controls.binding(Action::Jump),
            Action::Jump.default_binding()
        );
    }
}
