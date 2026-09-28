//! The Settings page of the pause menu: Graphics and Controls tabs, a row per
//! setting. Changes take effect (and are saved) straight away.
//!
//! - sliders: click or drag along the bar
//! - `<` `>` choosers and On/Off toggles: click to change
//! - key bindings: click one, then press a key or mouse button (Esc cancels).
//!   A key bound to two actions shows in red
//! - "Reset to defaults" resets the open tab only

use bevy::{prelude::*, ui::RelativeCursorPosition};

use super::{config::*, font::UiFont, pause::PausePage, scroll::spawn_scrolling};
use crate::{
    player::definition::PlayerInfo,
    settings::definition::{
        Action, Binding, Controls, FrameLimit, Graphics, Settings, ShadowQuality, SkyLightQuality,
    },
};

/// Which tab is open
#[derive(Resource, Default, Clone, Copy, PartialEq, Eq, Debug)]
pub enum SettingsTab {
    #[default]
    Graphics,
    Controls,
}

impl SettingsTab {
    const ALL: [Self; 2] = [Self::Graphics, Self::Controls];

    fn label(self) -> &'static str {
        match self {
            Self::Graphics => "Graphics",
            Self::Controls => "Controls",
        }
    }
}

/// The action waiting for a key to be pressed, after its binding was clicked
#[derive(Resource, Default)]
pub struct Rebinding(pub Option<Action>);

/// A setting with a row of its own (key bindings are separate: see
/// `BindButton`)
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
enum Setting {
    RenderDistance,
    Fov,
    Shadows,
    SkyLight,
    Bloom,
    FrameLimit,
    UiScale,
    Sensitivity,
    InvertY,
}

/// How a setting is changed
enum Kind {
    /// dragged between `min` and `max`, in steps of `step`
    Slider { min: f32, max: f32, step: f32 },
    /// stepped through a list with `<` `>`
    Chooser,
    /// On or Off
    Toggle,
}

impl Setting {
    const GRAPHICS: [Self; 7] = [
        Self::RenderDistance,
        Self::Fov,
        Self::Shadows,
        Self::SkyLight,
        Self::Bloom,
        Self::FrameLimit,
        Self::UiScale,
    ];
    const MOUSE: [Self; 2] = [Self::Sensitivity, Self::InvertY];

