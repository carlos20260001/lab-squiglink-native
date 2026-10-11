//! Color palette and algorithm for CrinGraph curve coloring.

use egui::Color32;

const LD_P1: f64 = 1.1673039782614187;
const LD_P2: f64 = LD_P1 * LD_P1;
const LD_P3: f64 = LD_P2 * LD_P1;

/// Return the original CrinGraph golden-angle palette color for a curve.
pub fn get_curve_color(id: usize) -> Color32 {
    get_curve_color_with_offset(id, 0)
}

/// Return a palette color shifted by a stable sequence offset.
pub fn get_curve_color_with_offset(id: usize, palette_shift: usize) -> Color32 {
    get_curve_color_at(id.wrapping_add(palette_shift), 0.0)
}

/// Match `getCurveColor(id, offset)` from the original graph, including channel offsets.
pub fn get_curve_color_at(id: usize, offset: f64) -> Color32 {
    let id = id as f64;
    let t = offset / 32.0;
    let i = id / LD_P3 + 0.76;
    let j = id / LD_P2 + 0.79;
    let k = id / LD_P1 + 0.32;
    let angle = 2.0 * std::f64::consts::PI * i;
    let hue_component = i + (angle - 0.3).cos() / 24.0 + (6.0 * angle).cos() / 32.0;
    let sine = (2.0 * std::f64::consts::PI * hue_component).sin();
    let hue = 360.0 * ((hue_component + t / LD_P2) % 1.0);
    let chroma = 88.0 + 30.0 * (j % 1.0 + 1.3 * sine - t / LD_P3);
    let lightness = 36.0 + 22.0 * (k % 1.0 + 1.1 * sine + 6.0 * t * (1.0 - sine));
    hcl_to_rgb(hue, chroma, lightness)
}

/// Targets use CrinGraph's dedicated low-chroma HCL branch.
pub fn target_color() -> Color32 {
    let id = -1.0_f64;
    let i = id / LD_P3 + 0.76;
    let hue = 360.0 * (1.0 - (-i % 1.0));
    hcl_to_rgb(hue, 5.0, 66.0)
}

fn hcl_to_rgb(hue: f64, chroma: f64, lightness: f64) -> Color32 {
    const XN: f64 = 0.96422;
    const YN: f64 = 1.0;
    const ZN: f64 = 0.82521;
    const T0: f64 = 4.0 / 29.0;
    const T1: f64 = 6.0 / 29.0;
    const T2: f64 = 3.0 * T1 * T1;
    let hue = hue.to_radians();
    let a = hue.cos() * chroma;
    let b = hue.sin() * chroma;
    let y = (lightness + 16.0) / 116.0;
    let x = y + a / 500.0;
    let z = y - b / 200.0;
    let lab_to_xyz = |value: f64| {
        if value > T1 {
            value * value * value
        } else {
            T2 * (value - T0)
        }
    };
    let x = XN * lab_to_xyz(x);
    let y = YN * lab_to_xyz(y);
    let z = ZN * lab_to_xyz(z);
    let linear_to_srgb = |value: f64| {
        255.0
            * if value <= 0.0031308 {
                12.92 * value
            } else {
                1.055 * value.powf(1.0 / 2.4) - 0.055
            }
    };
    let channels = [
        linear_to_srgb(3.1338561 * x - 1.6168667 * y - 0.4906146 * z),
        linear_to_srgb(-0.9787684 * x + 1.9161415 * y + 0.0334540 * z),
        linear_to_srgb(0.0719453 * x - 0.2289914 * y + 1.4052427 * z),
    ];
    Color32::from_rgb(
        clamp_channel(channels[0]),
        clamp_channel(channels[1]),
        clamp_channel(channels[2]),
    )
}

fn clamp_channel(channel: f64) -> u8 {
    channel.round().clamp(0.0, 255.0) as u8
}

#[cfg(test)]
mod tests {
    use super::{get_curve_color, get_curve_color_at, get_curve_color_with_offset, target_color};

    #[test]
    fn palette_colors_are_stable_and_opaque() {
        let first = get_curve_color(0);
        assert_eq!(first, get_curve_color(0));
        assert_eq!(first.a(), 255);
        assert_ne!(first, get_curve_color(1));
    }

    #[test]
    fn recolor_offsets_shift_the_palette_consistently() {
        assert_eq!(get_curve_color_with_offset(0, 0), get_curve_color(0));
        assert_ne!(
            get_curve_color_with_offset(0, 1),
            get_curve_color_with_offset(0, 0)
        );
        assert_eq!(get_curve_color_with_offset(0, 1), get_curve_color(1));
    }

    #[test]
    fn curve_colors_match_original_d3_hcl_palette_samples() {
        assert_eq!(get_curve_color(0), egui::Color32::from_rgb(0, 58, 157));
        assert_eq!(get_curve_color(1), egui::Color32::from_rgb(0, 172, 0));
        assert_ne!(get_curve_color_at(0, -1.0), get_curve_color_at(0, 1.0));
    }

    #[test]
    fn target_color_uses_low_chroma_hcl_conversion() {
        assert_eq!(target_color(), egui::Color32::from_rgb(169, 158, 154));
    }
}
