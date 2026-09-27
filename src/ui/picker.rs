//! The creative block picker: Tab opens a grid of every item; clicking one puts
//! it in the selected hotbar slot. The server does the actual change (and only
//! in creative), so this just sends `SetSlot`

use amity_protocol::inventory::{
    definition::{ActionChannel, GameMode, SetSlot},
    item::Item,
};
use bevy::{
    prelude::*,
    window::{CursorGrabMode, CursorOptions, PrimaryWindow},
};
use lightyear::prelude::{Controlled, MessageSender};

use super::config::*;
use crate::{
    player::{config::KEY_PICKER, definition::PlayerInfo},
    world::meshing::{material::BlockAtlas, textures::item_tile},
};

/// The picker's whole screen, while it's open
#[derive(Component)]
pub struct Picker;

/// A button in the picker, for this item
#[derive(Component)]
pub struct PickerItem(Item);

/// Tab opens the picker (creative only) and closes it; Escape closes it too.
/// While it's open the game is paused, so the cursor is free to click with
pub fn toggle_picker(
    mut cmds: Commands,
    keys: Res<ButtonInput<KeyCode>>,
    mut info: ResMut<PlayerInfo>,
    mut cursor: Single<&mut CursorOptions, With<PrimaryWindow>>,
    mode: Option<Single<&GameMode, With<Controlled>>>,
    open: Option<Single<Entity, With<Picker>>>,
    atlas: Option<Res<BlockAtlas>>,
) {
    match open {
        Some(picker) => {
            let tab = keys.just_pressed(KEY_PICKER);
            if !tab && !keys.just_pressed(KeyCode::Escape) {
                return;
            }
            cmds.entity(*picker).despawn();
            info.menu_open = false;
            // back to playing on Tab; Escape leaves the game paused
            if tab {
                info.paused = false;
                cursor.grab_mode = CursorGrabMode::Locked;
                cursor.visible = false;
            }
        }
        None => {
            let creative = mode.is_some_and(|mode| **mode == GameMode::Creative);
            let (true, Some(atlas)) = (keys.just_pressed(KEY_PICKER) && creative, atlas) else {
                return;
            };
            spawn_picker(&mut cmds, &atlas);
            info.menu_open = true;
            info.paused = true;
            cursor.grab_mode = CursorGrabMode::None;
            cursor.visible = true;
        }
    }
}

fn spawn_picker(cmds: &mut Commands, atlas: &BlockAtlas) {
    cmds.spawn((
        Picker,
        // the whole screen, dimmed, with the grid in the middle
        Node {
            position_type: PositionType::Absolute,
            width: Val::Percent(100.0),
            height: Val::Percent(100.0),
            align_items: AlignItems::Center,
            justify_content: JustifyContent::Center,
            ..default()
        },
        BackgroundColor(PICKER_DIM),
    ))
    .with_children(|screen| {
        screen
            .spawn(Node {
                display: Display::Grid,
                grid_template_columns: RepeatedGridTrack::px(PICKER_COLUMNS, PICKER_SLOT_SIZE),
                row_gap: Val::Px(PICKER_GAP),
                column_gap: Val::Px(PICKER_GAP),
                ..default()
            })
            .with_children(|grid| {
                for item in Item::all() {
                    grid.spawn((
                        PickerItem(item),
                        Button,
                        Node {
                            width: Val::Px(PICKER_SLOT_SIZE),
                            height: Val::Px(PICKER_SLOT_SIZE),
                            align_items: AlignItems::Center,
                            justify_content: JustifyContent::Center,
                            ..default()
                        },
                        BackgroundColor(PICKER_SLOT),
                    ))
                    .with_children(|button| {
                        button.spawn((
                            Node {
                                width: Val::Px(PICKER_ICON_SIZE),
                                height: Val::Px(PICKER_ICON_SIZE),
                                ..default()
                            },
                            ImageNode::from_atlas_image(
                                atlas.image.clone(),
                                TextureAtlas {
                                    layout: atlas.layout.clone(),
                                    index: item_tile(item) as usize,
                                },
                            ),
                        ));
                    });
                }
            });
    });
}

/// Clicking an item asks the server to put it in the selected slot; hovering
/// lights the button up
pub fn click_picker(
    mut buttons: Query<(&Interaction, &PickerItem, &mut BackgroundColor), Changed<Interaction>>,
    info: Res<PlayerInfo>,
    mut sender: Single<&mut MessageSender<SetSlot>>,
) {
    for (interaction, PickerItem(item), mut background) in &mut buttons {
        match interaction {
            Interaction::Pressed => {
                sender.send::<ActionChannel>(SetSlot {
                    slot: info.slot as usize,
                    item: Some(*item),
                });
            }
            Interaction::Hovered => background.0 = PICKER_SLOT_HOVERED,
            Interaction::None => background.0 = PICKER_SLOT,
        }
    }
}