    fn label(self) -> &'static str {
        match self {
            Self::RenderDistance => "Render distance",
            Self::Fov => "Field of view",
            Self::Shadows => "Shadows",
            Self::SkyLight => "Sky lighting",
            Self::Bloom => "Bloom",
            Self::FrameLimit => "Frame rate",
            Self::UiScale => "UI size",
            Self::Sensitivity => "Mouse sensitivity",
            Self::InvertY => "Invert mouse Y",
        }
    }

    fn kind(self) -> Kind {
        let slider = |(min, max), step| Kind::Slider { min, max, step };
        match self {
            Self::RenderDistance => slider(RENDER_DISTANCE_RANGE, 1.0),
            Self::Fov => slider(FOV_RANGE, 1.0),
            Self::Sensitivity => slider(SENSITIVITY_RANGE, 0.05),
            Self::Shadows | Self::SkyLight | Self::FrameLimit | Self::UiScale => Kind::Chooser,
            Self::Bloom | Self::InvertY => Kind::Toggle,
        }
    }

    /// A slider's value
    fn value(self, settings: &Settings) -> f32 {
        match self {
            Self::RenderDistance => settings.graphics.render_distance as f32,
            Self::Fov => settings.graphics.fov,
            Self::Sensitivity => settings.controls.sensitivity,
            _ => 0.0,
        }
    }

    /// Sets a slider's value
    fn set(self, settings: &mut Settings, value: f32) {
        match self {
            Self::RenderDistance => settings.graphics.render_distance = value as u8,
            Self::Fov => settings.graphics.fov = value,
            Self::Sensitivity => settings.controls.sensitivity = value,
            _ => {}
        }
    }

    /// Steps a chooser forward (`by` 1) or back (-1), or flips a toggle
    fn cycle(self, settings: &mut Settings, by: i32) {
        let graphics = &mut settings.graphics;
        match self {
            Self::Shadows => graphics.shadows = step(&ShadowQuality::ALL, graphics.shadows, by),
            Self::SkyLight => {
                graphics.sky_light = step(&SkyLightQuality::ALL, graphics.sky_light, by)
            }
            Self::FrameLimit => {
                graphics.frame_limit = step(&FrameLimit::ALL, graphics.frame_limit, by)
            }
            Self::UiScale => graphics.ui_scale = step(&UI_SCALES, graphics.ui_scale, by),
            Self::Bloom => graphics.bloom = !graphics.bloom,
            Self::InvertY => settings.controls.invert_y = !settings.controls.invert_y,
            _ => {}
        }
    }

    /// The value as shown
    fn text(self, settings: &Settings) -> String {
        let (graphics, controls) = (&settings.graphics, &settings.controls);
        let on_off = |on: bool| if on { "On" } else { "Off" }.to_string();
        match self {
            Self::RenderDistance => format!(
                "{} chunks ({} m)",
                graphics.render_distance,
                graphics.render_distance as u32 * 16
            ),
            Self::Fov => format!("{:.0} degrees", graphics.fov),
            Self::Shadows => format!("{:?}", graphics.shadows),
            Self::SkyLight => format!("{:?}", graphics.sky_light),
            Self::Bloom => on_off(graphics.bloom),
            Self::FrameLimit => match graphics.frame_limit.fps() {
                Some(fps) => format!("{fps:.0} fps"),
                None => format!("{:?}", graphics.frame_limit),
            },
            Self::UiScale => format!("{:.0}%", graphics.ui_scale * 100.0),
            Self::Sensitivity => format!("{:.2}x", controls.sensitivity),
            Self::InvertY => on_off(controls.invert_y),
        }
    }
}

/// The entry `by` places along from `current` in `all`, wrapping around. A
/// value not in the list (a hand-edited settings file) starts from the first
fn step<T: PartialEq + Copy>(all: &[T], current: T, by: i32) -> T {
    let at = all.iter().position(|&x| x == current).unwrap_or(0) as i32;
    all[(at + by).rem_euclid(all.len() as i32) as usize]
}

/// A key or button's name, as shown
fn binding_name(binding: Binding) -> String {
    match binding {
        Binding::Mouse(button) => match button {
            MouseButton::Left => "Left click".into(),
            MouseButton::Right => "Right click".into(),
            MouseButton::Middle => "Middle click".into(),
            MouseButton::Back => "Mouse back".into(),
            MouseButton::Forward => "Mouse forward".into(),
            MouseButton::Other(n) => format!("Mouse {n}"),
        },
        Binding::Key(key) => {
            // from the key's code name: KeyW -> W, Digit1 -> 1, ShiftLeft ->
            // Left Shift, ArrowUp -> Up arrow
            let name = format!("{key:?}");
            if let Some(rest) = name.strip_prefix("Key").or(name.strip_prefix("Digit")) {
                rest.into()
            } else if let Some(rest) = name.strip_prefix("Arrow") {
                format!("{rest} arrow")
            } else if let Some(rest) = name.strip_suffix("Left") {
                format!("Left {rest}")
            } else if let Some(rest) = name.strip_suffix("Right") {
                format!("Right {rest}")
            } else {
                name
            }
        }
    }
}

/// A tab's button
#[derive(Component)]
pub struct TabButton(SettingsTab);

/// A tab's contents
#[derive(Component)]
pub struct TabContent(SettingsTab);

/// A setting's value text
#[derive(Component)]
pub struct ValueText(Setting);

/// Clicking steps this setting by this much (a `<` or `>`, or a toggle's text)
#[derive(Component)]
pub struct CycleButton(Setting, i32);

