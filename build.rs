//! Packs every PNG in `assets/images/blocks` into one texture atlas for the block
//! mesher, and writes the name -> tile table next to it. Both land in OUT_DIR and
//! are embedded by `src/world/mesh.rs`, so adding a texture is just adding a file.

use std::{
    env,
    fs::{self, File},
    io::{BufReader, BufWriter},
    path::{Path, PathBuf},
};

const SOURCE: &str = "assets/images/blocks";
const TILE_PIXELS: u32 = 8;
const COLUMNS: u32 = 16;

fn main() {
    println!("cargo::rerun-if-changed={SOURCE}");

    let mut textures: Vec<(String, Vec<u8>)> = fs::read_dir(SOURCE)
        .unwrap_or_else(|e| panic!("can't read {SOURCE}: {e}"))
        .map(|entry| entry.expect("readable directory entry").path())
        .filter(|path| path.extension().is_some_and(|ext| ext == "png"))
        .map(|path| {
            let name = path.file_stem().unwrap().to_string_lossy().into_owned();
            (name, read_tile(&path))
        })
        .collect();
    // sorted so the same files always produce the same atlas
    textures.sort_by(|a, b| a.0.cmp(&b.0));

    // tile 0 is the "missing texture" checker; textures follow from tile 1
    let rows = (1 + textures.len() as u32).div_ceil(COLUMNS);
    let (width, height) = (COLUMNS * TILE_PIXELS, rows * TILE_PIXELS);
    let mut atlas = vec![0; (width * height * 4) as usize];
    blit(&mut atlas, width, 0, &missing_tile());
    for (i, (_, pixels)) in textures.iter().enumerate() {
        blit(&mut atlas, width, i as u32 + 1, pixels);
    }

    let out = PathBuf::from(env::var("OUT_DIR").unwrap());
    write_png(&out.join("atlas.png"), width, height, &atlas);

    let entries: String = textures
        .iter()
        .enumerate()
        .map(|(i, (name, _))| format!("    ({name:?}, {}),\n", i + 1))
        .collect();
    fs::write(
        out.join("atlas.rs"),
        format!(
            "pub const COLUMNS: u32 = {COLUMNS};\n\
             pub const ROWS: u32 = {rows};\n\
             pub const TILE_PIXELS: u32 = {TILE_PIXELS};\n\
             pub const MISSING: u32 = 0;\n\
             pub const TILES: &[(&str, u32)] = &[\n{entries}];\n"
        ),
    )
    .unwrap();
}

/// Decodes one texture to 8-bit RGBA, whatever colour type it was saved as
fn read_tile(path: &Path) -> Vec<u8> {
    let file = File::open(path).unwrap_or_else(|e| panic!("{}: {e}", path.display()));
    let mut decoder = png::Decoder::new(BufReader::new(file));
    decoder.set_transformations(png::Transformations::EXPAND | png::Transformations::STRIP_16);
    let mut reader = decoder.read_info().unwrap_or_else(|e| {
        panic!(
            "{}: not a readable png ({e}). If it's a git-lfs pointer, run `git lfs pull`",
            path.display()
        )
    });
    let mut buf = vec![0; reader.output_buffer_size().unwrap()];
    let info = reader
        .next_frame(&mut buf)
        .unwrap_or_else(|e| panic!("{}: {e}", path.display()));
    assert!(
        info.width == TILE_PIXELS && info.height == TILE_PIXELS,
        "{} is {}x{}, block textures must be {TILE_PIXELS}x{TILE_PIXELS}",
        path.display(),
        info.width,
        info.height
    );
    let pixels = &buf[..info.buffer_size()];
    match info.color_type {
        png::ColorType::Rgba => pixels.to_vec(),
        png::ColorType::Rgb => pixels.chunks(3).flat_map(|p| [p[0], p[1], p[2], 255]).collect(),
        png::ColorType::GrayscaleAlpha => pixels.chunks(2).flat_map(|p| [p[0], p[0], p[0], p[1]]).collect(),
        png::ColorType::Grayscale => pixels.iter().flat_map(|&g| [g, g, g, 255]).collect(),
        png::ColorType::Indexed => unreachable!("EXPAND turns palettes into RGB(A)"),
    }
}

/// Copies an 8x8 RGBA tile into its slot in the atlas
fn blit(atlas: &mut [u8], atlas_width: u32, tile: u32, pixels: &[u8]) {
    let (column, row) = (tile % COLUMNS, tile / COLUMNS);
    let row_bytes = (TILE_PIXELS * 4) as usize;
    for y in 0..TILE_PIXELS {
        let src = (y * TILE_PIXELS * 4) as usize;
        let dst = (((row * TILE_PIXELS + y) * atlas_width + column * TILE_PIXELS) * 4) as usize;
        atlas[dst..dst + row_bytes].copy_from_slice(&pixels[src..src + row_bytes]);
    }
}

/// Magenta and black checker, so a block without a texture is obvious in game
fn missing_tile() -> Vec<u8> {
    (0..TILE_PIXELS * TILE_PIXELS)
        .flat_map(|i| {
            let (x, y) = (i % TILE_PIXELS, i / TILE_PIXELS);
            if (x / 4 + y / 4) % 2 == 0 {
                [255, 0, 255, 255]
            } else {
                [0, 0, 0, 255]
            }
        })
        .collect()
}

fn write_png(path: &Path, width: u32, height: u32, rgba: &[u8]) {
    let mut encoder = png::Encoder::new(BufWriter::new(File::create(path).unwrap()), width, height);
    encoder.set_color(png::ColorType::Rgba);
    encoder.set_depth(png::BitDepth::Eight);
    encoder.write_header().unwrap().write_image_data(rgba).unwrap();
}
