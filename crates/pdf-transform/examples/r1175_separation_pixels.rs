//! What choosing one definition of an ink over another does to the page, on the corpus witnesses.
//!
//! `doc/adr/1188` section 3's measurement. ISO 32000-2 §8.6.6.4 says an additive device "never
//! applies a process colourant directly; it always reverts to the alternate colour space", so the
//! choice is expected to be visible; this says by how much. Scratch instrument, not a gate.

#![expect(
    clippy::expect_used,
    clippy::print_stdout,
    reason = "a scratch measurement whose whole product is a number a person reads"
)]

use std::path::Path;

use pdf_archive::{Flavour, Target};
use pdf_syntax::{Document, Limits};
use pdf_transform::archive::{ArchivePlan, Authorisations, Configuration};
use pdf_transform::tool::ToolOutputs;
use pdf_transform::{Budget, MemorySinks, Plan, Policy, Source, apply};

/// Converts `bytes` with the separation site answered by `winner`.
fn convert(bytes: &[u8], target: Target, winner: &str) -> Option<Vec<u8>> {
    let text = format!(
        "[site.\"graphics/separations-of-one-name-agree\"]\nremedy = \"supply\"\n\
         winner = \"{winner}\"\n"
    );
    let config = Configuration::read(&text, target).expect("the configuration reads");
    let mut authorised = Authorisations::default();
    for loss in pdf_transform::archive::Loss::ALL {
        authorised.authorise(loss);
    }
    let sinks = MemorySinks::new();
    apply(
        &Plan::Archive(ArchivePlan {
            source: 0,
            names: "out.pdf".parse().expect("a pattern"),
            target,
            authorised,
            profile: None,
            substitute_fonts: true,
            departures: Vec::new(),
            claim_conformance: false,
            derivations: Vec::new(),
            supplies: config.supplies(target),
            preservations: Vec::new(),
            tool_outputs: ToolOutputs::new(),
        }),
        &[Source::new(bytes.to_vec())],
        &sinks,
        &Policy::default(),
        &Budget::default(),
    )
    .expect("the conversion applies");
    sinks.into_outputs().pop().map(|(_, bytes)| bytes)
}

/// Page one of `bytes`, drawn by the correctness oracle at three times nominal size.
fn page_one(bytes: Vec<u8>) -> pdf_render::Raster {
    let document = Document::open_with_limits(bytes, Limits::DEFAULT).expect("it opens");
    let pages = pdf_model::Pages::new(&document);
    let page = pages.get(0).expect("a first page");
    let view = pdf_model::view::ViewState::of(&document);
    let list = pdf_model::content::interpret_with(&document, &page, &view).display_list;
    let target = pdf_render::TargetSpec::for_page(&list, 3.0, 1 << 32).expect("a target");
    <render_cpu::CpuRasterizer as pdf_render::Rasterizer>::rasterize(
        &mut render_cpu::CpuRasterizer::new(),
        &list,
        target,
    )
    .expect("drawn")
}

fn main() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../doc/veraPDF-corpus");
    for (part, target) in [
        ("PDF_A-2b", Target::Two(pdf_archive::Level::B)),
        ("PDF_A-4", Target::Four(Flavour::Plain)),
    ] {
        let dir = root.join(part).join("6.2 Graphics");
        let mut paths: Vec<_> = walk(&dir);
        paths.sort();
        for path in paths {
            let Ok(bytes) = std::fs::read(&path) else {
                continue;
            };
            let Ok(document) = Document::open_with_limits(bytes.clone(), Limits::DEFAULT) else {
                continue;
            };
            if !pdf_archive::check(&document, target)
                .failures()
                .any(|judgement| judgement.id == "graphics/separations-of-one-name-agree")
            {
                continue;
            }
            drop(document);
            let (Some(first), Some(most)) = (
                convert(&bytes, target, "first"),
                convert(&bytes, target, "most-used"),
            ) else {
                println!("{}: one of the two words wrote no file", path.display());
                continue;
            };
            let identical = first == most;
            let source_page = page_one(bytes.clone());
            let first_page = page_one(first);
            let most_page = page_one(most);
            let comparison =
                raster_compare::compare(&first_page, &most_page).expect("comparable rasters");
            let against_source =
                raster_compare::compare(&source_page, &first_page).expect("comparable rasters");
            println!(
                "{target} {}: bytes identical {identical}; {} of {} pixel(s) are not white; \
                 first-vs-most mean {:.4} worst-tile {:.4} differing {:.6}; \
                 source-vs-first mean {:.4} worst-tile {:.4} max {} differing {:.6}",
                path.file_name().expect("a name").to_string_lossy(),
                inked(&source_page),
                u64::from(source_page.width).saturating_mul(u64::from(source_page.height)),
                comparison.mean_error,
                comparison.worst_tile_error,
                comparison.differing_fraction,
                against_source.mean_error,
                against_source.worst_tile_error,
                against_source.max_error,
                against_source.differing_fraction,
            );
        }
    }
}

/// How many of a raster's pixels are not opaque white, so that a blank page is not read as
/// agreement.
fn inked(raster: &pdf_render::Raster) -> u64 {
    raster
        .data
        .chunks_exact(4)
        .filter(|pixel| pixel != &[0xff, 0xff, 0xff, 0xff])
        .count() as u64
}

/// Every PDF under `dir`.
fn walk(dir: &Path) -> Vec<std::path::PathBuf> {
    let mut out = Vec::new();
    let Ok(entries) = std::fs::read_dir(dir) else {
        return out;
    };
    for entry in entries.flatten() {
        let path = entry.path();
        if path.is_dir() {
            out.extend(walk(&path));
        } else if path
            .extension()
            .is_some_and(|extension| extension.eq_ignore_ascii_case("pdf"))
        {
            out.push(path);
        }
    }
    out
}
