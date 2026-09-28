//! The hotbar, the inventory that slides out beside it, and the item held on the
//! mouse cursor.
//!
//! The hotbar is always on screen: one column of slots down the right edge, the
//! selected one framed. Holding Tab slides 4 more columns out from the right
//! edge beside it, and frees the mouse to click slots. Clicks only *ask* the
//! server (`InventoryAction`); what's drawn is always the replicated `Inventory`,
//! so this can be restyled or replaced without touching the game logic.

use amity_protocol::inventory::{
    config::{HOTBAR_SLOTS, INVENTORY_SLOTS},
    definition::{ActionChannel, Click, GameMode, Inventory, InventoryAction},
};
use bevy::{
    asset::RenderAssetUsages,
    image::{CompressedImageFormats, ImageSampler, ImageType},
    prelude::*,
    window::{CursorGrabMode, CursorOptions, PrimaryWindow},
};
use lightyear::prelude::{Controlled, MessageSender};

use super::{config::*, font::UiFont};
use crate::{
    player::{
        config::{BUTTON_PRIMARY, BUTTON_SECONDARY, KEY_INVENTORY},
        definition::PlayerInfo,
    },
    world::meshing::{material::BlockAtlas, textures::item_tile},
};

/// The slot background, and the frame over the selected hotbar slot. Embedded
/// like the block atlas, so native and web load them the same way
const SLOT_IMAGE: &[u8] = include_bytes!("../../assets/images/ui/hotbar-icon.png");
const OUTLINE_IMAGE: &[u8] = include_bytes!("../../assets/images/ui/hotbar-outline.png");

/// Columns in the slide-out part (each as tall as the hotbar)
const INVENTORY_COLUMNS: usize = (INVENTORY_SLOTS - HOTBAR_SLOTS) / HOTBAR_SLOTS;

/// A clickable inventory slot, by slot number
#[derive(Component)]
pub struct Slot(usize);

/// Slot `n`'s item icon
#[derive(Component)]
pub struct SlotIcon(usize);

/// Slot `n`'s item count
#[derive(Component)]
pub struct SlotCount(usize);

/// Hotbar slot `n`'s selected frame, shown only on the selected slot
#[derive(Component)]
pub struct SlotOutline(usize);

/// The slide-out panel, and how far out it is: 0 hidden .. 1 fully out
#[derive(Component, Default)]
pub struct InventoryPanel(f32);

/// The item held on the mouse cursor: its icon and count
#[derive(Component)]
pub struct CursorItem;

#[derive(Component)]
pub struct CursorCount;

/// Loads an embedded UI png. Nearest sampling keeps pixel art crisp when scaled
fn ui_image(images: &mut Assets<Image>, png: &[u8]) -> Handle<Image> {
    images.add(
        Image::from_buffer(
            png,
            ImageType::Extension("png"),
            CompressedImageFormats::NONE,
            true,
            ImageSampler::nearest(),
            RenderAssetUsages::RENDER_WORLD,
        )
        .expect("UI images are embedded at build time, so they must be valid pngs"),
    )
}

/// What every slot's widget needs
struct SlotStyle {
    background: Handle<Image>,
    outline: Handle<Image>,
    atlas: Handle<Image>,
    layout: Handle<TextureAtlasLayout>,
    font: Handle<Font>,
}

/// A column of slots `first..first + HOTBAR_SLOTS`
fn spawn_column(parent: &mut ChildSpawnerCommands, first: usize, style: &SlotStyle) {
    parent
        .spawn(Node {
            flex_direction: FlexDirection::Column,
            row_gap: Val::Px(HOTBAR_GAP),
            ..default()
        })
        .with_children(|column| {
            for slot in first..first + HOTBAR_SLOTS {
                spawn_slot(column, slot, style);
            }
        });
}

/// One slot: its background, the item icon, the count, and (hotbar only) the
/// selected frame on top
fn spawn_slot(parent: &mut ChildSpawnerCommands, slot: usize, style: &SlotStyle) {
    parent
        .spawn((
            Slot(slot),
            // Interaction: hovering and clicking, for the open inventory
            Button,
            Node {
                width: Val::Px(HOTBAR_SLOT_SIZE),
                height: Val::Px(HOTBAR_SLOT_SIZE),
                align_items: AlignItems::Center,
                justify_content: JustifyContent::Center,
                ..default()
            },
            ImageNode::new(style.background.clone()),
        ))
        .with_children(|node| {
            node.spawn((
                SlotIcon(slot),
                Node {
                    width: Val::Px(HOTBAR_ICON_SIZE),
                    height: Val::Px(HOTBAR_ICON_SIZE),
                    ..default()
                },
                ImageNode::from_atlas_image(
                    style.atlas.clone(),
                    TextureAtlas {
                        layout: style.layout.clone(),
                        index: 0,
                    },
                ),
                Visibility::Hidden,
            ));
            node.spawn((
                SlotCount(slot),
                Text::new(""),
                TextFont {
                    font: style.font.clone().into(),
                    font_size: FontSize::Px(HOTBAR_COUNT_SIZE),
                    ..default()
                },
                Node {
                    position_type: PositionType::Absolute,
                    right: Val::Px(2.0),
                    bottom: Val::Px(0.0),
                    ..default()
                },
            ));
            if Inventory::is_hotbar(slot) {
                // spawned last, so it's drawn over the icon and count
                node.spawn((
                    SlotOutline(slot),
                    Node {
                        position_type: PositionType::Absolute,
                        width: Val::Percent(100.0),
                        height: Val::Percent(100.0),
                        ..default()
                    },
                    ImageNode::new(style.outline.clone()),
                    Visibility::Hidden,
                ));
            }
        });
}