/// A slider's clickable area
#[derive(Component)]
pub struct Slider(Setting);

/// The filled part of a slider's bar
#[derive(Component)]
pub struct SliderFill(Setting);

/// An action's binding: click to change it
#[derive(Component)]
pub struct BindButton(Action);

/// Resets the open tab
#[derive(Component)]
pub struct ResetButton;

/// Text that lights up while hovered, and is this colour otherwise
#[derive(Component)]
pub struct Hoverable(Color);

fn text(font: &UiFont, size: f32, colour: Color) -> impl Bundle {
    (
        TextFont {
            font: font.0.clone().into(),
            font_size: FontSize::Px(size),
            ..default()
        },
        TextColor(colour),
    )
}

/// Builds the settings page into `parent` (the pause menu's panel), with
/// `extra` components on its root
pub fn spawn_settings(parent: &mut ChildSpawnerCommands, font: &UiFont, extra: impl Bundle) {
    parent
        .spawn((
            Node {
                flex_direction: FlexDirection::Column,
                width: Val::Percent(100.0),
                height: Val::Percent(100.0),
                row_gap: Val::Px(SETTINGS_TAB_GAP / 2.0),
                ..default()
            },
            extra,
        ))
        .with_children(|page| {
            // the tabs along the top
            page.spawn(Node {
                column_gap: Val::Px(SETTINGS_TAB_GAP),
                flex_shrink: 0.0,
                ..default()
            })
            .with_children(|tabs| {
                for tab in SettingsTab::ALL {
                    tabs.spawn((
                        TabButton(tab),
                        Button,
                        Text::new(tab.label()),
                        text(font, SETTINGS_TAB_SIZE, MENU_TEXT_DIM),
                    ));
                }
            });

            // the open tab's rows, below
            page.spawn(Node {
                flex_grow: 1.0,
                min_height: Val::Px(0.0),
                ..default()
            })
            .with_children(|content| {
                spawn_scrolling(
                    content,
                    TabContent(SettingsTab::Graphics),
                    SETTINGS_ROW_GAP,
                    |rows| {
                        for setting in Setting::GRAPHICS {
                            spawn_setting(rows, font, setting);
                        }
                        spawn_footer(rows, font);
                    },
                );
                spawn_scrolling(
                    content,
                    TabContent(SettingsTab::Controls),
                    SETTINGS_ROW_GAP,
                    |rows| {
                        rows.spawn((
                            Text::new("Mouse"),
                            text(font, SETTINGS_TEXT_SIZE, MENU_HEADING),
                        ));
                        for setting in Setting::MOUSE {
                            spawn_setting(rows, font, setting);
                        }
                        rows.spawn((
                            Text::new("Keys"),
                            text(font, SETTINGS_TEXT_SIZE, MENU_HEADING),
                            Node {
                                margin: UiRect::top(Val::Px(SETTINGS_ROW_GAP)),
                                ..default()
                            },
                        ));
                        for action in Action::ALL {
                            spawn_row(rows, font, action.label(), |value| {
                                value.spawn((
                                    BindButton(action),
                                    Button,
                                    Text::default(),
                                    text(font, SETTINGS_TEXT_SIZE, MENU_TEXT),
                                ));
                            });
                        }
                        spawn_footer(rows, font);
                    },
                );
            });
        });
}

/// A row: the name on the left, and whatever `value` builds on the right
fn spawn_row(
    rows: &mut ChildSpawnerCommands,
    font: &UiFont,
    label: &str,
    value: impl FnOnce(&mut ChildSpawnerCommands),
) {
    rows.spawn(Node {
        align_items: AlignItems::Center,
        flex_shrink: 0.0,
        ..default()
    })
    .with_children(|row| {
        row.spawn((
            Text::new(label),
            text(font, SETTINGS_TEXT_SIZE, MENU_TEXT),
            Node {
                width: Val::Px(SETTINGS_LABEL_WIDTH),
                flex_shrink: 0.0,
                ..default()
            },
        ));
        row.spawn(Node {
            align_items: AlignItems::Center,
            column_gap: Val::Px(SETTINGS_ROW_GAP),
            ..default()
        })
        .with_children(value);
    });
}

