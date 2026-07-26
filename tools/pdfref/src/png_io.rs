//! PNG decoding and encoding for the harness.
//!
//! Reference renderers all emit PNG, but not the same PNG: `pdftoppm`, `mutool draw`
//! and `gs -sDEVICE=png16m` all write 8-bit RGB, while other tools and our own
//! artefacts use RGBA. Everything is normalised to [`RasterFormat::Rgba8`] on the way
//! in, so the comparison code never branches on pixel layout.

use std::path::Path;

use pdf_render::{Raster, RasterFormat};

use crate::HarnessError;

/// Reads a PNG and normalises it to straight-alpha RGBA8.
///
/// # Errors
///
/// [`HarnessError::Png`] if the file cannot be read or decoded, and
/// [`HarnessError::UnsupportedPng`] for a colour type or bit depth the harness does not
/// handle. Unsupported input is refused rather than approximated: a silently mangled
/// reference image would produce a difference that looks like our bug.
#[expect(
    clippy::arithmetic_side_effects,
    reason = "capacity hints derived from a buffer size the decoder already validated"
)]
pub fn read(path: &Path) -> Result<Raster, HarnessError> {
    let file = std::fs::File::open(path).map_err(|e| HarnessError::Png {
        path: path.into(),
        message: e.to_string(),
    })?;
    let decoder = png::Decoder::new(std::io::BufReader::new(file));
    let mut reader = decoder.read_info().map_err(|e| HarnessError::Png {
        path: path.into(),
        message: e.to_string(),
    })?;

    let mut buffer = vec![0; reader.output_buffer_size().unwrap_or(0)];
    let info = reader
        .next_frame(&mut buffer)
        .map_err(|e| HarnessError::Png {
            path: path.into(),
            message: e.to_string(),
        })?;

    if info.bit_depth != png::BitDepth::Eight {
        return Err(HarnessError::UnsupportedPng {
            path: path.into(),
            detail: format!("{:?} bit depth", info.bit_depth),
        });
    }

    let pixels = info.buffer_size();
    let data = match info.color_type {
        png::ColorType::Rgba => buffer[..pixels].to_vec(),
        png::ColorType::Rgb => {
            let mut rgba = Vec::with_capacity(pixels / 3 * 4);
            for chunk in buffer[..pixels].chunks_exact(3) {
                rgba.extend_from_slice(chunk);
                // Reference renderers composite onto an opaque page, so a missing alpha
                // channel means fully opaque rather than unknown.
                rgba.push(u8::MAX);
            }
            rgba
        }
        png::ColorType::Grayscale => {
            let mut rgba = Vec::with_capacity(pixels * 4);
            for &grey in &buffer[..pixels] {
                rgba.extend_from_slice(&[grey, grey, grey, u8::MAX]);
            }
            rgba
        }
        other => {
            return Err(HarnessError::UnsupportedPng {
                path: path.into(),
                detail: format!("{other:?} colour type"),
            });
        }
    };

    Ok(Raster {
        width: info.width,
        height: info.height,
        format: RasterFormat::Rgba8,
        data,
    })
}

/// Writes a raster as an RGBA8 PNG.
///
/// # Errors
///
/// [`HarnessError::Png`] if the file cannot be created or encoded.
pub fn write(path: &Path, raster: &Raster) -> Result<(), HarnessError> {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).map_err(|e| HarnessError::Png {
            path: path.into(),
            message: e.to_string(),
        })?;
    }
    let file = std::fs::File::create(path).map_err(|e| HarnessError::Png {
        path: path.into(),
        message: e.to_string(),
    })?;

    let mut encoder = png::Encoder::new(std::io::BufWriter::new(file), raster.width, raster.height);
    encoder.set_color(png::ColorType::Rgba);
    encoder.set_depth(png::BitDepth::Eight);
    encoder
        .write_header()
        .and_then(|mut writer| writer.write_image_data(&raster.data))
        .map_err(|e| HarnessError::Png {
            path: path.into(),
            message: e.to_string(),
        })
}
