//! Every [`Command::Group`] a page's display list holds, and whether its accumulated alpha is
//! ISO 32000-2 §11.6.4.2's **shape** — `Command::Group::alpha_is_shape`.
//!
//! The flag decides one arithmetic and only one: §8.5.4 constrains a group's shape by the clip
//! in force at its blit, and §10.7.4 makes that influence an intersection of sets rather than a
//! product, so a backend that holds the flag composites `min(f, C)` where one that does not
//! composites `α × C`. `render-cpu` takes the first since ADR 0492; quorra takes it where its own
//! `encode::opacity::every_opacity_is_one` can prove the same condition from the command list
//! alone (their ADR 0074).
//!
//! **Two proofs of one clause can differ, and this prints which groups ours admits so that the
//! difference is readable rather than inferred from a page.** It is the instrument
//! `doc/QUORRA_CLIP_LANE_AND_UPLOAD.md` section 6's first question asks for.
//!
//! ```sh
//! cargo run --release -p pdf-model --example group_shape_census -- doc/pdf.js/test/pdfs/*.pdf
//! ```
//!
//! One line per group, deepest nesting last, then one summary line per document and one for the
//! run. A document with no group on its first page prints nothing but its summary, which is the
//! answer as often as a list of groups is.

#![expect(
    clippy::print_stdout,
    reason = "an example whose entire output is a measurement"
)]

use pdf_model::{Pages, interpret};
use pdf_render::{BlendMode, Command, Paint};
use pdf_syntax::Document;

/// What one page's groups came to.
#[derive(Default, Clone, Copy)]
struct Tally {
    /// Groups of any kind.
    groups: usize,
    /// Of them, those whose alpha is their shape.
    shape: usize,
    /// Of them, those [`derivable_from_the_commands`] cannot prove — where a backend holding
    /// only the scene composites a product and this tree's backends take the intersection.
    ours_only: usize,
    /// Groups the command-list proof admits and this tree's flag does not.
    ///
    /// Expected to stay zero: our flag reads `/AIS`, which is strictly more information.
    theirs_only: usize,
}

impl Tally {
    /// Folds another page's or another document's tally into this one.
    fn add(&mut self, other: Self) {
        self.groups = self.groups.saturating_add(other.groups);
        self.shape = self.shape.saturating_add(other.shape);
        self.ours_only = self.ours_only.saturating_add(other.ours_only);
        self.theirs_only = self.theirs_only.saturating_add(other.theirs_only);
    }
}

/// The same question asked of the command list **alone**, the way a backend that never sees the
/// `/AIS` flag has to ask it (quorra's `encode::opacity::every_opacity_is_one`, their ADR 0074).
///
/// ISO 32000-2 §11.6.4.2 gives every elementary object an intrinsic opacity of 1.0, so an opacity
/// below 1.0 can only enter through §11.6.4.3's mask, §11.6.4.4's constant, or a nested group
/// carrying either — and all three are visible in the commands. What such a proof *cannot* see is
/// §11.6.4.3's NOTE 1: a mask or a constant that `/AIS true` made a **shape** rather than an
/// opacity, which is shape all the way down and leaves the equality intact. Only an interpreter
/// knows that, which is why [`Command::Group::alpha_is_shape`] is answered in `pdf-model`.
///
/// **This is a model of quorra's predicate over our display list rather than their code over
/// their scene**, so the count it produces is the size of a population and not a claim about any
/// particular group of theirs. It is written to refuse exactly what theirs refuses: an image,
/// any paint but an opaque solid, a mask anywhere, an alpha below 1.0, and a non-isolated nested
/// group.
///
/// [`Command::Group::alpha_is_shape`]: pdf_render::Command
fn derivable_from_the_commands(commands: &[Command]) -> bool {
    commands.iter().all(|command| match command {
        Command::Fill { paint, mask, .. } | Command::Stroke { paint, mask, .. } => {
            mask.is_none() && matches!(paint, Paint::Solid(colour) if colour.a >= 1.0)
        }
        Command::Group {
            commands,
            alpha,
            mask,
            isolated,
            ..
        } => *alpha >= 1.0 && mask.is_none() && *isolated && derivable_from_the_commands(commands),
        // An image is refused with everything else this walk cannot see through: its samples
        // are its opacity as much as its constant is, and they are not in the command list at
        // all. One arm rather than two because the answer is the same and `Command` is
        // non-exhaustive, so the wildcard is owed anyway.
        _ => false,
    })
}