/// Pixels across the slide-out columns (with the gap before the hotbar)
fn panel_width() -> f32 {
    INVENTORY_COLUMNS as f32 * (HOTBAR_SLOT_SIZE + HOTBAR_GAP)
}

/// Where the panel sits inside its clip, `out` of the way (0..1) from hidden to
/// fully out, as pixels from the clip's right edge (the hotbar's left edge).
/// Hidden, it's entirely past that edge: behind the hotbar, cut off by the clip.
/// Out, it's one gap left of the hotbar
fn panel_right(out: f32) -> f32 {
    let hidden = -panel_width();
    let shown = 0.0;
    hidden + (shown - hidden) * out
}

/// Builds the hotbar, the (hidden) inventory panel and the cursor item, once the
/// block atlas (for icons) exists
pub fn spawn_inventory(
    mut cmds: Commands,
    atlas: Res<BlockAtlas>,
    font: Res<UiFont>,
    mut images: ResMut<Assets<Image>>,
) {
    let style = SlotStyle {
        background: ui_image(&mut images, SLOT_IMAGE),
        outline: ui_image(&mut images, OUTLINE_IMAGE),
        atlas: atlas.image.clone(),
        layout: atlas.layout.clone(),
        font: font.0.clone(),
    };
    let centred = |right: f32| Node {
        position_type: PositionType::Absolute,
        right: Val::Px(right),
        top: Val::Px(0.0),
        bottom: Val::Px(0.0),
        flex_direction: FlexDirection::RowReverse,
        align_items: AlignItems::Center,
        column_gap: Val::Px(HOTBAR_GAP),
        ..default()
    };

    // The panel slides out from behind the hotbar and back in. It lives in a
    // clip whose right edge is the hotbar's left edge: whatever of it is still
    // "behind" the hotbar is past that edge, so it's cut off, not drawn over it
    cmds.spawn(Node {
        position_type: PositionType::Absolute,
        right: Val::Px(HOTBAR_MARGIN + HOTBAR_SLOT_SIZE),
        top: Val::Px(0.0),
        bottom: Val::Px(0.0),
        width: Val::Px(panel_width()),
        overflow: Overflow::clip(),
        ..default()
    })
    .with_children(|clip| {
        clip.spawn((
            InventoryPanel::default(),
            Node {
                padding: UiRect::right(Val::Px(HOTBAR_GAP)),
                ..centred(panel_right(0.0))
            },
        ))
        .with_children(|panel| {
            // right to left: the column next to the hotbar holds slots 5..10
            for column in 0..INVENTORY_COLUMNS {
                spawn_column(panel, HOTBAR_SLOTS + column * HOTBAR_SLOTS, &style);
            }
        });
    });
    cmds.spawn(centred(HOTBAR_MARGIN)).with_children(|hotbar| {
        spawn_column(hotbar, 0, &style);
    });

    // what's held on the mouse: follows the cursor, above everything
    cmds.spawn((
        CursorItem,
        Node {
            position_type: PositionType::Absolute,
            width: Val::Px(CURSOR_ICON_SIZE),
            height: Val::Px(CURSOR_ICON_SIZE),
            ..default()
        },
        ImageNode::from_atlas_image(
            style.atlas.clone(),
            TextureAtlas {
                layout: style.layout.clone(),
                index: 0,
            },
        ),
        GlobalZIndex(10),
        Visibility::Hidden,
    ))
    .with_children(|item| {
        item.spawn((
            CursorCount,
            Text::new(""),
            TextFont {
                font: style.font.clone().into(),
                font_size: FontSize::Px(HOTBAR_COUNT_SIZE),
                ..default()
            },
            Node {
                position_type: PositionType::Absolute,
                right: Val::Px(-4.0),
                bottom: Val::Px(-4.0),
                ..default()
            },
        ));
    });
}

/// Holding Tab opens the inventory (unless paused), letting go closes it: the
/// mouse is freed while it's open, and on closing, the server puts whatever was
/// on the cursor back into the slots
pub fn open_inventory(
    keys: Res<ButtonInput<KeyCode>>,
    mut info: ResMut<PlayerInfo>,
    mut cursor: Single<&mut CursorOptions, With<PrimaryWindow>>,
    mut sender: Single<&mut MessageSender<InventoryAction>>,
) {
    let open = keys.pressed(KEY_INVENTORY) && !info.paused;
    if open == info.menu_open {
        return;
    }
    info.menu_open = open;
    if open {
        cursor.grab_mode = CursorGrabMode::None;
        cursor.visible = true;
    } else {
        sender.send::<ActionChannel>(InventoryAction::Close);
        // back to playing, unless the game was paused meanwhile
        if !info.paused {
            cursor.grab_mode = CursorGrabMode::Locked;
            cursor.visible = false;
        }
    }
}