/// A setting's row, built to suit its kind
fn spawn_setting(rows: &mut ChildSpawnerCommands, font: &UiFont, setting: Setting) {
    let value_text = |value: &mut ChildSpawnerCommands, extra: Option<CycleButton>| {
        // a fixed-width box, so the arrows stay put as the value changes
        value
            .spawn(Node {
                width: Val::Px(SETTINGS_VALUE_WIDTH),
                justify_content: JustifyContent::Center,
                ..default()
            })
            .with_children(|centre| {
                let mut text_entity = centre.spawn((
                    ValueText(setting),
                    Text::default(),
                    text(font, SETTINGS_TEXT_SIZE, MENU_TEXT),
                ));
                if let Some(cycle) = extra {
                    text_entity.insert((cycle, Button, Hoverable(MENU_TEXT)));
                }
            });
    };
    let arrow = |value: &mut ChildSpawnerCommands, label: &str, by: i32| {
        value.spawn((
            CycleButton(setting, by),
            Button,
            Hoverable(MENU_TEXT_DIM),
            Text::new(label),
            text(font, SETTINGS_TEXT_SIZE, MENU_TEXT_DIM),
        ));
    };

    spawn_row(rows, font, setting.label(), |value| match setting.kind() {
        Kind::Slider { .. } => {
            // the clickable area, around a thin bar and its filled part
            value
                .spawn((
                    Slider(setting),
                    Button,
                    RelativeCursorPosition::default(),
                    Node {
                        width: Val::Px(SETTINGS_SLIDER_WIDTH),
                        height: Val::Px(SETTINGS_SLIDER_HIT_HEIGHT),
                        align_items: AlignItems::Center,
                        ..default()
                    },
                ))
                .with_children(|area| {
                    area.spawn((
                        Node {
                            width: Val::Percent(100.0),
                            height: Val::Px(SETTINGS_SLIDER_THICKNESS),
                            ..default()
                        },
                        BackgroundColor(SETTINGS_SLIDER_TRACK),
                    ))
                    .with_children(|bar| {
                        bar.spawn((
                            SliderFill(setting),
                            Node {
                                height: Val::Percent(100.0),
                                ..default()
                            },
                            BackgroundColor(SETTINGS_SLIDER_FILL),
                        ));
                    });
                });
            value_text(value, None);
        }
        Kind::Chooser => {
            arrow(value, "<", -1);
            value_text(value, None);
            arrow(value, ">", 1);
        }
        // clicking the value itself flips it
        Kind::Toggle => value_text(value, Some(CycleButton(setting, 1))),
    });
}

/// "Reset to defaults", and a note that there's nothing to save
fn spawn_footer(rows: &mut ChildSpawnerCommands, font: &UiFont) {
    rows.spawn((
        ResetButton,
        Button,
        Hoverable(MENU_TEXT_DIM),
        Text::new("Reset to defaults"),
        text(font, SETTINGS_TEXT_SIZE, MENU_TEXT_DIM),
        Node {
            margin: UiRect::top(Val::Px(SETTINGS_ROW_GAP * 2.0)),
            align_self: AlignSelf::FlexStart,
            ..default()
        },
    ));
    rows.spawn((
        Text::new("Changes take effect and are saved straight away."),
        text(font, SETTINGS_TEXT_SIZE, MENU_TEXT_DIM),
    ));
}

/// Whether the settings page is showing
fn showing(info: &PlayerInfo, page: &PausePage) -> bool {
    info.paused && *page == PausePage::Settings
}

