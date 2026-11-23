#[cfg(all(target_os = "linux", target_arch = "x86_64"))]
use std::fmt::Write as _;

#[cfg(all(target_os = "linux", target_arch = "x86_64"))]
use crate::RgbImage;

#[cfg(all(target_os = "linux", target_arch = "x86_64"))]
pub(super) fn format_rgb_3dp(image: &RgbImage<f32>) -> String {
    let mut output = String::new();
    writeln!(output, "[").unwrap();
    for y in 0..image.height() {
        write!(output, "  [").unwrap();
        for x in 0..image.width() {
            let [red, green, blue] = image.get_pixel(x, y).0;
            if x > 0 {
                write!(output, ", ").unwrap();
            }
            write!(output, "[{red:.3}, {green:.3}, {blue:.3}]").unwrap();
        }
        writeln!(output, "],").unwrap();
    }
    writeln!(output, "]").unwrap();
    output
}
