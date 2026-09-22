//! A Rust path a doc comment names is a path this tree declares.
//!
//! Not a conformance question; it lives here for `state_sections.rs`'s reason — this is the crate
//! whose gates read the repository's own files rather than a PDF.
//!
//! # What it guards
//!
//! `Edit::ChooseFile`'s doc comment named `viewer_host::policy::may_choose_file`, which nobody had
//! written. [`conformance::names`] says why nothing in this project could see that, and what
//! resolution means here.
//!
//! # Why a named population rather than a count
//!
//! The sweep's finding is not yet zero, and a bare number that may only fall says nothing about
//! *which* path is new. [`STANDING`] names every path the tree carries today, each one a sentence
//! naming an item this workspace does not declare; a path outside that list fails, and a path on
//! it that has been fixed fails too, so the list cannot outlive its facts. Working the list down
//! is `doc/todo/01`'s, and the day it empties this becomes the zero the population deserves.
//!
//! # And the calibration, which is not optional here
//!
//! A sweep that comes back clean is a sentence about the sweep (trap 13). The second test plants
//! the defect — a path under a module of this crate that nothing declares — into the sweep's own
//! machinery and fails unless the sweep names it. It plants into the functions rather than into a
//! file, because a plant written into the tree is a plant somebody has to remember to remove.
//!
//! ADR 1273.

#![expect(
    clippy::print_stdout,
    reason = "test code: the gate prints its denominators and its findings, so a failure and a \
              clean run are read the same way"
)]

use std::collections::BTreeSet;

use conformance::names::{self, Reach};

/// Every path the tree carries whose prefix it declares and whose name it does not.
///
/// One line per finding, `file the::path`. Each is a sentence to correct — a renamed item, a name
/// a round meant to write and did not, or a library's type named without its crate — and
/// `doc/todo/01` carries the reading. Nothing may be added here to make a build pass: a new one is
/// a comment written today about code that is not there.
///
/// **No line number**, deliberately. A doc comment moves whenever anything above it is edited, so
/// a list keyed by line would fail on somebody else's unrelated change and teach the next round to
/// regenerate it rather than read it. The run prints the line; this list says which sentences are
/// known.
const STANDING: [&str; 44] = [
    "crates/pdf-archive/src/lib.rs table::REQUIREMENTS",
    "crates/pdf-archive/src/table/file_structure.rs Check::PerTarget",
    "crates/pdf-colour/src/function.rs Function::sample_extent",
    "crates/pdf-colour/src/icc.rs Profile::from_pcs_perceptual",
    "crates/pdf-font/src/loading.rs LoadedFont::program_shortfall",
    "crates/pdf-model/src/content/colour.rs GraphicsState::black_point",
    "crates/pdf-model/src/content/pattern.rs GraphicsState::black_point",
    "crates/pdf-model/src/content.rs FontCache::bind",
    "crates/pdf-model/src/content.rs GraphicsState::black_point",
    "crates/pdf-model/src/content/text.rs Interpreter::paint_path",
    "crates/pdf-model/src/image.rs content::image::transferred_image",
    "crates/pdf-model/src/image.rs pdf_sandbox::decode::shortfall_of",
    "crates/pdf-model/src/structure.rs Tree::document_language",
    "crates/pdf-model/tests/selection_geometry.rs content::glyph_quad",
    "crates/pdf-model/tests/variable_text.rs Frame::drift",
    "crates/pdf-render/src/backend.rs crate::group_cost::MAX_GROUP_BLIT_PAGES",
    "crates/pdf-signature/tests/signatures.rs Trust::NotCurrent",
    "crates/pdf-syntax/src/document.rs StreamRefusal::TooLarge",
    "crates/pdf-syntax/src/write.rs xref::read_at",
    "crates/pdf-transform/src/structure.rs Carry::keep_child",
    "crates/render-cpu/src/scan.rs BlendMode::should_pre_scale_coverage",
    "crates/render-raster/examples/bring_up.rs StartupTimings::adapter_enumeration",
    "crates/render-raster/tests/headless_quorra.rs StagedComposeReason::InsideKnockoutGroup",
    "crates/render-raster/tests/retained_frame.rs PresentFrame::page",
    "crates/viewer-confined/src/resume.rs Confined::explain",
    "crates/viewer-core/src/command.rs crate::Query::Accessibility",
    "crates/viewer-core/src/presentation.rs Open::current",
    "crates/viewer-core/src/viewer.rs Answer::Form",
    "crates/viewer-core/src/viewer.rs Open::current",
    "crates/viewer-ui/src/bin/quorra-confined/screen.rs Mark::Image",
    "crates/viewer-ui/src/bin/quorra-confined/screen.rs pdf_render::MAX_PIXELS",
    "crates/viewer-ui/src/bin/quorra/presentation.rs render_raster::PresentFrame::page",
    "crates/viewer-ui/src/bin/quorra/sidebar.rs Surface::without_a_page",
    "crates/viewer-ui/src/bin/quorra/stale.rs Settled::cost",
    "crates/viewer-ui/src/bin/quorra/timing.rs QuorraWindowRenderer::present",
    "crates/viewer-ui/src/chrome.rs Surface::without_a_page",
    "raster/crates/raster-gpu/examples/function_paint/program.rs walk::analyse",
    "raster/crates/raster-gpu/src/error/render.rs raster::accumulate_edge",
    "raster/crates/raster-gpu/tests/ceilings.rs raster::direction",
    "raster/crates/raster-gpu/tests/scale_invariance.rs raster::direction",
    "raster/crates/raster-scene/src/blend.rs Compose::Copy",
    "tools/conformance/src/cited.rs Rung::Implementing",
    "tools/conformance/src/cited.rs Rung::Mentioning",
    "tools/conformance/src/cited.rs Rung::Unclaimed",
];

