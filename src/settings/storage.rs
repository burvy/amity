//! Keeping settings between sessions: a `settings.ron` file natively, the
//! browser's local storage on the web. A missing or unreadable save just means
//! the defaults

use bevy::prelude::*;

use super::definition::Settings;

pub fn load() -> Settings {
    let Some(text) = read() else {
        return Settings::default();
    };
    ron::from_str(&text).unwrap_or_else(|error| {
        warn!("couldn't read the saved settings, using the defaults: {error}");
        Settings::default()
    })
}

pub fn save(settings: &Settings) {
    match ron::ser::to_string_pretty(settings, ron::ser::PrettyConfig::default()) {
        Ok(text) => write(&text),
        Err(error) => warn!("couldn't save the settings: {error}"),
    }
}

#[cfg(not(target_family = "wasm"))]
mod native {
    use std::{env, fs, path::PathBuf};

    use bevy::prelude::*;

    /// The OS's folder for app settings, then `Amity/settings.ron` inside it:
    /// %APPDATA% on Windows, ~/Library/Application Support on macOS,
    /// $XDG_CONFIG_HOME or ~/.config on Linux
    fn path() -> Option<PathBuf> {
        let base = if cfg!(target_os = "windows") {
            env::var_os("APPDATA").map(PathBuf::from)
        } else if cfg!(target_os = "macos") {
            env::var_os("HOME").map(|home| PathBuf::from(home).join("Library/Application Support"))
        } else {
            env::var_os("XDG_CONFIG_HOME")
                .map(PathBuf::from)
                .or_else(|| env::var_os("HOME").map(|home| PathBuf::from(home).join(".config")))
        }?;
        Some(base.join("Amity").join("settings.ron"))
    }

    pub fn read() -> Option<String> {
        fs::read_to_string(path()?).ok()
    }

    pub fn write(text: &str) {
        let Some(path) = path() else {
            warn!("no settings folder found, settings won't be kept");
            return;
        };
        let result = path
            .parent()
            .map_or(Ok(()), fs::create_dir_all)
            .and_then(|()| fs::write(&path, text));
        if let Err(error) = result {
            warn!("couldn't save the settings to {}: {error}", path.display());
        }
    }
}

#[cfg(target_family = "wasm")]
mod web {
    /// The key the settings are kept under in the browser's local storage
    const KEY: &str = "amity.settings";

    fn storage() -> Option<web_sys::Storage> {
        web_sys::window()?.local_storage().ok()?
    }

    pub fn read() -> Option<String> {
        storage()?.get_item(KEY).ok()?
    }

    pub fn write(text: &str) {
        if let Some(storage) = storage() {
            let _ = storage.set_item(KEY, text);
        }
    }
}

#[cfg(not(target_family = "wasm"))]
use native::{read, write};
#[cfg(target_family = "wasm")]
use web::{read, write};

/// Settings changed recently: saved once they've been still for a moment, so
/// dragging a slider doesn't write the file every frame
#[derive(Resource, Default)]
pub struct SaveTimer(Option<f32>);

/// Seconds after the last change before saving
const SAVE_DELAY_SECS: f32 = 0.5;

pub fn save_when_changed(settings: Res<Settings>, time: Res<Time>, mut timer: ResMut<SaveTimer>) {
    let now = time.elapsed_secs();
    // `is_added`: loading them at startup isn't a change worth saving
    if settings.is_changed() && !settings.is_added() {
        timer.0 = Some(now + SAVE_DELAY_SECS);
    }
    if timer.0.is_some_and(|at| now >= at) {
        timer.0 = None;
        save(&settings);
    }
}
