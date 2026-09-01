//! What one wheel notch costs the event thread, with and without §12.5.3's re-interpretation.
//!
//! `Viewer::handle` is synchronous: whatever a command makes the core do is done before the host
//! gets its next event, so a gesture's smoothness is this number. §12.5.3's `NoZoom` makes an
//! annotation's placement a function of the magnification, so a zoom of a page carrying one has
//! to interpret that page again — and that interpretation runs here, on the thread that
//! dispatches every other event (`doc/todo/46`).
//!
//! The measurement is a **pair**, because a duration alone cannot say which part of it is the
//! clause's: the same zoom sequence is driven on a page the file makes view-dependent and on one
//! it does not, and the difference is the re-interpretation. That is `doc/habits.md`'s *attribute
//! by removing the suspect* — the suspect here is removed by choosing a page without it rather
//! than by editing the code.
//!
//! ```sh
//! cargo run --profile gates -p viewer-core --example zoom_cost -- <file.pdf> [ticks]
//! cargo run --profile gates -p viewer-core --example zoom_cost -- <file.pdf> [ticks] <page>
//! ```
//!
//! With no page, both arms are chosen from the document: the first page whose `/Annots` carry
//! Table 167's `NoZoom` flag, and the first page whose annotations do not. With a page (one-based,
//! as a reader counts) only that page is driven, which is what a bisect wants; with the word
//! `all`, every view-dependent page is driven and the run is ordered by what a notch costs, which
//! is what says whether the first such page is a fair witness for the document.
//!
//! The wheel is not the only gesture on this path — `Viewer::settle` derives the magnification
//! from the viewport, so a resize under a fit mode reaches the same code (ADR 0766) — and the
//! `resize` arm below drives that one, at the same page, so the two gestures are priced side by
//! side rather than one being assumed from the other.
//!
//! **The arrangement is scrolled across a page boundary before the gesture starts**, because that
//! is where a reader of a continuous document spends most of their time and it is the only place
//! Table 29's arrangement holds more than one page. The count of pages the notch asks a render
//! for is printed beside the duration for exactly that reason: a per-notch cost that is a *page's*
//! multiplies by what the arrangement is showing, and a number taken with one page on the screen
//! cannot say so.

#![expect(
    clippy::print_stdout,
    reason = "an example whose entire output is the measurement"
)]

use std::time::{Duration, Instant};

use pdf_syntax::Document;
use viewer_core::{Command, DocumentId, Event, PageTarget, Viewer, Zoom};

/// The document this run measures.
const DOCUMENT: DocumentId = DocumentId(1);

/// The viewport every arm is driven in, in device pixels.
const VIEWPORT: (u32, u32) = (1100, 1200);

/// How many notches in one direction before the run turns round.
///
/// A run of `Zoom::In` reaches the magnification ceiling, and past it `set_magnification` reports
/// no change and the notch costs nothing — which would read as the fix this example exists to
/// measure. Eight notches is 1.25⁸, under six times, so every notch of the run does the work.
const LEG: usize = 8;

/// One there-and-back of the gesture, which is what the step counter is reduced modulo.
const SPAN: usize = LEG * 2;

/// How many device pixels one step of the resize arm takes off each viewport dimension.
const STEP: u32 = 16;

/// Table 167's `NoZoom`, which is bit position 4 and therefore the value 8.
const FLAG_NO_ZOOM: i64 = 8;