/// While a binding waits for a key: the next key or mouse button pressed
/// becomes it, and Esc cancels. The press is then used up, so nothing else
/// reacts to it (clicking another binding doesn't also start changing that one)
pub fn rebind(
    info: Res<PlayerInfo>,
    page: Res<PausePage>,
    tab: Res<SettingsTab>,
    mut rebinding: ResMut<Rebinding>,
    mut keys: ResMut<ButtonInput<KeyCode>>,
    mut mouse: ResMut<ButtonInput<MouseButton>>,
    mut settings: ResMut<Settings>,
) {
    let Some(action) = rebinding.0 else {
        return;
    };
    // left the page: forget it
    if !showing(&info, &page) || *tab != SettingsTab::Controls {
        rebinding.0 = None;
        return;
    }
    let key = keys.get_just_pressed().next().copied();
    let button = mouse.get_just_pressed().next().copied();
    let binding = match (key, button) {
        (Some(KeyCode::Escape), _) => {
            keys.clear_just_pressed(KeyCode::Escape);
            rebinding.0 = None;
            return;
        }
        (Some(key), _) => {
            keys.clear_just_pressed(key);
            Binding::Key(key)
        }
        (None, Some(button)) => {
            mouse.clear_just_pressed(button);
            Binding::Mouse(button)
        }
        (None, None) => return,
    };
    if settings.controls.binding(action) != binding {
        settings.controls.bindings.insert(action, binding);
    }
    rebinding.0 = None;
}

/// Clicks on the page: tabs, choosers, toggles, sliders, bindings and reset
pub fn click_settings(
    info: Res<PlayerInfo>,
    page: Res<PausePage>,
    mouse: Res<ButtonInput<MouseButton>>,
    mut tab: ResMut<SettingsTab>,
    mut rebinding: ResMut<Rebinding>,
    mut settings: ResMut<Settings>,
    tabs: Query<(&TabButton, &Interaction)>,
    cycles: Query<(&CycleButton, &Interaction)>,
    sliders: Query<(&Slider, &Interaction, &RelativeCursorPosition)>,
    binds: Query<(&BindButton, &Interaction)>,
    reset: Query<&Interaction, With<ResetButton>>,
) {
    if !showing(&info, &page) {
        return;
    }
    // changes go to a copy, so that settings only count as changed (and are
    // reapplied and saved) when something really changed
    let mut changed = settings.clone();

    // sliders follow the mouse while held
    for (Slider(setting), interaction, cursor) in &sliders {
        let (Interaction::Pressed, Some(at), Kind::Slider { min, max, step }) =
            (interaction, cursor.normalized, setting.kind())
        else {
            continue;
        };
        // `at` runs from -0.5 (left end) to 0.5 (right end)
        let value = min + (at.x + 0.5).clamp(0.0, 1.0) * (max - min);
        setting.set(
            &mut changed,
            ((value / step).round() * step).clamp(min, max),
        );
    }

    // the rest act once per click
    if mouse.just_pressed(MouseButton::Left) {
        let clicked = |interaction: &Interaction| *interaction == Interaction::Pressed;
        for (TabButton(which), interaction) in &tabs {
            if clicked(interaction) {
                *tab = *which;
            }
        }
        for (CycleButton(setting, by), interaction) in &cycles {
            if clicked(interaction) {
                setting.cycle(&mut changed, *by);
            }
        }
        for (BindButton(action), interaction) in &binds {
            if clicked(interaction) {
                rebinding.0 = Some(*action);
            }
        }
        if reset.iter().any(clicked) {
            match *tab {
                SettingsTab::Graphics => changed.graphics = Graphics::default(),
                SettingsTab::Controls => changed.controls = Controls::default(),
            }
        }
    }

    if changed != *settings {
        *settings = changed;
    }
}

