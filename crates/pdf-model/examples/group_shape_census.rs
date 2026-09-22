//! Every [`Command::Group`] a page's display list holds, and whether its accumulated alpha is
//! ISO 32000-2 §11.6.4.2's **shape** — `Command::Group::alpha_is_shape`.
//!
//! The flag decides one arithmetic and only one: §8.5.4 constrains a group's shape by the clip
//! in force at its blit, and §10.7.4 makes that influence an intersection of sets rather than a
//! product, so a backend that holds the flag composites `min(f, C)` where one that does not
//! composites `α × C`. `render-cpu` takes the first since ADR 0492; raster takes it where its own
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
//!
//! # And §11.4.6's knockout, which is the one clause that reads shape apart from opacity
//!
//! > The existence of the knockout feature is the main reason for maintaining a separate shape
//! > value rather than only a single alpha that combines shape and opacity.
//!
//! So the same walk counts where that separation is actually *exercised*: a knockout group at
//! all, a knockout group holding a [`Command::Shaped`] — which is exactly an element whose shape
//! is not the coverage it is drawn with, `ca`/`CA` below 1.0 or a soft mask — and a knockout
//! group whose initial backdrop is its own rather than transparency. Beside them, the groups
//! §11.4.6 did **not** reach: `pdf-model` reports each with the reason it refused, and this
//! prints them by reason, because a population of refusals ranked by cause is what decides
//! whether a construction is owed.

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
    /// Of them, §11.4.6's knockout groups — the one construction that reads a shape apart
    /// from an opacity.
    knockout: usize,
    /// Of those, the ones holding at least one [`Command::Shaped`]: an element whose shape is
    /// not the coverage it is drawn with, which is where the separation is paid for.
    knockout_shaped: usize,
    /// Of those, the ones holding at least one **bare** element whose drawn alpha is below
    /// 1.0 — a §11.6.4.4 constant (`ca`/`CA`) read as opacity, or an image's constant. Its
    /// shape *is* its coverage, so it goes to the backend unwrapped and §11.4.6's `1 − f`
    /// backdrop weight is carried by Porter-Duff `Source`, not by a stated shape. This is the
    /// population `a_bare_translucent_knockout_element_reads_its_constant_as_opacity`
    /// calibrates: where the distinction is owed but a second channel is not.
    knockout_translucent: usize,
    /// Of those, the ones whose initial backdrop is the group's own (`isolated` false beside
    /// `knockout`), which is ADR 0327's construction and the oracle's alone.
    knockout_on_backdrop: usize,
}

impl Tally {
    /// Folds another page's or another document's tally into this one.
    fn add(&mut self, other: Self) {
        self.groups = self.groups.saturating_add(other.groups);
        self.shape = self.shape.saturating_add(other.shape);
        self.ours_only = self.ours_only.saturating_add(other.ours_only);
        self.theirs_only = self.theirs_only.saturating_add(other.theirs_only);
        self.knockout = self.knockout.saturating_add(other.knockout);
        self.knockout_shaped = self.knockout_shaped.saturating_add(other.knockout_shaped);
        self.knockout_translucent = self
            .knockout_translucent
            .saturating_add(other.knockout_translucent);
        self.knockout_on_backdrop = self
            .knockout_on_backdrop
            .saturating_add(other.knockout_on_backdrop);
    }
}

