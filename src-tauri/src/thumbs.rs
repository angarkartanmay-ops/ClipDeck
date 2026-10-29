// Copyright (c) 2026 Tanmay Angarkar. Licensed under PolyForm Noncommercial 1.0.0.

//! Thumbnail generation for image items (runs on the capture worker).

use std::io::Cursor;

use image::{DynamicImage, ImageFormat};

use crate::clip::{ClipData, DIB, PNG};

/// Large enough to stay sharp on full-screen tiles at 150% scaling.
const MAX_W: u32 = 960;
const MAX_H: u32 = 640;
/// Quick Look previews are capped so a huge capture stays responsive.
const PREVIEW_MAX: u32 = 2560;

fn decode(data: &ClipData) -> Option<DynamicImage> {
    data.get(PNG)
        .and_then(|png| image::load_from_memory_with_format(png, ImageFormat::Png).ok())
        .or_else(|| data.get(DIB).and_then(decode_dib))
}

pub fn make(data: &ClipData) -> Option<Vec<u8>> {
    encode(decode(data)?.thumbnail(MAX_W, MAX_H))
}

/// Full-size PNG for the Quick Look preview. A copied PNG is returned as is.
pub fn preview(data: &ClipData) -> Option<Vec<u8>> {
    if let Some(png) = data.get(PNG) {
        return Some(png.to_vec());
    }
    let img = decode(data)?;
    if img.width() > PREVIEW_MAX || img.height() > PREVIEW_MAX {
        encode(img.thumbnail(PREVIEW_MAX, PREVIEW_MAX))
    } else {
        encode(img)
    }
}

fn encode(img: DynamicImage) -> Option<Vec<u8>> {
    let mut rgba = img.to_rgba8();
    // Many apps put 32-bit DIBs with an unused (zero) alpha channel on the
    // clipboard; treat a fully transparent image as opaque.
    if rgba.pixels().all(|p| p[3] == 0) {
        rgba.pixels_mut().for_each(|p| p[3] = 255);
    }
    let mut out = Cursor::new(Vec::new());
    DynamicImage::ImageRgba8(rgba)
        .write_to(&mut out, ImageFormat::Png)
        .ok()?;
    Some(out.into_inner())
}

/// Decodes a packed DIB (CF_DIB) by prefixing the BMP file header.
fn decode_dib(dib: &[u8]) -> Option<DynamicImage> {
    if dib.len() < 40 {
        return None;
    }
    let header_size = u32::from_le_bytes(dib[0..4].try_into().ok()?) as usize;
    let bit_count = u16::from_le_bytes(dib[14..16].try_into().ok()?) as usize;
    let compression = u32::from_le_bytes(dib[16..20].try_into().ok()?);
    let colors_used = u32::from_le_bytes(dib[32..36].try_into().ok()?) as usize;
    let masks = match (header_size, compression) {
        (40, 3) => 12, // BI_BITFIELDS
        (40, 6) => 16, // BI_ALPHABITFIELDS
        _ => 0,
    };
    let palette = if bit_count <= 8 {
        4 * if colors_used > 0 {
            colors_used
        } else {
            1 << bit_count
        }
    } else {
        4 * colors_used
    };
    let offset = 14 + header_size + masks + palette;
    let file_size = 14 + dib.len();

    let mut bmp = Vec::with_capacity(file_size);
    bmp.extend_from_slice(b"BM");
    bmp.extend_from_slice(&(file_size as u32).to_le_bytes());
    bmp.extend_from_slice(&[0, 0, 0, 0]);
    bmp.extend_from_slice(&(offset as u32).to_le_bytes());
    bmp.extend_from_slice(dib);
    image::load_from_memory_with_format(&bmp, ImageFormat::Bmp).ok()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::clip::ClipFormat;

    #[test]
    fn thumbnail_from_dib() {
        // 2x2 24-bit bottom-up DIB.
        let mut dib = Vec::new();
        dib.extend_from_slice(&40u32.to_le_bytes());
        dib.extend_from_slice(&2i32.to_le_bytes());
        dib.extend_from_slice(&2i32.to_le_bytes());
        dib.extend_from_slice(&1u16.to_le_bytes());
        dib.extend_from_slice(&24u16.to_le_bytes());
        dib.extend_from_slice(&[0u8; 24]);
        for _ in 0..2 {
            dib.extend_from_slice(&[255, 0, 0, 0, 255, 0, 0, 0]); // 2 px + 2 bytes padding
        }
        let data = ClipData {
            formats: vec![ClipFormat {
                name: DIB.into(),
                data: dib,
            }],
        };
        let png = make(&data).expect("thumbnail");
        assert!(png.starts_with(b"\x89PNG"));
    }
}
