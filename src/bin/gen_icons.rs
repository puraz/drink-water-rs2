//! Generate the water-drop icon as a PNG file.
//!
//! Run once before bundling:
//!   cargo run --bin gen-icons

use std::fs::File;
use std::io::BufWriter;
use std::path::Path;

fn main() {
    let (rgba, w, h) = drink_water_rs2::icon::create_water_drop_rgba();

    let out_dir = Path::new("assets");
    std::fs::create_dir_all(out_dir).expect("create assets/");

    let path = out_dir.join("icon.png");
    let file = File::create(&path).expect("create icon.png");
    let wtr = BufWriter::new(file);

    let mut encoder = png::Encoder::new(wtr, w, h);
    encoder.set_color(png::ColorType::Rgba);
    encoder.set_depth(png::BitDepth::Eight);
    // Keep write to cICP chunk since png 0.17 uses srgb by default

    let mut writer = encoder.write_header().expect("png header");
    writer.write_image_data(&rgba).expect("png data");

    eprintln!("✅ Icon saved to {:?}", path);
    eprintln!("   Next steps:");
    eprintln!("   1. cargo install cargo-bundle");
    eprintln!("   2. cargo bundle --bin drink-water-settings");
}
