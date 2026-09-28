//! The creative block picker: a grid of every item, shown left of the inventory
//! while it's open in creative. Clicking an item puts a full stack of it on the
//! cursor, to place in a slot; clicking the picker while holding something
//! throws it away. The server does the actual change (creative only)

use amity_protocol::inventory::{
    config::HOTBAR_SLOTS,
    definition::{ActionChannel, GameMode, Inventory, InventoryAction},
    item::Item,
};
use bevy::prelude::*;
use lightyear::prelude::{Controlled, MessageSender};

use super::config::*;
use crate::{
    player::{config::BUTTON_PRIMARY, definition::PlayerInfo},
    world::meshing::{material::BlockAtlas, textures::item_tile},
};

/// The picker's panel
#[derive(Component)]
pub struct Picker;

/// A button in the picker, for this item
#[derive(Component)]
pub struct PickerItem(Item);

/// Builds the (hidden) picker once the block atlas (for icons) exists
pub fn spawn_picker(mut cmds: Commands, atlas: Res<BlockAtlas>) {
    // left of the hotbar and the 4 inventory columns
    let inventory_width = (HOTBAR_SLOTS as f32) * (HOTBAR_SLOT_SIZE + HOTBAR_GAP);
    cmds.spawn((
        Picker,
        // Interaction: clicking the picker's background throws away what's held
        Button,
        Node {
            position_type: PositionType::Absolute,
            right: Val::Px(HOTBAR_MARGIN + inventory_width + PICKER_MARGIN),
            top: Val::Percent(50.0),
            // centred on the middle of the screen, top to bottom
            margin: UiRect::top(Val::Px(-(PICKER_SLOT_SIZE + PICKER_GAP) * 2.5)),
            display: Display::Grid,
            grid_template_columns: RepeatedGridTrack::px(PICKER_COLUMNS, PICKER_SLOT_SIZE),
            row_gap: Val::Px(PICKER_GAP),
            column_gap: Val::Px(PICKER_GAP),
            padding: UiRect::all(Val::Px(PICKER_GAP)),
            ..default()
        },
        BackgroundColor(PICKER_BACKGROUND),
        Visibility::Hidden,
    ))
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
}

/// Shown while the inventory is open, in creative
pub fn show_picker(
    info: Res<PlayerInfo>,
    mode: Option<Single<&GameMode, With<Controlled>>>,
    mut picker: Single<&mut Visibility, With<Picker>>,
) {
    let creative = mode.is_some_and(|mode| **mode == GameMode::Creative);
    let shown = if info.menu_open && creative {
        Visibility::Inherited
    } else {
        Visibility::Hidden
    };
    if **picker != shown {
        **picker = shown;
    }
}

/// Clicking an item picks up a full stack of it; clicking the picker's
/// background while holding something throws it away. Hovering lights items up
pub fn click_picker(
    info: Res<PlayerInfo>,
    mouse: Res<ButtonInput<MouseButton>>,
    mut items: Query<(&Interaction, &PickerItem, &mut BackgroundColor)>,
    background: Single<&Interaction, With<Picker>>,
    inventory: Option<Single<&Inventory, With<Controlled>>>,
    mut sender: Single<&mut MessageSender<InventoryAction>>,
) {
    for (interaction, _, mut color) in &mut items {
        let wanted = if *interaction == Interaction::None {
            PICKER_SLOT
        } else {
            PICKER_SLOT_HOVERED
        };
        if color.0 != wanted {
            color.0 = wanted;
        }
    }
    if !info.menu_open || !mouse.just_pressed(BUTTON_PRIMARY) {
        return;
    }
    let hovered = items
        .iter()
        .find(|(interaction, ..)| **interaction != Interaction::None)
        .map(|(_, PickerItem(item), _)| *item);
    let holding = inventory.is_some_and(|inventory| inventory.cursor.is_some());
    match hovered {
        Some(item) => {
            sender.send::<ActionChannel>(InventoryAction::Pick { item: Some(item) });
        }
        None if holding && **background != Interaction::None => {
            sender.send::<ActionChannel>(InventoryAction::Pick { item: None });
        }
        None => {}
    }
}