fn main() {
    let mut run = Tally::default();
    let mut documents = 0_usize;
    let mut with_a_set_flag = 0_usize;
    for path in std::env::args().skip(1) {
        let name = std::path::Path::new(&path)
            .file_name()
            .map_or_else(|| path.clone(), |name| name.to_string_lossy().into_owned());
        let Ok(bytes) = std::fs::read(&path) else {
            println!("{name}\tunreadable");
            continue;
        };
        let Ok(document) = Document::open(bytes) else {
            println!("{name}\tunopened");
            continue;
        };
        documents = documents.saturating_add(1);
        let Some(page) = Pages::new(&document).get(0) else {
            println!("{name}\tno page");
            continue;
        };
        let list = interpret(&document, &page).display_list;
        let mut page_tally = Tally::default();
        walk(&name, list.commands(), 0, &mut page_tally);
        println!(
            "{name}\t{} group(s), {} carrying shape, {} of them beyond a command-list proof, \
             {} the other way",
            page_tally.groups, page_tally.shape, page_tally.ours_only, page_tally.theirs_only
        );
        if page_tally.shape > 0 {
            with_a_set_flag = with_a_set_flag.saturating_add(1);
        }
        run.add(page_tally);
    }
    println!(
        "# {documents} document(s), {} group(s) on their first pages, {} carrying shape, \
         on {with_a_set_flag} page(s); {} of them a command-list proof cannot reach, \
         {} the other way",
        run.groups, run.shape, run.ours_only, run.theirs_only
    );
}

/// What a group's elements are, as a compact histogram.
///
/// The kinds are what the proof turns on rather than decoration: `element_alpha_is_shape` admits
/// a fill, a stroke, an image and a nested group on stated conditions, and refuses a
/// [`Command::Shaped`] outright — so a group of two fills and a group of two shaped objects are
/// two different answers to the same question.
fn kinds(commands: &[Command]) -> String {
    let mut counts: Vec<(&str, usize)> = Vec::new();
    for command in commands {
        let kind = match command {
            Command::Fill { .. } => "fill",
            Command::Stroke { .. } => "stroke",
            Command::Image { .. } => "image",
            Command::Group { .. } => "group",
            Command::Shaped { .. } => "shaped",
            _ => "other",
        };
        match counts.iter_mut().find(|(name, _)| *name == kind) {
            Some((_, n)) => *n = n.saturating_add(1),
            None => counts.push((kind, 1)),
        }
    }
    counts
        .iter()
        .map(|(name, n)| format!("{n} {name}"))
        .collect::<Vec<_>>()
        .join(", ")
}

/// One level of the display list, printing each group it holds and recursing into it.
///
/// [`Command::Shaped`] is descended through both halves because §11.4.6's shaped object states
/// the same object twice and a group may be inside either.
fn walk(name: &str, commands: &[Command], depth: usize, tally: &mut Tally) {
    for command in commands {
        match command {
            Command::Group {
                commands,
                alpha,
                clip,
                mask,
                blend,
                isolated,
                knockout,
                alpha_is_shape,
                ..
            } => {
                tally.groups = tally.groups.saturating_add(1);
                if *alpha_is_shape {
                    tally.shape = tally.shape.saturating_add(1);
                }
                let theirs = derivable_from_the_commands(commands);
                match (*alpha_is_shape, theirs) {
                    (true, false) => tally.ours_only = tally.ours_only.saturating_add(1),
                    (false, true) => tally.theirs_only = tally.theirs_only.saturating_add(1),
                    _ => {}
                }
                println!(
                    "  {name}\tdepth {depth}\talpha {alpha:.4}\tclip {}\tmask {}\tblend {}\t\
                     isolated {isolated}\tknockout {knockout}\talpha_is_shape {alpha_is_shape}\t\
                     from the commands alone {theirs}\t{} element(s): {}",
                    if clip.is_some() { "yes" } else { "no" },
                    if mask.is_some() { "yes" } else { "no" },
                    if *blend == BlendMode::Normal {
                        "Normal"
                    } else {
                        "other"
                    },
                    commands.len(),
                    kinds(commands)
                );
                walk(name, commands, depth.saturating_add(1), tally);
            }
            Command::Shaped { object, shape } => {
                walk(name, std::slice::from_ref(object), depth, tally);
                walk(name, std::slice::from_ref(shape), depth, tally);
            }
            // `Command` is `#[non_exhaustive]`, so a mark this census has no question about is
            // one arm rather than three — and a variant added later joins it silently, which is
            // right for an instrument that counts groups.
            _ => {}
        }
    }
}
