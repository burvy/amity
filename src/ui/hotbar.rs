//! The hotbar: our player's items, down the right edge of the screen, with the
//! selected slot outlined. It only reads the replicated `Hotbar` and the selected
//! slot, so it can be restyled or replaced without touching the game logic

use amity_protocol::inventory::{
    config::HOTBAR_SLOTS,
    definition::{GameMode, Hotbar},
};
use bevy::{
    asset::RenderAssetUsages,
    image::{CompressedImageFormats, ImageSampler, ImageType},
    prelude::*,
};
use lightyear::prelude::Controlled;

use super::config::*;
use crate::{
    player::definition::PlayerInfo,
    world::meshing::{material::BlockAtlas, textures::item_tile},
};

/// The slot background, and the frame over the selected slot. Embedded like the
/// block atlas, so native and web load them the same way
const SLOT_IMAGE: &[u8] = include_bytes!("../../assets/images/ui/hotbar-icon.png");
const OUTLINE_IMAGE: &[u8] = include_bytes!("../../assets/images/ui/hotbar-outline.png");

/// Hotbar slot `n`'s selected frame, shown only on the selected slot
#[derive(Component)]
pub struct HotbarOutline(usize);

/// Hotbar slot `n`'s item icon
#[derive(Component)]
pub struct HotbarIcon(usize);

/// Hotbar slot `n`'s item count
#[derive(Component)]
pub struct HotbarCount(usize);

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

/// Builds the empty hotbar once the block atlas (for icons) exists
pub fn spawn_hotbar(mut cmds: Commands, atlas: Res<BlockAtlas>, mut images: ResMut<Assets<Image>>) {
    let slot_image = ui_image(&mut images, SLOT_IMAGE);
    let outline_image = ui_image(&mut images, OUTLINE_IMAGE);
    cmds.spawn(Node {
        // a column along the right edge, centred top to bottom
        position_type: PositionType::Absolute,
        right: Val::Px(HOTBAR_MARGIN),
        top: Val::Px(0.0),
        bottom: Val::Px(0.0),
        flex_direction: FlexDirection::Column,
        justify_content: JustifyContent::Center,
        row_gap: Val::Px(HOTBAR_GAP),
        ..default()
    })
    .with_children(|column| {
        for slot in 0..HOTBAR_SLOTS {
            column
                .spawn((
                    Node {
                        width: Val::Px(HOTBAR_SLOT_SIZE),
                        height: Val::Px(HOTBAR_SLOT_SIZE),
                        align_items: AlignItems::Center,
                        justify_content: JustifyContent::Center,
                        ..default()
                    },
                    ImageNode::new(slot_image.clone()),
                ))
                .with_children(|slot_node| {
                    slot_node.spawn((
                        HotbarIcon(slot),
                        Node {
                            width: Val::Px(HOTBAR_ICON_SIZE),
                            height: Val::Px(HOTBAR_ICON_SIZE),
                            ..default()
                        },
                        ImageNode::from_atlas_image(
                            atlas.image.clone(),
                            TextureAtlas {
                                layout: atlas.layout.clone(),
                                index: 0,
                            },
                        ),
                        Visibility::Hidden,
                    ));
                    slot_node.spawn((
                        HotbarCount(slot),
                        Text::new(""),
                        TextFont {
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
                    // spawned last, so it's drawn over the icon and count
                    slot_node.spawn((
                        HotbarOutline(slot),
                        Node {
                            position_type: PositionType::Absolute,
                            width: Val::Percent(100.0),
                            height: Val::Percent(100.0),
                            ..default()
                        },
                        ImageNode::new(outline_image.clone()),
                        Visibility::Hidden,
                    ));
                });
        }
    });
}

/// Shows what's in each slot and which is selected
pub fn update_hotbar(
    player: Option<Single<(&Hotbar, &GameMode), With<Controlled>>>,
    info: Res<PlayerInfo>,
    mut outlines: Query<(&HotbarOutline, &mut Visibility), Without<HotbarIcon>>,
    mut icons: Query<(&HotbarIcon, &mut ImageNode, &mut Visibility), Without<HotbarOutline>>,
    mut counts: Query<(&HotbarCount, &mut Text)>,
) {
    let Some(player) = player else {
        return;
    };
    let (hotbar, mode) = player.into_inner();

    for (HotbarOutline(slot), mut visibility) in &mut outlines {
        let shown = if *slot == info.slot as usize {
            Visibility::Inherited
        } else {
            Visibility::Hidden
        };
        if *visibility != shown {
            *visibility = shown;
        }
    }
    for (HotbarIcon(slot), mut image, mut visibility) in &mut icons {
        match hotbar.get(*slot) {
            Some(stack) => {
                if let Some(atlas) = &mut image.texture_atlas {
                    atlas.index = item_tile(stack.item) as usize;
                }
                *visibility = Visibility::Inherited;
            }
            None => *visibility = Visibility::Hidden,
        }
    }
    for (HotbarCount(slot), mut text) in &mut counts {
        // creative items are endless, so no count
        let count = match hotbar.get(*slot) {
            Some(stack) if *mode == GameMode::Survival && stack.count > 1 => stack.count.to_string(),
            _ => String::new(),
        };
        if text.0 != count {
            text.0 = count;
        }
    }
}