fn main() {
    let mut arguments = std::env::args().skip(1);
    let Some(path) = arguments.next() else {
        println!("usage: zoom_cost <file.pdf> [ticks] [page]");
        return;
    };
    let ticks: usize = arguments
        .next()
        .and_then(|count| count.parse().ok())
        .unwrap_or(32);
    let chosen = arguments.next();
    let Ok(bytes) = std::fs::read(&path) else {
        println!("cannot read {path}");
        return;
    };
    let Some(found) = witnesses(&bytes) else {
        println!("{path} could not be read for its annotations");
        return;
    };
    println!(
        "{path}, {ticks} notch(es) per arm, viewport {}x{}",
        VIEWPORT.0, VIEWPORT.1
    );
    println!(
        "  {} of {} page(s) state Table 167's NoZoom in an /F",
        found.dependent.len(),
        found.pages
    );

    let arms: Vec<(&str, usize)> = match chosen.as_deref() {
        Some("all") => found
            .dependent
            .iter()
            .map(|&page| ("view-dependent", page))
            .collect(),
        Some(page) => page
            .parse::<usize>()
            .ok()
            .map(|page| vec![("page", page.saturating_sub(1))])
            .unwrap_or_default(),
        None => found
            .dependent
            .first()
            .map(|&page| ("view-dependent", page))
            .into_iter()
            .chain(found.plain.map(|page| ("plain", page)))
            .collect(),
    };
    let mut measured: Vec<(&str, usize, Arm, Arm)> = Vec::new();
    for (what, page) in arms {
        let (Some(zoom), Some(resize)) = (
            drive(&bytes, page, ticks, Gesture::Zoom),
            drive(&bytes, page, ticks, Gesture::Resize),
        ) else {
            println!("  {what:>14} page {}: did not open", page.saturating_add(1));
            continue;
        };
        measured.push((what, page, zoom, resize));
    }
    // Ordered by the wheel notch, because the question this example is asked is which page is
    // the worst one a gesture can land on rather than which page comes first.
    measured.sort_by_key(|(_, _, zoom, _)| std::cmp::Reverse(median(&zoom.spans)));
    for (what, page, zoom, resize) in measured {
        println!(
            "  {what:>14} page {:>5}: zoom {:>10.3?}   resize {:>10.3?}   {} page(s) asked per step",
            page.saturating_add(1),
            median(&zoom.spans),
            median(&resize.spans),
            zoom.pages.iter().max().copied().unwrap_or_default(),
        );
    }
}

/// Which gesture the arm drives, both of which move the magnification (ADR 0766).
#[derive(Clone, Copy)]
enum Gesture {
    /// A wheel notch: `Command::Zoom`, alternating direction every [`LEG`] notches.
    Zoom,
    /// A window resize under a fit mode, which `Viewer::settle` turns into a magnification.
    Resize,
}

/// What one arm of the measurement produced.
struct Arm {
    /// What each command of the gesture cost the event thread.
    spans: Vec<Duration>,
    /// How many pages each of those commands asked for a render of.
    pages: Vec<usize>,
}

/// One arm of the measurement, or `None` where the document did not open.
fn drive(bytes: &[u8], page: usize, ticks: usize, gesture: Gesture) -> Option<Arm> {
    let mut viewer = Viewer::new(VIEWPORT.0, VIEWPORT.1, 1.0);
    let opened: Vec<Event> = viewer
        .handle(Command::Open {
            id: DOCUMENT,
            bytes: bytes.to_vec(),
            password: None,
            fragment: None,
        })
        .collect();
    if !opened
        .iter()
        .any(|event| matches!(event, Event::Opened { .. }))
    {
        return None;
    }
    viewer
        .handle(Command::GoTo(PageTarget::Index(page)))
        .for_each(drop);
    // **A fit mode, because a resize only reaches §12.5.3 through one.** `Viewer::settle` derives
    // the magnification from the viewport, and a document that stated §12.3.2.1's `/OpenAction`
    // with a magnification — the ISO specification is one — opens at a fixed scale instead, where
    // dragging the window edge changes nothing at all. Pressing "fit page" is what a reader does
    // and what ADR 0766's attribution round did.
    viewer
        .handle(Command::Zoom {
            zoom: Zoom::FitPage,
            at: None,
        })
        .for_each(drop);
    // Half a viewport down, which under the fit above is half a page: Table 29's `OneColumn` —
    // what the ISO specification's own catalog asks for — then holds the bottom of one page and
    // the top of the next, which is the arrangement a continuous document is read in.
    viewer
        .handle(Command::Scroll {
            dx: 0.0,
            #[expect(
                clippy::cast_precision_loss,
                reason = "a viewport dimension is a few thousand pixels"
            )]
            dy: VIEWPORT.1 as f32 / 2.0,
        })
        .for_each(drop);
    // The first zoom of a document would carry the page's *first* interpretation, which is not
    // what a gesture pays: one notch out and back leaves the arrangement interpreted and the
    // magnification where it started.
    viewer.handle(command(gesture, 0)).for_each(drop);
    viewer.handle(command(gesture, 1)).for_each(drop);

    let mut arm = Arm {
        spans: Vec::with_capacity(ticks),
        pages: Vec::with_capacity(ticks),
    };
    for tick in 0..ticks {
        let command = command(gesture, tick);
        let started = Instant::now();
        let asked = viewer
            .handle(command)
            .filter(|event| matches!(event, Event::NeedsRender(_)))
            .count();
        arm.spans.push(started.elapsed());
        arm.pages.push(asked);
    }
    Some(arm)
}

