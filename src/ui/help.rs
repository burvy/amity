//! The Help screen: `assets/help.md` shown as formatted, scrollable text.
//!
//! The Markdown is parsed (with pulldown-cmark) into simple blocks, headings,
//! paragraphs, list items, rules and tables, which are then built as UI nodes.
//! Supported: `#`..`###` headings, paragraphs, `-` and `1.` lists (nested),
//! `**bold**`, `*italic*`, `` `code` ``, `[links](...)` (shown as text),
//! `---` rules, code blocks and tables.

use bevy::{input::mouse::AccumulatedMouseScroll, prelude::*};
use pulldown_cmark::{Event, Options, Parser, Tag, TagEnd};

use super::{config::*, font::UiFont};

/// The help text. Embedded like the textures, so native and web show the same
const HELP: &str = include_str!("../../assets/help.md");

/// A run of text in one style
#[derive(Clone, Debug, PartialEq)]
struct Span {
    text: String,
    bold: bool,
    italic: bool,
    code: bool,
}

/// One block of the page, top to bottom
#[derive(Clone, Debug, PartialEq)]
enum Block {
    Heading(u8, Vec<Span>),
    Paragraph(Vec<Span>),
    /// how deeply nested, its marker ("•" or "3."), and its text
    ListItem(usize, String, Vec<Span>),
    Rule,
    Code(String),
    /// rows of cells; the first row is the header
    Table(Vec<Vec<Vec<Span>>>),
}

/// Turns Markdown into blocks. Anything not supported (images, HTML) is skipped
fn parse(markdown: &str) -> Vec<Block> {
    let mut blocks = Vec::new();
    // text collected for the block being read
    let mut spans: Vec<Span> = Vec::new();
    let (mut bold, mut italic) = (false, false);
    // each open list: its next number, or None for bullets
    let mut lists: Vec<Option<u64>> = Vec::new();
    let mut item_marker = String::new();
    let mut code_block: Option<String> = None;
    let mut table: Option<Vec<Vec<Vec<Span>>>> = None;

    let push_text = |spans: &mut Vec<Span>, text: &str, bold, italic, code| {
        spans.push(Span {
            text: text.to_string(),
            bold,
            italic,
            code,
        });
    };

    for event in Parser::new_ext(markdown, Options::ENABLE_TABLES) {
        match event {
            Event::Start(Tag::Strong) => bold = true,
            Event::End(TagEnd::Strong) => bold = false,
            Event::Start(Tag::Emphasis) => italic = true,
            Event::End(TagEnd::Emphasis) => italic = false,
            Event::Start(Tag::List(first)) => {
                // a list inside an item: the item's own text comes first
                if !lists.is_empty() && !spans.is_empty() {
                    let text = std::mem::take(&mut spans);
                    blocks.push(Block::ListItem(lists.len() - 1, item_marker.clone(), text));
                }
                lists.push(first);
            }
            Event::End(TagEnd::List(_)) => {
                lists.pop();
            }
            Event::End(TagEnd::Item) => {
                // items without blank lines between them have no paragraph: their
                // text is still waiting here
                if !spans.is_empty() {
                    let text = std::mem::take(&mut spans);
                    blocks.push(Block::ListItem(lists.len() - 1, item_marker.clone(), text));
                }
            }
            Event::Start(Tag::Item) => {
                item_marker = match lists.last_mut() {
                    Some(Some(number)) => {
                        *number += 1;
                        format!("{}.", *number - 1)
                    }
                    _ => "•".into(),
                };
            }
            Event::Start(Tag::CodeBlock(_)) => code_block = Some(String::new()),
            Event::End(TagEnd::CodeBlock) => {
                blocks.push(Block::Code(code_block.take().unwrap_or_default()));
            }
            Event::Start(Tag::Table(_)) => table = Some(Vec::new()),
            Event::End(TagEnd::Table) => blocks.push(Block::Table(table.take().unwrap_or_default())),
            Event::Start(Tag::TableHead) | Event::Start(Tag::TableRow) => {
                if let Some(table) = &mut table {
                    table.push(Vec::new());
                }
            }
            Event::End(TagEnd::TableCell) => {
                if let Some(row) = table.as_mut().and_then(|table| table.last_mut()) {
                    row.push(std::mem::take(&mut spans));
                }
            }
            Event::Text(text) => match &mut code_block {
                Some(code) => code.push_str(&text),
                None => push_text(&mut spans, &text, bold, italic, false),
            },
            Event::Code(text) => push_text(&mut spans, &text, bold, italic, true),
            Event::SoftBreak => push_text(&mut spans, " ", bold, italic, false),
            Event::HardBreak => push_text(&mut spans, "\n", bold, italic, false),
            Event::Rule => blocks.push(Block::Rule),
            Event::End(TagEnd::Heading(level)) => {
                blocks.push(Block::Heading(level as u8, std::mem::take(&mut spans)));
            }
            Event::End(TagEnd::Paragraph) => {
                let text = std::mem::take(&mut spans);
                // a paragraph inside a list item is the item's text
                if lists.is_empty() {
                    blocks.push(Block::Paragraph(text));
                } else {
                    blocks.push(Block::ListItem(lists.len() - 1, item_marker.clone(), text));
                }
            }
            _ => {}
        }
    }
    blocks
}