/// The same question asked of the command list **alone**, the way a backend that never sees the
/// `/AIS` flag has to ask it (raster's `encode::opacity::every_opacity_is_one`, their ADR 0074).
///
/// ISO 32000-2 §11.6.4.2 gives every elementary object an intrinsic opacity of 1.0, so an opacity
/// below 1.0 can only enter through §11.6.4.3's mask, §11.6.4.4's constant, or a nested group
/// carrying either — and all three are visible in the commands. What such a proof *cannot* see is
/// §11.6.4.3's NOTE 1: a mask or a constant that `/AIS true` made a **shape** rather than an
/// opacity, which is shape all the way down and leaves the equality intact. Only an interpreter
/// knows that, which is why [`Command::Group::alpha_is_shape`] is answered in `pdf-model`.
///
/// **This is a model of raster's predicate over our display list rather than their code over
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
    let mut with_a_knockout = 0_usize;
    let mut with_a_stated_shape = 0_usize;
    let mut refused_pages = 0_usize;
    let mut refusals: Vec<(&'static str, usize)> = Vec::new();
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
        let interpreted = interpret(&document, &page);
        let mut page_tally = Tally::default();
        walk(
            &name,
            interpreted.display_list.commands(),
            0,
            &mut page_tally,
        );
        for refused in interpreted
            .unsupported
            .iter()
            .filter_map(|report| refused_knockout(&format!("{report:?}")))
        {
            println!("  {name}\trefused\t{refused}");
            match refusals.iter_mut().find(|(reason, _)| *reason == refused) {
                Some((_, count)) => *count = count.saturating_add(1),
                None => refusals.push((refused, 1)),
            }
            refused_pages = refused_pages.saturating_add(1);
        }
        println!(
            "{name}\t{} group(s), {} carrying shape, {} of them beyond a command-list proof, \
             {} the other way; {} knockout, {} with a stated shape, {} on the group's own \
             backdrop",
            page_tally.groups,
            page_tally.shape,
            page_tally.ours_only,
            page_tally.theirs_only,
            page_tally.knockout,
            page_tally.knockout_shaped,
            page_tally.knockout_on_backdrop
        );
        if page_tally.shape > 0 {
            with_a_set_flag = with_a_set_flag.saturating_add(1);
        }
        if page_tally.knockout > 0 {
            with_a_knockout = with_a_knockout.saturating_add(1);
        }
        if page_tally.knockout_shaped > 0 {
            with_a_stated_shape = with_a_stated_shape.saturating_add(1);
        }
        run.add(page_tally);
    }
    println!(
        "# {documents} document(s), {} group(s) on their first pages, {} carrying shape, \
         on {with_a_set_flag} page(s); {} of them a command-list proof cannot reach, \
         {} the other way",
        run.groups, run.shape, run.ours_only, run.theirs_only
    );
    println!(
        "# knockout: {} group(s) on {with_a_knockout} page(s), {} holding a stated shape on \
         {with_a_stated_shape} page(s), {} holding a bare translucent element, {} on the \
         group's own backdrop; {refused_pages} refusal(s)",
        run.knockout, run.knockout_shaped, run.knockout_translucent, run.knockout_on_backdrop
    );
    for (reason, count) in &refusals {
        println!("#   {count}\t{reason}");
    }
}

/// Which of `note_group_structure`'s three refusals a report is, or `None` for a report about
/// something else.
///
/// The reasons are matched by the sentence the interpreter prints rather than by a code, because
/// a report is what a reader of the gate sees and an instrument that agreed with a private enum
/// while disagreeing with the printed sentence would be measuring the wrong thing (trap 27: the
/// assertion is only as good as what it excludes, so the knockout prefix is required first).
fn refused_knockout(report: &str) -> Option<&'static str> {
    if !report.contains("knockout, and an element composites over another") {
        return None;
    }
    Some(if report.contains("/AIS was stated both ways") {
        "/AIS both ways (§11.6.4.3)"
    } else if report.contains("non-isolated, and an element blends") {
        "non-isolated with a blending element (§11.4.6 NOTE 6)"
    } else if report.contains("image mask whose soft mask could not be kept apart") {
        "a stencil whose /SMask was combined as it was read (SampleAlpha::Both)"
    } else {
        "a paint or element whose shape this renderer cannot describe"
    })
}

/// Whether a knockout element goes to the backend **bare** and carries a drawn alpha below
/// 1.0 — the §11.6.4.4 constant read as opacity that `Compose::Knockout`'s `Source` mode
/// weights the backdrop against, rather than a stated [`Command::Shaped`] shape.
///
/// A [`Command::Shaped`] is excluded because its opacity is already stated apart from its
/// shape; a group is excluded because it reaches the backend as a raster. What is left is an
/// elementary mark whose alpha is a constant: a solid paint below full opacity, or an image
/// drawn at a constant below 1.0. A shading below 1.0 arrives as a `Command::Shaped` instead
/// (its coverage is not its colour), so it is not bare and not counted here.
fn bare_translucent(command: &Command) -> bool {
    match command {
        Command::Fill { paint, .. } | Command::Stroke { paint, .. } => {
            matches!(paint, Paint::Solid(colour) if colour.a < 1.0)
        }
        Command::Image { alpha, .. } => *alpha < 1.0,
        _ => false,
    }
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
                if *knockout {
                    tally.knockout = tally.knockout.saturating_add(1);
                    if commands
                        .iter()
                        .any(|element| matches!(element, Command::Shaped { .. }))
                    {
                        tally.knockout_shaped = tally.knockout_shaped.saturating_add(1);
                    }
                    if commands.iter().any(bare_translucent) {
                        tally.knockout_translucent = tally.knockout_translucent.saturating_add(1);
                    }
                    if !*isolated {
                        tally.knockout_on_backdrop = tally.knockout_on_backdrop.saturating_add(1);
                    }
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
