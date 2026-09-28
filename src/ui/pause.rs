//! The pause menu (Esc): a list of choices down the left over the dimmed game,
//! with Help and Settings opening in a panel beside it.
//!
//! The game keeps running while paused (it's multiplayer): only our inputs stop,
//! and the mouse is freed. The game also starts paused, with this menu showing:
//! on the web, locking the mouse needs a click anyway.

use bevy::{
    app::AppExit,
    prelude::*,
    window::{CursorGrabMode, CursorOptions, PrimaryWindow},
};

use super::{config::*, font::UiFont, help};
use crate::player::definition::PlayerInfo;

/// Which page the menu's panel shows
#[derive(Resource, Default, Clone, Copy, PartialEq, Eq, Debug)]
pub enum PausePage {
    /// just the list
    #[default]
    Main,
    Help,
    Settings,
}

/// The whole menu
#[derive(Component)]
pub struct PauseMenu;

/// A choice in the list
#[derive(Component, Clone, Copy, PartialEq, Eq, Debug)]
pub enum MenuButton {
    Resume,
    Help,
    Settings,
    Leave,
}

impl MenuButton {
    const ALL: [MenuButton; 4] = [Self::Resume, Self::Help, Self::Settings, Self::Leave];

    fn label(self) -> &'static str {
        match self {
            Self::Resume => "Resume",
            Self::Help => "Help",
            Self::Settings => "Settings",
            Self::Leave => "Leave",
        }
    }
}

/// A page shown in the panel
#[derive(Component)]
pub struct PageOf(PausePage);

pub fn spawn_pause_menu(mut cmds: Commands, font: Res<UiFont>) {
    let text = |size: f32, colour: Color| {
        (
            TextFont {
                font: font.0.clone().into(),
                font_size: FontSize::Px(size),
                ..default()
            },
            TextColor(colour),
        )
    };
    cmds.spawn((
        PauseMenu,
        Node {
            position_type: PositionType::Absolute,
            width: Val::Percent(100.0),
            height: Val::Percent(100.0),
            padding: UiRect::all(Val::Px(MENU_MARGIN)),
            column_gap: Val::Px(MENU_MARGIN),
            ..default()
        },
        BackgroundColor(MENU_DIM),
        // over the hotbar and everything else
        GlobalZIndex(20),
        Visibility::Hidden,
    ))
    .with_children(|menu| {
        // the list down the left
        menu.spawn(Node {
            flex_direction: FlexDirection::Column,
            justify_content: JustifyContent::Center,
            row_gap: Val::Px(MENU_BUTTON_GAP),
            width: Val::Px(MENU_LIST_WIDTH),
            flex_shrink: 0.0,
            ..default()
        })
        .with_children(|list| {
            list.spawn((
                Text::new("AMITY"),
                text(MENU_TITLE_SIZE, MENU_HEADING),
                Node {
                    margin: UiRect::bottom(Val::Px(MENU_BUTTON_GAP * 2.0)),
                    ..default()
                },
            ));
            for button in MenuButton::ALL {
                list.spawn((button, Button, Text::new(button.label()), text(MENU_BUTTON_SIZE, MENU_TEXT_DIM)));
            }
        });

        // the panel on the right, for Help and Settings
        menu.spawn(Node {
            flex_grow: 1.0,
            max_width: Val::Px(MENU_PANEL_MAX_WIDTH),
            height: Val::Percent(100.0),
            ..default()
        })
        .with_children(|panel| {
            help::spawn_help(panel, &font, (PageOf(PausePage::Help), Visibility::Hidden));
            panel.spawn((
                PageOf(PausePage::Settings),
                Text::new("Settings are coming next."),
                text(MENU_BUTTON_SIZE / 2.0, MENU_TEXT),
                Visibility::Hidden,
            ));
        });
    });
}

/// Frees the mouse and shows the menu
fn pause(info: &mut PlayerInfo, cursor: &mut CursorOptions, page: &mut PausePage) {
    info.paused = true;
    *page = PausePage::Main;
    cursor.grab_mode = CursorGrabMode::None;
    cursor.visible = true;
}

/// Locks the mouse again and hides the menu
fn resume(info: &mut PlayerInfo, cursor: &mut CursorOptions) {
    info.paused = false;
    cursor.grab_mode = CursorGrabMode::Locked;
    cursor.visible = false;
}

/// Esc: pauses; while paused, goes back from a page to the list, and from the
/// list back to the game
pub fn escape(
    keys: Res<ButtonInput<KeyCode>>,
    mut info: ResMut<PlayerInfo>,
    mut page: ResMut<PausePage>,
    mut cursor: Single<&mut CursorOptions, With<PrimaryWindow>>,
) {
    if !keys.just_pressed(KeyCode::Escape) {
        return;
    }
    if !info.paused {
        pause(&mut info, &mut cursor, &mut page);
    } else if *page != PausePage::Main {
        *page = PausePage::Main;
    } else {
        resume(&mut info, &mut cursor);
    }
}

/// Clicking a choice; hovering one marks it with `>` and brightens it
pub fn click_menu(
    mut buttons: Query<(&MenuButton, &Interaction, &mut Text, &mut TextColor)>,
    mut info: ResMut<PlayerInfo>,
    mut page: ResMut<PausePage>,
    mut cursor: Single<&mut CursorOptions, With<PrimaryWindow>>,
    mut exit: MessageWriter<AppExit>,
) {
    if !info.paused {
        return;
    }
    for (button, interaction, mut text, mut colour) in &mut buttons {
        // the open page's button stays marked too
        let open = match button {
            MenuButton::Help => *page == PausePage::Help,
            MenuButton::Settings => *page == PausePage::Settings,
            _ => false,
        };
        let marked = open || *interaction != Interaction::None;
        let label = if marked {
            format!("> {}", button.label())
        } else {
            button.label().to_string()
        };
        if text.0 != label {
            text.0 = label;
        }
        let wanted = if marked { MENU_TEXT } else { MENU_TEXT_DIM };
        if colour.0 != wanted {
            colour.0 = wanted;
        }

        if *interaction == Interaction::Pressed {
            match button {
                MenuButton::Resume => resume(&mut info, &mut cursor),
                MenuButton::Help => *page = PausePage::Help,
                MenuButton::Settings => *page = PausePage::Settings,
                MenuButton::Leave => leave(&mut exit),
            }
        }
    }
}

/// Leaving: back to the site's front page on the web (which also ends the
/// connection), quitting natively
fn leave(exit: &mut MessageWriter<AppExit>) {
    #[cfg(target_family = "wasm")]
    {
        let _ = exit;
        if let Some(window) = web_sys::window() {
            let _ = window.location().assign("/");
        }
    }
    #[cfg(not(target_family = "wasm"))]
    exit.write(AppExit::Success);
}

/// Shows the menu while paused, and only the open page in its panel
pub fn show_menu(
    info: Res<PlayerInfo>,
    page: Res<PausePage>,
    mut menu: Single<&mut Visibility, (With<PauseMenu>, Without<PageOf>)>,
    mut pages: Query<(&PageOf, &mut Visibility), Without<PauseMenu>>,
) {
    let shown = if info.paused {
        Visibility::Inherited
    } else {
        Visibility::Hidden
    };
    if **menu != shown {
        **menu = shown;
    }
    for (PageOf(of), mut visibility) in &mut pages {
        let wanted = if *of == *page {
            Visibility::Inherited
        } else {
            Visibility::Hidden
        };
        if *visibility != wanted {
            *visibility = wanted;
        }
    }
}