#[test]
fn every_rust_path_a_doc_comment_names_is_one_this_tree_declares() {
    let root = conformance::workspace_root();
    let found = names::sweep(&root).expect("the tree's sources and the names they declare");
    print!("{}", names::report(&found));
    assert!(
        found.index.files_read > 0 && found.comment_lines > 0 && !found.mentions.is_empty(),
        "the sweep read nothing: {} file(s), {} comment line(s), {} path(s) — a clean answer over \
         an empty population is a sentence about the sweep",
        found.index.files_read,
        found.comment_lines,
        found.mentions.len(),
    );
    let standing: BTreeSet<&str> = STANDING.iter().copied().collect();
    assert_eq!(
        standing.len(),
        STANDING.len(),
        "a line of the standing list is written twice"
    );
    let today: BTreeSet<String> = found
        .on(Reach::Absent)
        .iter()
        .map(|mention| format!("{} {}", mention.path.display(), mention.named))
        .collect();
    let arrived: Vec<&String> = today
        .iter()
        .filter(|line| !standing.contains(line.as_str()))
        .collect();
    assert!(
        arrived.is_empty(),
        "a doc comment names an item this workspace does not declare, which reads exactly like a \
         reference that works. Either the item exists under that name or the sentence names one \
         that does:\n  {}",
        arrived
            .iter()
            .map(|line| (*line).clone())
            .collect::<Vec<_>>()
            .join("\n  ")
    );
    let gone: Vec<&&str> = standing
        .iter()
        .filter(|line| !today.contains(**line))
        .collect();
    assert!(
        gone.is_empty(),
        "the standing list names {} path(s) the sweep no longer finds. A list that outlives its \
         facts is what this gate is for: delete these lines from `STANDING`.\n  {}",
        gone.len(),
        gone.iter()
            .map(|line| (**line).to_owned())
            .collect::<Vec<_>>()
            .join("\n  ")
    );
}

#[test]
fn the_sweep_names_a_planted_path_nothing_declares() {
    let root = conformance::workspace_root();
    let plant = names::calibrate(&root).expect("the tree's sources and the names they declare");
    assert_eq!(
        plant.len(),
        1,
        "the plant is one path in one comment, and the sweep read {plant:?}"
    );
    assert_eq!(
        plant[0],
        (
            "ledger::no_function_of_this_name_is_declared".to_owned(),
            Reach::Absent
        ),
        "the plant names a module this crate declares and a function nothing declares, so the \
         sweep owes it the finding rung"
    );
}
