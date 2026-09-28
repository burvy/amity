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

use super::{config::*, font::UiFont, help, settings};
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
            settings::spawn_settings(panel, &font, (PageOf(PausePage::Settings), Visibility::Hidden));
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
/// list back to the game. (Not while a key binding is waiting for a key: there
/// Esc cancels that instead)
pub fn escape(
    keys: Res<ButtonInput<KeyCode>>,
    rebinding: Res<settings::Rebinding>,
    mut info: ResMut<PlayerInfo>,
    mut page: ResMut<PausePage>,
    mut cursor: Single<&mut CursorOptions, With<PrimaryWindow>>,
) {
    if !keys.just_pressed(KeyCode::Escape) || rebinding.0.is_some() {
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

/// On the web, the browser keeps Esc for itself while it has the mouse locked:
/// it releases the mouse, and the game may never see the key. So when the game
/// thinks it's playing but the browser no longer has the mouse locked, pause as
/// if Esc was pressed. (Right after resuming, the browser takes a moment to lock
/// it; that's allowed for.)
#[cfg(target_family = "wasm")]
pub fn pause_when_mouse_released(
    mut info: ResMut<PlayerInfo>,
    mut page: ResMut<PausePage>,
    mut cursor: Single<&mut CursorOptions, With<PrimaryWindow>>,
    time: Res<Time<Real>>,
    mut playing_since: Local<Option<f32>>,
) {
    /// seconds the browser gets to lock the mouse after resuming
    const LOCK_GRACE_SECS: f32 = 0.5;
    if info.paused || info.menu_open {
        *playing_since = None;
        return;
    }
    let now = time.elapsed_secs();
    let since = *playing_since.get_or_insert(now);
    let locked = web_sys::window()
        .and_then(|window| window.document())
        .is_some_and(|document| document.pointer_lock_element().is_some());
    if !locked && now - since > LOCK_GRACE_SECS {
        pause(&mut info, &mut cursor, &mut page);
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

/// Shows the menu while paused, and only the open page in its panel (hidden
/// pages are taken out of the layout too, so the open one gets all the room)
pub fn show_menu(
    info: Res<PlayerInfo>,
    page: Res<PausePage>,
    mut menu: Single<&mut Visibility, (With<PauseMenu>, Without<PageOf>)>,
    mut pages: Query<(&PageOf, &mut Visibility, &mut Node), Without<PauseMenu>>,
) {
    let shown = if info.paused {
        Visibility::Inherited
    } else {
        Visibility::Hidden
    };
    if **menu != shown {
        **menu = shown;
    }
    for (PageOf(of), mut visibility, mut node) in &mut pages {
        let (wanted, display) = if *of == *page {
            (Visibility::Inherited, Display::Flex)
        } else {
            (Visibility::Hidden, Display::None)
        };
        if *visibility != wanted {
            *visibility = wanted;
        }
        if node.display != display {
            node.display = display;
        }
    }
}
