/// Generate a blue water drop icon (normal mode)
pub fn create_water_drop_rgba() -> (Vec<u8>, u32, u32) {
    create_colored_water_drop_rgba((41, 128, 185), (150, 200, 240), 0.6)
}

/// Generate a gray water drop icon (DND mode)
pub fn create_gray_water_drop_rgba() -> (Vec<u8>, u32, u32) {
    create_colored_water_drop_rgba((160, 160, 160), (200, 200, 200), 0.3)
}

/// Generate a water drop icon with custom colors.
///
/// `base` is the main color, `highlight` is the top-left reflection,
/// `brightness` controls the highlight intensity (0.0–1.0).
fn create_colored_water_drop_rgba(
    base: (u8, u8, u8),
    highlight: (u8, u8, u8),
    brightness: f64,
) -> (Vec<u8>, u32, u32) {
    let width = 64u32;
    let height = 64u32;
    let mut rgba = vec![0u8; (width * height * 4) as usize];

    let (r, g, b) = base;
    let (hl_r, hl_g, hl_b) = highlight;

    let cx = width as f64 / 2.0_f64; // 32.0
    let cy = height as f64 * 0.58_f64; // ~37.0 - center of the bottom bulb

    for py in 0..height {
        for px in 0..width {
            let idx = ((py * width + px) * 4) as usize;
            let (x, y) = (px as f64, py as f64);

            if is_inside_water_drop(x, y, width as f64, height as f64) {
                // Base color
                rgba[idx] = r;
                rgba[idx + 1] = g;
                rgba[idx + 2] = b;
                rgba[idx + 3] = 255;

                // Add highlight (top-left light reflection)
                let dx = x - (cx - 8.0);
                let dy = y - (cy - 8.0);
                let dist_hl = (dx * dx + dy * dy).sqrt();
                if dist_hl < 10.0 {
                    let factor = 1.0 - (dist_hl / 10.0).min(1.0);
                    let bright = factor * brightness;
                    rgba[idx] =
                        (rgba[idx] as f64 + (hl_r as f64 - rgba[idx] as f64) * bright) as u8;
                    rgba[idx + 1] = (rgba[idx + 1] as f64
                        + (hl_g as f64 - rgba[idx + 1] as f64) * bright)
                        as u8;
                    rgba[idx + 2] = (rgba[idx + 2] as f64
                        + (hl_b as f64 - rgba[idx + 2] as f64) * bright)
                        as u8;
                }

                // Add a darker edge at the bottom-right for depth
                let dx2 = x - (cx + 6.0);
                let dy2 = y - (cy + 6.0);
                let dist_dark = (dx2 * dx2 + dy2 * dy2).sqrt();
                if dist_dark < 12.0 {
                    let factor = 1.0 - (dist_dark / 12.0).min(1.0);
                    let dark = factor * 0.3;
                    rgba[idx] = (rgba[idx] as f64 * (1.0 - dark)) as u8;
                    rgba[idx + 1] = (rgba[idx + 1] as f64 * (1.0 - dark)) as u8;
                    rgba[idx + 2] = (rgba[idx + 2] as f64 * (1.0 - dark)) as u8;
                }
            }
        }
    }

    (rgba, width, height)
}

/// Check if a point is inside the water drop shape
fn is_inside_water_drop(x: f64, y: f64, w: f64, h: f64) -> bool {
    let cx = w / 2.0_f64;
    let cy = h * 0.58_f64;
    let radius = w * 0.32_f64;

    let top_y = h * 0.06_f64;
    let bottom_y = h * 0.92_f64;

    let normalized_y = (y - top_y) / (bottom_y - top_y);

    // Bottom bulb: circular region
    if normalized_y >= 0.4_f64 {
        let circle_y = (y - cy) / radius;
        if circle_y.abs() > 1.0_f64 {
            return false;
        }
        let half_w = (radius * (1.0_f64 - circle_y * circle_y).sqrt()).max(0.0_f64);
        (x - cx).abs() <= half_w
    } else {
        // Top taper: width goes from ~55% of radius at normalized_y=0.4 to 0 at top
        let taper_frac = normalized_y / 0.4_f64;
        let max_w = radius * 0.55_f64;
        let half_w = max_w * taper_frac;
        (x - cx).abs() <= half_w
    }
}