/// The command for one step of a gesture, turning round every [`LEG`] steps.
fn command(gesture: Gesture, step: usize) -> Command {
    let outward = (step / LEG).is_multiple_of(2);
    match gesture {
        Gesture::Zoom => Command::Zoom {
            zoom: if outward { Zoom::In } else { Zoom::Out },
            at: None,
        },
        // A fit mode makes the viewport decide the magnification, so a resize is a magnification
        // change with no `Command::Zoom` in it at all.
        Gesture::Resize => {
            // A triangle wave rather than a sawtooth, so that no two consecutive steps ask for
            // the same width: a resize to the width already showing changes no magnification and
            // would be a free step in the middle of the measurement.
            let phase = step % SPAN;
            let rung = u32::try_from(if phase < LEG {
                phase
            } else {
                SPAN.saturating_sub(phase)
            })
            .unwrap_or_default();
            let inset = rung.saturating_mul(STEP);
            // **Both dimensions**, because `Zoom::FitPage` is the smaller of the two fits and a
            // drag of the side of a window whose *height* is the binding one changes no
            // magnification at all — which reads as a gesture that costs nothing rather than as
            // a gesture that reaches no clause.
            Command::Resize {
                width: VIEWPORT.0.saturating_sub(inset),
                height: VIEWPORT.1.saturating_sub(inset),
                scale: 1.0,
            }
        }
    }
}

/// The median of one arm's durations.
///
/// A median rather than a mean, because a gesture is a run of notches and one of them lands on
/// the allocator: what a person feels is the typical notch, and the tail is what the range in
/// `doc/history/` is for.
fn median(spans: &[Duration]) -> Duration {
    let mut sorted = spans.to_vec();
    sorted.sort_unstable();
    sorted.get(sorted.len() / 2).copied().unwrap_or_default()
}

/// Which pages §12.5.3 makes view-dependent, and one page it does not.
///
/// Table 167's flags are read straight off `/Annots` rather than by interpreting the page, which
/// is cheap and is a **superset** of the pages `Interpretation::view_dependent` is true of. Two
/// subclauses move that population in both directions and neither is visible in `/F`: §12.5.6.4
/// makes a `Text` annotation behave as though both flags were always set whatever the file says,
/// and §12.5.6.10's four markup subtypes are drawn in the text of the document and therefore have
/// them cleared (ADR 0172). ISO 32000-2's own 341 are mostly the second
/// of those — 211 strike-outs at one flag value — so the count printed above is what the *files*
/// state and not what the clause decides. `pdf_model::annotation::no_zoom_in_force` is the reading
/// that decides, and `pdf-model/examples/replacement_census` is what compares the two.
///
/// The superset costs this example nothing: a page the clause is not about drives its arm in a
/// few hundred nanoseconds and sorts to the bottom of the run.
struct Witnesses {
    /// How many pages the document has, so the population below has a denominator.
    pages: usize,
    /// Every page whose annotations set `NoZoom`, in page order.
    dependent: Vec<usize>,
    /// The first page that has annotations and none of them set it, as the control arm.
    plain: Option<usize>,
}

/// Reads [`Witnesses`] off the document, or `None` where it cannot be opened at all.
fn witnesses(bytes: &[u8]) -> Option<Witnesses> {
    let document = Document::open(bytes.to_vec()).ok()?;
    let pages = pdf_model::page::Pages::new(&document);
    let mut found = Witnesses {
        pages: pages.len(),
        dependent: Vec::new(),
        plain: None,
    };
    for index in 0..pages.len() {
        let Some(page) = pages.get(index) else {
            continue;
        };
        let annotations = document.get_key(&page.dict, "Annots");
        let Some(entries) = annotations.as_array() else {
            continue;
        };
        let no_zoom = entries.iter().any(|entry| {
            document
                .resolve(entry)
                .as_dict()
                .map(|dict| document.get_key(dict, "F"))
                .and_then(|flags| flags.as_integer())
                .is_some_and(|flags| flags & FLAG_NO_ZOOM != 0)
        });
        if no_zoom {
            found.dependent.push(index);
        } else if !entries.is_empty() {
            found.plain.get_or_insert(index);
        }
    }
    Some(found)
}
