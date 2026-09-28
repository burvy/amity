//! A scrolling box for the pause menu's pages (Help, Settings): the mouse wheel
//! scrolls whichever one is showing, and a thumb down its right edge shows where
//! you are

use bevy::{input::mouse::AccumulatedMouseScroll, prelude::*};

use super::config::*;

/// A scrolling box, and its scrollbar's thumb
#[derive(Component)]
pub struct Scrolling {
    thumb: Entity,
}

/// Builds a page that fills `parent` and scrolls: `extra` goes on the page's
/// root, and `fill` builds the contents, a column with `row_gap` between rows
pub fn spawn_scrolling(
    parent: &mut ChildSpawnerCommands,
    extra: impl Bundle,
    row_gap: f32,
    fill: impl FnOnce(&mut ChildSpawnerCommands),
) {
    parent
        .spawn((
            Node {
                width: Val::Percent(100.0),
                height: Val::Percent(100.0),
                ..default()
            },
            extra,
        ))
        .with_children(|page| {
            // the thumb, positioned by `scroll`
            let thumb = page
                .spawn((
                    Node {
                        position_type: PositionType::Absolute,
                        right: Val::Px(0.0),
                        width: Val::Px(MENU_SCROLLBAR_WIDTH),
                        ..default()
                    },
                    BackgroundColor(MENU_SCROLLBAR),
                ))
                .id();
            page.spawn((
                Scrolling { thumb },
                Node {
                    flex_direction: FlexDirection::Column,
                    width: Val::Percent(100.0),
                    height: Val::Percent(100.0),
                    overflow: Overflow::scroll_y(),
                    // room for the thumb
                    padding: UiRect::right(Val::Px(MENU_SCROLLBAR_WIDTH * 3.0)),
                    row_gap: Val::Px(row_gap),
                    ..default()
                },
                ScrollPosition::default(),
            ))
            .with_children(fill);
        });
}

/// The mouse wheel scrolls the showing box, and the thumbs follow
pub fn scroll(
    wheel: Res<AccumulatedMouseScroll>,
    mut boxes: Query<(
        &Scrolling,
        &mut ScrollPosition,
        &ComputedNode,
        &InheritedVisibility,
    )>,
    mut thumbs: Query<&mut Node>,
) {
    for (scrolling, mut position, computed, visible) in &mut boxes {
        if !visible.get() {
            continue;
        }
        // sizes in logical pixels, like the scroll position
        let scale = computed.inverse_scale_factor;
        let view = computed.size.y * scale;
        let content = computed.content_size.y * scale;
        let max = (content - view).max(0.0);
        let y = (position.0.y - wheel.delta.y * MENU_SCROLL_SPEED).clamp(0.0, max);
        if position.0.y != y {
            position.0.y = y;
        }

        if let Ok(mut thumb) = thumbs.get_mut(scrolling.thumb) {
            if max <= 0.0 {
                thumb.height = Val::Px(0.0);
            } else {
                let height = view * view / content;
                thumb.height = Val::Px(height);
                thumb.top = Val::Px(y / max * (view - height));
            }
        }
    }
}
