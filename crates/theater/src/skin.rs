//! PNG-based skin pack loader.
//!
//! A skin PNG is any image whose dominant colours are extracted to form a
//! VaultTheme palette.  The convention (from the VaultForge design doc):
//!   - pixel (0,0)   → `primary`   colour
//!   - pixel (1,0)   → `secondary` colour
//!   - pixel (2,0)   → `background` colour
//!   - pixel (3,0)   → `accent`    colour
//!   - pixel (4,0)   → `text`      colour
//!
//! If the image is smaller than 5 pixels wide the loader falls back to
//! dominant-colour extraction via a simple 5-bucket mean.

use crate::theme::{ThemePalette, VaultTheme};
use anyhow::{Context, Result};
use image::{GenericImageView, Rgba};
use std::path::Path;

fn rgba_to_hex(px: Rgba<u8>) -> String {
    format!("#{:02X}{:02X}{:02X}", px[0], px[1], px[2])
}

/// Extract a palette from a PNG file.
pub fn load_skin(path: &Path) -> Result<VaultTheme> {
    let img = image::open(path)
        .with_context(|| format!("opening skin PNG {}", path.display()))?;

    let (width, _height) = img.dimensions();

    let palette = if width >= 5 {
        // Convention: read the first 5 pixels of the top row.
        let primary   = rgba_to_hex(img.get_pixel(0, 0));
        let secondary = rgba_to_hex(img.get_pixel(1, 0));
        let bg        = rgba_to_hex(img.get_pixel(2, 0));
        let accent    = rgba_to_hex(img.get_pixel(3, 0));
        let text      = rgba_to_hex(img.get_pixel(4, 0));
        ThemePalette {
            surface: bg.clone(),
            border:  secondary.clone(),
            glow:    format!("{}60", accent.trim_start_matches('#')),
            bg, primary, secondary, accent, text,
        }
    } else {
        // Fallback: compute mean colour per quintic bucket across all pixels.
        let pixels: Vec<Rgba<u8>> = img.pixels().map(|(_, _, p)| p).collect();
        let total = pixels.len();
        if total == 0 {
            anyhow::bail!("skin PNG has no pixels");
        }
        let chunk = (total / 5).max(1);
        let buckets: Vec<Rgba<u8>> = (0..5)
            .map(|i| {
                let slice = &pixels[(i * chunk).min(total)..((i + 1) * chunk).min(total)];
                if slice.is_empty() {
                    return Rgba([128, 128, 128, 255]);
                }
                let (r, g, b) = slice.iter().fold((0u64, 0u64, 0u64), |(ra, ga, ba), px| {
                    (ra + px[0] as u64, ga + px[1] as u64, ba + px[2] as u64)
                });
                let n = slice.len() as u64;
                Rgba([(r / n) as u8, (g / n) as u8, (b / n) as u8, 255])
            })
            .collect();

        let primary   = rgba_to_hex(buckets[0]);
        let secondary = rgba_to_hex(buckets[1]);
        let bg        = rgba_to_hex(buckets[2]);
        let accent    = rgba_to_hex(buckets[3]);
        let text      = rgba_to_hex(buckets[4]);
        ThemePalette {
            surface: bg.clone(),
            border:  secondary.clone(),
            glow:    format!("{}60", accent.trim_start_matches('#')),
            bg, primary, secondary, accent, text,
        }
    };

    Ok(VaultTheme {
        faction: None,
        palette,
        skin_png_path: Some(path.to_string_lossy().into_owned()),
        theme_index: 0,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use image::{ImageBuffer, RgbaImage};
    use std::env::temp_dir;

    fn write_test_png(colors: &[(u8, u8, u8)]) -> std::path::PathBuf {
        let width = colors.len() as u32;
        let mut img: RgbaImage = ImageBuffer::new(width.max(1), 1);
        for (x, &(r, g, b)) in colors.iter().enumerate() {
            img.put_pixel(x as u32, 0, image::Rgba([r, g, b, 255]));
        }
        let path = temp_dir().join(format!("test-skin-{}.png", width));
        img.save(&path).unwrap();
        path
    }

    #[test]
    fn loads_5pixel_skin() {
        let path = write_test_png(&[
            (255, 0, 0),
            (0, 255, 0),
            (0, 0, 255),
            (255, 255, 0),
            (255, 255, 255),
        ]);
        let theme = load_skin(&path).unwrap();
        assert_eq!(theme.palette.primary, "#FF0000");
        assert_eq!(theme.palette.secondary, "#00FF00");
        assert_eq!(theme.palette.bg, "#0000FF");
        assert_eq!(theme.palette.accent, "#FFFF00");
        assert_eq!(theme.palette.text, "#FFFFFF");
        assert!(theme.skin_png_path.is_some());
        std::fs::remove_file(&path).ok();
    }

    #[test]
    fn fallback_for_narrow_skin() {
        // 2-pixel image — triggers bucket fallback
        let path = write_test_png(&[(200, 100, 50), (100, 200, 150)]);
        let theme = load_skin(&path).unwrap();
        // Just ensure it doesn't panic and returns something non-empty
        assert!(!theme.palette.primary.is_empty());
        std::fs::remove_file(&path).ok();
    }
}
