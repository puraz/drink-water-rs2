//! 生成水滴图标 PNG 文件
//!
//! 打包前运行一次:
//!   cargo run --bin gen-icons

use std::fs::File;
use std::io::BufWriter;
use std::path::Path;

fn main() {
    let (rgba, w, h) = drink_water_rs2::icon::create_water_drop_rgba();

    let out_dir = Path::new("assets");
    std::fs::create_dir_all(out_dir).expect("创建 assets 目录失败");

    let path = out_dir.join("icon.png");
    let file = File::create(&path).expect("创建 icon.png 失败");
    let wtr = BufWriter::new(file);

    let mut encoder = png::Encoder::new(wtr, w, h);
    encoder.set_color(png::ColorType::Rgba);
    encoder.set_depth(png::BitDepth::Eight);
    // png 0.17 默认使用 sRGB，所以保留 cICP 块

    let mut writer = encoder.write_header().expect("写入 PNG 头失败");
    writer.write_image_data(&rgba).expect("写入 PNG 数据失败");

    eprintln!("✅ 图标已保存 {:?}", path);
    eprintln!("   下一步：");
    eprintln!("   1. cargo install cargo-bundle");
    eprintln!("   2. cargo bundle --bin drink-water-settings");
}