/// The help page's scrolling box
#[derive(Component)]
pub struct HelpScroll;

/// The scrollbar's thumb
#[derive(Component)]
pub struct HelpThumb;

/// Builds the help page into `parent` (the pause menu's panel), with `extra`
/// components on its root
pub fn spawn_help(parent: &mut ChildSpawnerCommands, font: &UiFont, extra: impl Bundle) {
    let blocks = parse(HELP);
    let mut page = parent.spawn((
        Node {
            width: Val::Percent(100.0),
            height: Val::Percent(100.0),
            ..default()
        },
        extra,
    ));
    page.with_children(|page| {
        page.spawn((
            HelpScroll,
            Node {
                flex_direction: FlexDirection::Column,
                width: Val::Percent(100.0),
                height: Val::Percent(100.0),
                overflow: Overflow::scroll_y(),
                padding: UiRect::right(Val::Px(MENU_SCROLLBAR_WIDTH * 3.0)),
                row_gap: Val::Px(HELP_BLOCK_GAP),
                ..default()
            },
            ScrollPosition::default(),
        ))
        .with_children(|scroll| {
            for block in &blocks {
                spawn_block(scroll, block, font);
            }
        });
        // the scrollbar: a thumb down the right edge
        page.spawn((
            HelpThumb,
            Node {
                position_type: PositionType::Absolute,
                right: Val::Px(0.0),
                width: Val::Px(MENU_SCROLLBAR_WIDTH),
                ..default()
            },
            BackgroundColor(MENU_SCROLLBAR),
        ));
    });
}

fn spawn_block(parent: &mut ChildSpawnerCommands, block: &Block, font: &UiFont) {
    match block {
        Block::Heading(level, spans) => {
            let size = if *level == 1 { HELP_TITLE_SIZE } else { HELP_TEXT_SIZE };
            spawn_text(parent, spans, font, size, MENU_HEADING, Node {
                margin: UiRect::top(Val::Px(HELP_BLOCK_GAP)),
                ..default()
            });
        }
        Block::Paragraph(spans) => spawn_text(parent, spans, font, HELP_TEXT_SIZE, MENU_TEXT, Node::default()),
        Block::ListItem(depth, marker, spans) => {
            parent
                .spawn(Node {
                    margin: UiRect::left(Val::Px(HELP_INDENT * (*depth as f32 + 1.0))),
                    column_gap: Val::Px(HELP_INDENT / 2.0),
                    ..default()
                })
                .with_children(|row| {
                    let marker = vec![Span {
                        text: marker.clone(),
                        bold: false,
                        italic: false,
                        code: false,
                    }];
                    spawn_text(row, &marker, font, HELP_TEXT_SIZE, MENU_TEXT_DIM, Node::default());
                    spawn_text(row, spans, font, HELP_TEXT_SIZE, MENU_TEXT, Node {
                        flex_shrink: 1.0,
                        ..default()
                    });
                });
        }
        Block::Rule => {
            parent.spawn((
                Node {
                    height: Val::Px(2.0),
                    width: Val::Percent(100.0),
                    ..default()
                },
                BackgroundColor(MENU_TEXT_DIM),
            ));
        }
        Block::Code(code) => {
            parent
                .spawn((
                    Node {
                        padding: UiRect::all(Val::Px(HELP_INDENT / 2.0)),
                        ..default()
                    },
                    BackgroundColor(MENU_CODE_BACKGROUND),
                ))
                .with_children(|code_box| {
                    let text = vec![Span {
                        text: code.trim_end().to_string(),
                        bold: false,
                        italic: false,
                        code: true,
                    }];
                    spawn_text(code_box, &text, font, HELP_TEXT_SIZE, MENU_CODE, Node::default());
                });
        }
        Block::Table(rows) => {
            let columns = rows.iter().map(Vec::len).max().unwrap_or(1) as u16;
            parent
                .spawn(Node {
                    display: Display::Grid,
                    grid_template_columns: RepeatedGridTrack::auto(columns),
                    column_gap: Val::Px(HELP_INDENT),
                    row_gap: Val::Px(HELP_BLOCK_GAP / 3.0),
                    ..default()
                })
                .with_children(|grid| {
                    for (row_index, row) in rows.iter().enumerate() {
                        for cell in row {
                            // the header row is headed like a heading
                            let colour = if row_index == 0 { MENU_HEADING } else { MENU_TEXT };
                            spawn_text(grid, cell, font, HELP_TEXT_SIZE, colour, Node::default());
                        }
                    }
                });
        }
    }
}