/// Keeps the page showing the current settings: values, sliders, bindings (red
/// where two actions share one), the open tab, and hover highlights
pub fn update_settings(
    info: Res<PlayerInfo>,
    page: Res<PausePage>,
    tab: Res<SettingsTab>,
    rebinding: Res<Rebinding>,
    settings: Res<Settings>,
    mut values: Query<(&ValueText, &mut Text), Without<BindButton>>,
    mut fills: Query<(&SliderFill, &mut Node), Without<TabContent>>,
    mut binds: Query<(&BindButton, &Interaction, &mut Text, &mut TextColor), Without<ValueText>>,
    mut tabs: Query<
        (&TabButton, &Interaction, &mut TextColor),
        (Without<BindButton>, Without<Hoverable>),
    >,
    mut contents: Query<(&TabContent, &mut Node), Without<SliderFill>>,
    mut hoverables: Query<(&Hoverable, &Interaction, &mut TextColor), Without<BindButton>>,
) {
    if !showing(&info, &page) {
        return;
    }
    for (TabContent(of), mut node) in &mut contents {
        let display = if *of == *tab {
            Display::Flex
        } else {
            Display::None
        };
        if node.display != display {
            node.display = display;
        }
    }
    for (TabButton(of), interaction, mut colour) in &mut tabs {
        let wanted = if *of == *tab {
            MENU_HEADING
        } else if *interaction != Interaction::None {
            MENU_TEXT
        } else {
            MENU_TEXT_DIM
        };
        colour.set_if_neq(TextColor(wanted));
    }
    for (Hoverable(idle), interaction, mut colour) in &mut hoverables {
        let wanted = if *interaction != Interaction::None {
            MENU_HEADING
        } else {
            *idle
        };
        colour.set_if_neq(TextColor(wanted));
    }

    for (ValueText(setting), mut text) in &mut values {
        let wanted = setting.text(&settings);
        if text.0 != wanted {
            text.0 = wanted;
        }
    }
    for (SliderFill(setting), mut node) in &mut fills {
        let Kind::Slider { min, max, .. } = setting.kind() else {
            continue;
        };
        let fraction = (setting.value(&settings) - min) / (max - min);
        let width = Val::Percent(fraction.clamp(0.0, 1.0) * 100.0);
        if node.width != width {
            node.width = width;
        }
    }

    let controls = &settings.controls;
    for (BindButton(action), interaction, mut text, mut colour) in &mut binds {
        let binding = controls.binding(*action);
        let waiting = rebinding.0 == Some(*action);
        let wanted = if waiting {
            "Press a key (Esc cancels)".to_string()
        } else {
            binding_name(binding)
        };
        if text.0 != wanted {
            text.0 = wanted;
        }
        let shared = Action::ALL
            .iter()
            .any(|&other| other != *action && controls.binding(other) == binding);
        let wanted = if waiting {
            MENU_HEADING
        } else if shared {
            SETTINGS_CONFLICT
        } else if *interaction != Interaction::None {
            MENU_HEADING
        } else {
            MENU_TEXT
        };
        colour.set_if_neq(TextColor(wanted));
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn choosers_wrap_around() {
        assert_eq!(
            step(&ShadowQuality::ALL, ShadowQuality::High, 1),
            ShadowQuality::Off
        );
        assert_eq!(
            step(&ShadowQuality::ALL, ShadowQuality::Off, -1),
            ShadowQuality::High
        );
        // a hand-edited value not in the list starts from the first
        assert_eq!(step(&UI_SCALES, 1.1, 1), UI_SCALES[1]);
    }

    #[test]
    fn key_names_read_well() {
        assert_eq!(binding_name(Binding::Key(KeyCode::KeyW)), "W");
        assert_eq!(binding_name(Binding::Key(KeyCode::Digit3)), "3");
        assert_eq!(binding_name(Binding::Key(KeyCode::ShiftLeft)), "Left Shift");
        assert_eq!(binding_name(Binding::Key(KeyCode::ArrowUp)), "Up arrow");
        assert_eq!(binding_name(Binding::Key(KeyCode::Space)), "Space");
        assert_eq!(
            binding_name(Binding::Mouse(MouseButton::Right)),
            "Right click"
        );
    }
}
