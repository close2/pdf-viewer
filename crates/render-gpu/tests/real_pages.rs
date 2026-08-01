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

    assert_eq!(refused, 0, "every one of these pages can be drawn in bands");

    // **Page 6 stopped being the witness in the hundred-and-forty-seventh session, and it is
    // worth saying why rather than quietly dropping the assertion.** This test used to require
    // `banded > 0` here, on the grounds that a render coming back both unrefused and unbanded
    // meant something had changed underneath it. Something did: `DisplayList::add_clip` began
    // handing back an existing identifier for an identical region, and page 6's 303 clips —
    // one region, stated 303 times — became one. It no longer overflows Vello's buffers at
    // 1.9008, or at 5.0 (measured). The banding is still correct and still needed, because the
    // constants it works around are fixed and another document can still exceed them; what is
    // gone is a *real page* that does. `a_scene_too_large_for_one_pass_is_banded` below is the
    // replacement, and it states the shape rather than borrowing one. ADR 0132.
}

/// A scene the device cannot draw in one pass is drawn in bands, not left blank.
///
/// The synthetic witness for ADR 0127's banding, written after the real one stopped overflowing.
/// Its shape is not invented: it is page 6 of ISO 32000-2 with each fill given a clip of its own,
/// which is the scene page 6 *was* until identical clip regions began sharing an identifier. What
/// the nudge stands in for is a producer whose per-run clip rectangle differs in the last decimal
/// place, which deduplication cannot collapse and which a device therefore still has to band.
#[test]
fn a_scene_too_large_for_one_pass_is_banded() {
    let Ok(mut gpu) = render_gpu::GpuRasterizer::new_headless() else {
        println!("skipped: no GPU adapter, not even a software one");
        return;
    };

    let path =
        Path::new(env!("CARGO_MANIFEST_DIR")).join("../../doc/ISO_32000-2_sponsored_EC3.pdf");
    let bytes = std::fs::read(&path).expect("the specification is committed");
    let document = Document::open(bytes).expect("it opens");
    let pages = pdf_model::Pages::new(&document);
    let page = pages.get(5).expect("page 6 exists");
    let source = pdf_model::content::interpret(&document, &page).display_list;

    let mut list = pdf_render::DisplayList::new(source.page_size);
    let bounds = list.page_bounds();
    // Two thousand is enough to overflow and keeps the test's own cost down; the whole page
    // bands the same way and takes a minute of device time to do it.
    for (index, command) in source.commands().iter().take(2000).enumerate() {
        let pdf_render::Command::Fill {
            path,
            transform,
            fill_rule,
            paint,
            blend,
            ..
        } = command
        else {
            continue;
        };
        // Each clip is the page, nudged by a ten-thousandth of a point per command, so no two
        // are equal and none changes what is drawn.
        #[expect(
            clippy::cast_precision_loss,
            reason = "an index over one page's commands, bounded by thousands"
        )]
        let nudge = index as f32 * 0.0001;
        let mut region = pdf_render::Path::new();
        region.push(pdf_render::PathCommand::MoveTo(pdf_render::Point::new(
            nudge, nudge,
        )));
        region.push(pdf_render::PathCommand::LineTo(pdf_render::Point::new(
            bounds.max.x,
            nudge,
        )));
        region.push(pdf_render::PathCommand::LineTo(pdf_render::Point::new(
            bounds.max.x,
            bounds.max.y,
        )));
        region.push(pdf_render::PathCommand::LineTo(pdf_render::Point::new(
            nudge,
            bounds.max.y,
        )));
        region.push(pdf_render::PathCommand::Close);
        let clip = list
            .add_clip(pdf_render::Clip {
                path: region,
                transform: pdf_render::Transform::IDENTITY,
                fill_rule: pdf_render::FillRule::NonZero,
                parent: None,
            })
            .expect("one clip per command is far under the bound");
        list.push(pdf_render::Command::Fill {
            path: std::sync::Arc::clone(path),
            transform: *transform,
            fill_rule: *fill_rule,
            paint: paint.clone(),
            clip: Some(clip),
            mask: None,
            blend: *blend,
        });
    }

    let target = TargetSpec::for_page(&list, 1.9008, 1 << 28).expect("a target");
    let expected = ink(&render_cpu::CpuRasterizer::new()
        .rasterize(&list, target)
        .expect("the processor draws it"));
    let raster = gpu.rasterize(&list, target).expect("the device draws it");
    println!("drawn in {} bands", gpu.bands().count());
    assert!(
        gpu.bands().count() > 1,
        "this scene no longer overflows the device's buffers, so it is no longer the witness \
         ADR 0127's banding needs"
    );
    let drawn = ink(&raster);
    let ratio = drawn as f64 / expected.max(1) as f64;
    assert!(
        (0.9..1.1).contains(&ratio),
        "banded, the device drew {drawn} against the processor's {expected}, a ratio of {ratio:.3}"
    );
}
