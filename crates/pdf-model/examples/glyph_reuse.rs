//! What a glyph coverage cache would hit, counted before anyone writes one.
//!
//! ADR 0128 counted page 6 of ISO 32000-2 at 5933 fills of 107 distinct outlines and made a
//! coverage cache the next thing to measure. The number that decides its *design* is not that
//! one. A cached coverage bitmap can be reused only where the path, the linear part of the device
//! transform **and the sub-pixel phase of its translation** all match, because a glyph a third of
//! a pixel to the right is antialiased differently — so what matters is how many *distinct*
//! entries a page needs, and that depends on how finely the phase is quantised.
//!
//! This prints the whole curve: the exact count, which is what an optimisation that moves no pixel
//! could reuse, and the count at each quantisation. ADR 0131 has what the two ends of it mean and
//! what the oracle says about the quantised end.
//!
//! ```sh
//! cargo run --release -p pdf-model --example glyph_reuse -- [file.pdf] [page] [scale]
//! ```
//!
//! With no arguments it reads page 6 of ISO 32000-2, which is the page ADR 0128 counted and the
//! page ADR 0127 could not draw.
#![expect(
    clippy::expect_used,
    clippy::arithmetic_side_effects,
    clippy::print_stdout,
    reason = "a measurement tool: it should stop loudly if its input is missing, its \
              counters are bounded by one page's commands, and printing is the whole point"
)]

use std::collections::BTreeSet;
use std::fmt::Write as _;

/// One cache entry as this counts them: outline identity, the transform's linear part, and the
/// two quantised phases.
type Entry = (usize, (u32, u32, u32, u32), i32, i32);

use pdf_render::{Command, DisplayList, Transform};

/// How many sub-pixel phases each axis is quantised to, for the second count.
const PHASES: [f32; 5] = [2.0, 4.0, 8.0, 16.0, 32.0];

fn main() {
    let mut args = std::env::args().skip(1);
    let path = args.next().map_or_else(
        || {
            std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
                .join("../../doc/ISO_32000-2_sponsored_EC3.pdf")
        },
        std::path::PathBuf::from,
    );
    let index: usize = args.next().map_or(6, |n| n.parse().expect("a page number"));
    let scale: f32 = args.next().map_or(1.0, |n| n.parse().expect("a scale"));

    let bytes = std::fs::read(&path).expect("readable");
    let document = pdf_syntax::Document::open(bytes).expect("valid PDF");
    let page = pdf_model::Pages::new(&document)
        .get(index - 1)
        .expect("page exists");
    let list = pdf_model::interpret(&document, &page).display_list;
    let target = pdf_render::TargetSpec::for_page(&list, scale, 1 << 30).expect("a target");

    let mut fills = 0_usize;
    let mut outlines = BTreeSet::new();
    let mut exact = BTreeSet::new();
    let mut quantised: Vec<BTreeSet<Entry>> = vec![BTreeSet::new(); PHASES.len()];
    let mut small = 0_usize;
    walk(&list, target.transform, &mut |path, transform| {
        fills += 1;
        let outline = std::sync::Arc::as_ptr(path) as usize;
        outlines.insert(outline);
        let linear = (
            transform.a.to_bits(),
            transform.b.to_bits(),
            transform.c.to_bits(),
            transform.d.to_bits(),
        );
        exact.insert((
            outline,
            linear,
            transform.e.fract().to_bits(),
            transform.f.fract().to_bits(),
        ));
        #[expect(
            clippy::cast_possible_truncation,
            reason = "a fraction times at most 32, so the product is far inside i32"
        )]
        for (index, phases) in PHASES.iter().enumerate() {
            quantised[index].insert((
                outline,
                linear,
                (transform.e.fract() * phases).round() as i32,
                (transform.f.fract() * phases).round() as i32,
            ));
        }
        let mut extent = [f32::MAX, f32::MAX, f32::MIN, f32::MIN];
        for point in path.commands().iter().flat_map(points) {
            let x = transform.a * point.x + transform.c * point.y;
            let y = transform.b * point.x + transform.d * point.y;
            extent = [
                extent[0].min(x),
                extent[1].min(y),
                extent[2].max(x),
                extent[3].max(y),
            ];
        }
        if extent[2] - extent[0] <= 64.0 && extent[3] - extent[1] <= 64.0 {
            small += 1;
        }
    });

    // The clip count is here because the profile that motivated all this found clip masks to be
    // the largest single item on the same page, and it is one line to answer "how many".
    let mut clips = BTreeSet::new();
    let mut shapes = BTreeSet::new();
    for command in list.commands() {
        if let Some(clip) = command.clip() {
            clips.insert(clip);
            shapes.insert(chain_shape(&list, clip));
        }
    }

    println!(
        "{} page {index} at {scale}x: {fills} fills of {} outlines, {small} of them small; \
         {} distinct clips of {} distinct shapes",
        path.display(),
        outlines.len(),
        clips.len(),
        shapes.len(),
    );
    println!("  cache entries, exact phase: {}", exact.len());
    for (phases, entries) in PHASES.iter().zip(quantised.iter()) {
        println!(
            "  cache entries, phase quantised to 1/{phases}: {}",
            entries.len()
        );
    }
}

/// A clip chain written out, so that two chains describing the same region compare equal.
///
/// The mask cache keys on the leaf's `ClipId`, which is a *name*; this is what the region would
/// be keyed by if it were keyed by what it is.
fn chain_shape(list: &DisplayList, leaf: pdf_render::ClipId) -> String {
    let mut out = String::new();
    let mut current = Some(leaf);
    let mut depth = 0;
    while let Some(id) = current {
        depth += 1;
        if depth > 64 {
            break;
        }
        let Some(clip) = list.clip(id) else { break };
        let written = write!(
            out,
            "{:?}|{:?}|{:?};",
            clip.path.commands(),
            clip.transform,
            clip.fill_rule
        );
        // Writing into a `String` cannot fail: `fmt::Write for String` never returns `Err`.
        debug_assert!(written.is_ok());
        current = clip.parent;
    }
    out
}

/// A fill's device extent, for the "small" count: what a coverage cache would hold a bitmap of.
///
/// The points one path command names.
fn points(command: &pdf_render::PathCommand) -> Vec<pdf_render::Point> {
    match *command {
        pdf_render::PathCommand::MoveTo(p) | pdf_render::PathCommand::LineTo(p) => vec![p],
        pdf_render::PathCommand::CurveTo(a, b, c) => vec![a, b, c],
        pdf_render::PathCommand::Close => Vec::new(),
    }
}

/// Every fill in the list and the device transform it is drawn with.
fn walk(
    list: &DisplayList,
    to_device: Transform,
    each: &mut impl FnMut(&std::sync::Arc<pdf_render::Path>, Transform),
) {
    for command in list.commands() {
        match command {
            Command::Fill {
                path, transform, ..
            } => each(path, transform.then(to_device)),
            Command::Group { commands, .. } => {
                for command in commands {
                    if let Command::Fill {
                        path, transform, ..
                    } = command
                    {
                        each(path, transform.then(to_device));
                    }
                }
            }
            _ => {}
        }
    }
}