/// A text node made of styled spans: bold is brighter, italic and code tinted
/// (the pixel font has no bold or italic faces)
fn spawn_text(
    parent: &mut ChildSpawnerCommands,
    spans: &[Span],
    font: &UiFont,
    size: f32,
    colour: Color,
    node: Node,
) {
    let text_font = TextFont {
        font: font.0.clone().into(),
        font_size: FontSize::Px(size),
        ..default()
    };
    parent
        .spawn((Text::new(""), text_font.clone(), TextColor(colour), node))
        .with_children(|text| {
            for span in spans {
                let span_colour = if span.code {
                    MENU_CODE
                } else if span.bold {
                    MENU_BOLD
                } else if span.italic {
                    MENU_ITALIC
                } else {
                    colour
                };
                text.spawn((
                    TextSpan(span.text.clone()),
                    text_font.clone(),
                    TextColor(span_colour),
                ));
            }
        });
}

/// The mouse wheel scrolls the page, and the thumb shows where you are
pub fn scroll_help(
    wheel: Res<AccumulatedMouseScroll>,
    mut scroll: Query<(&mut ScrollPosition, &ComputedNode, &InheritedVisibility), With<HelpScroll>>,
    mut thumb: Query<&mut Node, With<HelpThumb>>,
) {
    let Ok((mut position, computed, visible)) = scroll.single_mut() else {
        return;
    };
    if !visible.get() {
        return;
    }
    // sizes in logical pixels, like the scroll position
    let scale = computed.inverse_scale_factor;
    let view = computed.size.y * scale;
    let content = computed.content_size.y * scale;
    let max = (content - view).max(0.0);
    position.0.y = (position.0.y - wheel.delta.y * HELP_SCROLL_SPEED).clamp(0.0, max);

    if let Ok(mut thumb) = thumb.single_mut() {
        if max <= 0.0 {
            thumb.height = Val::Px(0.0);
        } else {
            let height = view * view / content;
            thumb.height = Val::Px(height);
            thumb.top = Val::Px(position.0.y / max * (view - height));
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn plain(spans: &[Span]) -> String {
        spans.iter().map(|span| span.text.as_str()).collect()
    }

    #[test]
    fn headings_paragraphs_and_styles() {
        let blocks = parse("# Title\n\nSome **bold** and `code`.\n\n---\n");
        assert!(matches!(&blocks[0], Block::Heading(1, spans) if plain(spans) == "Title"));
        let Block::Paragraph(spans) = &blocks[1] else { panic!("{blocks:?}") };
        assert_eq!(plain(spans), "Some bold and code.");
        assert!(spans.iter().any(|span| span.bold && span.text == "bold"));
        assert!(spans.iter().any(|span| span.code && span.text == "code"));
        assert_eq!(blocks[2], Block::Rule);
    }

    #[test]
    fn lists_nest_and_number() {
        let blocks = parse("- one\n  - inner\n- two\n\n1. first\n2. second\n");
        let items: Vec<(usize, String, String)> = blocks
            .iter()
            .filter_map(|block| match block {
                Block::ListItem(depth, marker, spans) => Some((*depth, marker.clone(), plain(spans))),
                _ => None,
            })
            .collect();
        assert_eq!(items[0], (0, "•".into(), "one".into()));
        assert_eq!(items[1], (1, "•".into(), "inner".into()));
        assert_eq!(items[2], (0, "•".into(), "two".into()));
        assert_eq!(items[3], (0, "1.".into(), "first".into()));
        assert_eq!(items[4], (0, "2.".into(), "second".into()));
    }

    #[test]
    fn tables_keep_rows_and_cells() {
        let blocks = parse("| A | B |\n|---|---|\n| 1 | 2 |\n");
        let Block::Table(rows) = &blocks[0] else { panic!("{blocks:?}") };
        assert_eq!(rows.len(), 2);
        assert_eq!(plain(&rows[1][1]), "2");
    }

    #[test]
    fn the_real_help_file_parses() {
        assert!(parse(HELP).len() > 10);
    }
}
