//! A page drawn whole onto an opaque medium is put on it by the device, and the bytes are the
//! ones the processor's three passes made (ADR 2150).
//!
//! `QuorraRasterizer::rasterize` premultiplied the straight raster raster handed back, imposed
//! §11.4.7's medium with [`pdf_render::impose_on_medium`] and demultiplied again; for a page that
//! owes no pass over its own components it now asks the device for the page on its medium, and
//! converts nothing. This holds the two to the byte on real pages: the same page through a
//! second backend whose medium is [`Medium::NONE`] — which hands raster's straight bytes back
//! untouched where nothing crops the page — put through the processor's passes here, against the
//! page on white.

#![expect(
    clippy::expect_used,
    clippy::panic,
    clippy::arithmetic_side_effects,
    reason = "test code: an explanatory panic is the intended failure mode, and the two \
              transcriptions multiply bytes, whose products a `u16` holds"
)]

use std::path::Path;

use pdf_render::{Color, DisplayList, Medium, Rasterizer as _, TargetSpec};
use pdf_syntax::Document;
use render_raster::QuorraRasterizer;

/// `render_raster`'s premultiply, transcribed: the processor's first pass.
fn premultiply(data: &mut [u8]) {
    for pixel in data.chunks_exact_mut(4) {
        let alpha = u16::from(pixel[3]);
        if alpha == 255 {
            continue;
        }
        for channel in &mut pixel[..3] {
            let scaled = (u16::from(*channel) * alpha + 127) / 255;
            *channel = u8::try_from(scaled).expect("a product of two bytes over 255 is a byte");
        }
    }
}

/// `render_raster`'s demultiply, transcribed: the processor's last pass.
fn demultiply(data: &mut [u8]) {
    for pixel in data.chunks_exact_mut(4) {
        let alpha = u16::from(pixel[3]);
        if alpha == 0 || alpha == 255 {
            continue;
        }
        for channel in &mut pixel[..3] {
            *channel = u8::try_from(((u16::from(*channel) * 255 + alpha / 2) / alpha).min(255))
                .expect("clamped to a byte");
        }
    }
}

fn pages(file: &str, indices: &[usize]) -> Vec<DisplayList> {
    let path = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../doc")
        .join(file);
    let document =
        Document::open(std::fs::read(&path).expect("the document is in doc/")).expect("opens");
    let pages = pdf_model::Pages::new(&document);
    indices
        .iter()
        .map(|&index| {
            let page = pages.get(index).expect("that page exists");
            pdf_model::content::interpret(&document, &page).display_list
        })
        .collect()
}

fn backend(medium: Medium) -> QuorraRasterizer {
    QuorraRasterizer::new_headless()
        .unwrap_or_else(|error| panic!("no adapter for raster: {error} — these tests do not skip"))
        .with_medium(medium)
}

#[test]
fn a_page_on_white_is_the_processor_s_three_passes_to_the_byte() {
    let mut on_white = backend(Medium::PAGE_ONLY);
    let mut on_nothing = backend(Medium::NONE);
    let mut lists = pages("PDF20_AN001-BPC.pdf", &[0, 1]);
    lists.extend(pages("ISO_32000-2_sponsored_EC3.pdf", &[0, 6]));
    let mut first = true;
    let mut compared = 0;
    for list in &lists {
        // 1.04 is the width a toolkit window asks for, and not a whole number of pixels per point.
        for scale in [1.0, 619.0 / list.page_size.width, 2.0] {
            let target = TargetSpec::for_page(list, scale, 1 << 28).expect("a target");
            // Where §14.11.2.1's clip cuts the target the medium-less raster has been cropped
            // and converted already, so it is not raster's own bytes; `raster-gpu`'s
            // `a_backdrop_goes_under_on_the_device` holds the crop on the device instead.
            if pdf_render::crop_area(list, target).is_some() {
                continue;
            }
            let drawn = on_white.rasterize(list, target).expect("draws").data;
            if first {
                assert!(
                    on_white
                        .last_phases()
                        .iter()
                        .any(|(name, _)| *name == "pipeline compile (first use)"),
                    "the first page compiled the device's pass, so it was the device that drew \
                     the medium: {:?}",
                    on_white.last_phases()
                );
                first = false;
            }
            let mut expected = on_nothing.rasterize(list, target).expect("draws").data;
            premultiply(&mut expected);
            pdf_render::impose_on_medium(&mut expected, Color::WHITE);
            demultiply(&mut expected);
            let differing = drawn
                .chunks_exact(4)
                .zip(expected.chunks_exact(4))
                .filter(|(a, b)| a != b)
                .count();
            assert_eq!(
                differing, 0,
                "{} × {} at {scale}: the device's medium differs from the processor's",
                target.width, target.height
            );
            compared += 1;
        }
    }
    assert!(compared >= 8, "only {compared} targets were compared");
}