/// Slides the panel towards out (open) or in (closed)
pub fn slide_panel(
    info: Res<PlayerInfo>,
    time: Res<Time>,
    mut panel: Single<(&mut InventoryPanel, &mut Node)>,
) {
    let (panel, node) = &mut *panel;
    let target = if info.menu_open { 1.0 } else { 0.0 };
    // ease towards the target: most of the way within INVENTORY_SLIDE_SECS
    let step = 1.0 - (-time.delta_secs() * 3.0 / INVENTORY_SLIDE_SECS).exp();
    panel.0 += (target - panel.0) * step;
    if (target - panel.0).abs() < 0.001 {
        panel.0 = target;
    }
    node.right = Val::Px(panel_right(panel.0));
}

/// Clicking a slot while the inventory is open: left or right button, sent to
/// the server to carry out (see `Inventory::click`)
pub fn click_slots(
    info: Res<PlayerInfo>,
    mouse: Res<ButtonInput<MouseButton>>,
    slots: Query<(&Slot, &Interaction)>,
    mut sender: Single<&mut MessageSender<InventoryAction>>,
) {
    if !info.menu_open {
        return;
    }
    let click = if mouse.just_pressed(BUTTON_PRIMARY) {
        Click::Left
    } else if mouse.just_pressed(BUTTON_SECONDARY) {
        Click::Right
    } else {
        return;
    };
    // bevy only tracks the left button, so find the slot under the mouse by hover
    if let Some((Slot(slot), _)) = slots
        .iter()
        .find(|(_, interaction)| **interaction != Interaction::None)
    {
        sender.send::<ActionChannel>(InventoryAction::Click { slot: *slot, click });
    }
}

/// Shows every slot's item and count, the hotbar's selected frame, and the item
/// held on the cursor
pub fn update_slots(
    player: Option<Single<(&Inventory, &GameMode), With<Controlled>>>,
    info: Res<PlayerInfo>,
    window: Single<&Window, With<PrimaryWindow>>,
    mut outlines: Query<(&SlotOutline, &mut Visibility), (Without<SlotIcon>, Without<CursorItem>)>,
    mut icons: Query<(&SlotIcon, &mut ImageNode, &mut Visibility), (Without<SlotOutline>, Without<CursorItem>)>,
    mut counts: Query<(&SlotCount, &mut Text), Without<CursorCount>>,
    mut cursor_item: Single<
        (&mut Node, &mut ImageNode, &mut Visibility),
        (With<CursorItem>, Without<SlotIcon>, Without<SlotOutline>),
    >,
    mut cursor_count: Single<&mut Text, (With<CursorCount>, Without<SlotCount>)>,
) {
    let Some(player) = player else {
        return;
    };
    let (inventory, mode) = player.into_inner();
    // creative items are endless, so no counts
    let count_text = |count: u32| {
        if *mode == GameMode::Survival && count > 1 {
            count.to_string()
        } else {
            String::new()
        }
    };

    for (SlotOutline(slot), mut visibility) in &mut outlines {
        let shown = if *slot == info.slot as usize {
            Visibility::Inherited
        } else {
            Visibility::Hidden
        };
        if *visibility != shown {
            *visibility = shown;
        }
    }
    for (SlotIcon(slot), mut image, mut visibility) in &mut icons {
        show_item(inventory.get(*slot), &mut image, &mut visibility);
    }
    for (SlotCount(slot), mut text) in &mut counts {
        let count = inventory.get(*slot).map_or(String::new(), |stack| count_text(stack.count));
        if text.0 != count {
            text.0 = count;
        }
    }

    // the cursor's item sits under the mouse pointer, while the inventory is open
    let (node, image, visibility) = &mut *cursor_item;
    let held = inventory.cursor.filter(|_| info.menu_open);
    show_item(held, image, visibility);
    if let Some(position) = window.cursor_position() {
        node.left = Val::Px(position.x - CURSOR_ICON_SIZE / 2.0);
        node.top = Val::Px(position.y - CURSOR_ICON_SIZE / 2.0);
    }
    let count = held.map_or(String::new(), |stack| count_text(stack.count));
    if cursor_count.0 != count {
        cursor_count.0 = count;
    }
}

/// An item's icon, or hidden for no item
fn show_item(
    stack: Option<amity_protocol::inventory::definition::ItemStack>,
    image: &mut ImageNode,
    visibility: &mut Visibility,
) {
    match stack {
        Some(stack) => {
            if let Some(atlas) = &mut image.texture_atlas {
                let tile = item_tile(stack.item) as usize;
                if atlas.index != tile {
                    atlas.index = tile;
                }
            }
            if *visibility != Visibility::Inherited {
                *visibility = Visibility::Inherited;
            }
        }
        None => {
            if *visibility != Visibility::Hidden {
                *visibility = Visibility::Hidden;
            }
        }
    }
}
