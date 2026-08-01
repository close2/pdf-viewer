//! Real pages through the GPU backend, against the CPU backend, at more than one resolution.
//!
//! # The gap this closes
//!
//! `headless_gpu.rs` compares the two backends over `test-scenes`' fixtures: a gradient, a
//! knockout group, sixteen blend modes. Every one of them is a handful of commands, and every one
//! is rendered at one modest size. **A page of text at a high resolution is neither**, and that
//! turns out to be the shape that breaks: Vello sizes its GPU working buffers from a table of
//! constants — its own comment calls them "hand picked to accommodate the vello test scenes" —
//! and a scene needing more overflows them *on the device*, which sets a flag, stops filling, and
//! returns a blank target with no error at all.
//!
//! Page 6 of ISO 32000-2 at 1132×1600 was exactly that on an AMD 890M: 5933 paths over 7100
//! tiles, every pixel empty, and a viewer showing a black page with nothing in the log. It was
//! reported by the person using the program, because nothing here could see it (ADR 0127).
//!
//! # What is asserted
//!
//! **A backend may refuse a scene. It may not silently draw nothing.** So for each page and each
//! scale: either the GPU errors — which a caller can act on, and `viewer-ui` does, by drawing the
//! page with `render-cpu` — or it produces ink within a few per cent of what the CPU produced.
//! The comparison is *total ink* rather than per-pixel, because the two rasterisers legitimately
//! differ at every glyph edge and this test is not about that; it is about the difference between
//! a page and nothing.

#![expect(
    clippy::arithmetic_side_effects,
    clippy::cast_precision_loss,
    reason = "test code, summing and dividing pixel counts of rasters this test itself sized"
)]

use std::path::Path;

use pdf_render::{Raster, Rasterizer, TargetSpec};
use pdf_syntax::Document;

/// The total darkness of a raster, which is what "did anything get drawn" means here.
fn ink(raster: &Raster) -> u64 {
    raster
        .data
        .chunks_exact(4)
        .map(|pixel| u64::from(255 - pixel[0].min(pixel[1]).min(pixel[2])))
        .sum()
}

#[test]
fn a_page_is_drawn_or_refused_and_never_silently_blank() {
    let Ok(mut gpu) = render_gpu::GpuRasterizer::new_headless() else {
        println!("skipped: no GPU adapter, not even a software one");
        return;
    };
    println!("adapter: {}", gpu.context().adapter_description());

    // The specification's own PDFs, which every checkout has. Page 6 of ISO 32000-2 is the
    // witness from the field; the others are ordinary pages, there so that a device which
    // *cannot* draw any of this is distinguishable from one that trips on the dense one.
    let cases: [(&str, usize); 4] = [
        ("ISO_32000-2_sponsored_EC3.pdf", 5),
        ("ISO_32000-2_sponsored_EC3.pdf", 6),
        ("ISO_32000-2_sponsored_EC3.pdf", 4),
        ("PDF20_AN001-BPC.pdf", 0),
    ];

    let mut refused = 0;
    let mut banded = 0;
    for (name, index) in cases {
        let path = Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../../doc")
            .join(name);
        let bytes =
            std::fs::read(&path).unwrap_or_else(|e| panic!("{} is committed: {e}", path.display()));
        let document = Document::open(bytes).expect("it opens");
        let pages = pdf_model::Pages::new(&document);
        let page = pages.get(index).expect("the page exists");
        let list = pdf_model::content::interpret(&document, &page).display_list;

        // 1.0 is what the reference comparison uses; 1.9008 is a 1023-page specification fitted
        // to a 1280×1600 window, which is a 16-inch display at its usual scale factor. The
        // second is the one that was blank.
        for scale in [1.0_f32, 1.9008] {
            let target = TargetSpec::for_page(&list, scale, 1 << 28).expect("a target");
            let expected = ink(&render_cpu::CpuRasterizer::new()
                .rasterize(&list, target)
                .expect("the CPU backend draws every page"));
            match gpu.rasterize(&list, target) {
                Err(refusal) => {
                    // A refusal is a legitimate answer and the caller acts on it. What it may
                    // not be is silent, which is what this whole file is about.
                    println!(
                        "  {name} page {} at {scale}: refused — {refusal}",
                        index + 1
                    );
                    refused += 1;
                }
                Ok(raster) => {
                    let drawn = ink(&raster);
                    if gpu.bands().count() > 1 {
                        println!(
                            "  {name} page {} at {scale}: drawn in {} bands",
                            index + 1,
                            gpu.bands().count()
                        );
                        banded += 1;
                    }
                    assert!(
                        drawn > 0 || expected == 0,
                        "{name} page {} at {scale} ({}x{}): the device drew *nothing* and said \
                         nothing, where the processor drew {expected}",
                        index + 1,
                        target.width,
                        target.height,
                    );
                    let ratio = drawn as f64 / expected.max(1) as f64;
                    assert!(
                        (0.9..1.1).contains(&ratio),
                        "{name} page {} at {scale}: the device drew {drawn} against the \
                         processor's {expected}, a ratio of {ratio:.3}",
                        index + 1
                    );
                }
            }
        }
    }
    println!("{refused} of 8 renders refused, {banded} drawn in bands");

    // **The witness has to be drawn, and drawn the hard way.** Page 6 at 1.9008 is what the
    // person reported; it overflows Vello's tile buffer in one pass, so if it comes back both
    // unrefused and unbanded, something has changed underneath this test and the thing it was
    // written to check is no longer being checked.
    assert!(
        banded > 0,
        "no render needed banding — the scene that overflows the device's buffers is no longer \
         reaching it, so this test is no longer testing what it was written for"
    );
    assert_eq!(refused, 0, "every one of these pages can be drawn in bands");
}
