//! ISO 32000-2 §12.4.4's presentations: a page's transition, its timing, and its own states.
//!
//! §12.4.4.1 opens with what a presentation is: a document "displayed in the form of a
//! presentation or slide show, advancing from one page to the next either automatically or under
//! user control", and §12.4.4.2 adds the ability to advance between states of *one* page.
//!
//! A page states three things about that, and all three are read here. Two of them are
//! §12.4.4.1's, on the page object itself: [`transition`] is Table 31's `/Trans`, "the style and
//! duration of the visual transition to use when moving from another page to the given page" —
//! whose *value* is Table 164's transition dictionary — and [`display_duration`] is `/Dur`, the
//! seconds after which a presentation advances by itself. The
//! third is §12.4.4.2's [`steps`], the page's own states in order.
//!
//! # A page entry, not an inherited one
//!
//! `/Dur` and `/Trans` are read from the page's own dictionary and not up its `/Parent` chain,
//! because §7.7.3.4 makes exactly four entries inheritable and neither of these is among them.
//! Table 31 lists both as entries of a page object; a `/Pages` node stating one states nothing.
//!
//! # The clause's opening sentence is the whole idea
//!
//! Sub-page navigation is the ability to navigate not only between pages but between different
//! states of the same page.
//!
//! The clause's own NOTE 1 says what a state is made of, and it is a thing this program already
//! has: "[a] single page in a PDF presentation could have a series of bullet points that could
//! be individually turned on and off. In such an example, the bullets would be represented by
//! optional content, and each state of the page would be represented as a navigation node."
//!
//! So a navigation node is a pair of §12.6 actions — one to go forward, one to go back — and
//! walking a page's states is `crate::view::ViewState` performing them. §8.11's groups are
//! read in full, §12.6.4.13's action sets them, and this is the list that says in what order.
//! That is why the ledger's row for this subclause has called it "the one presentation row
//! whose missing piece is a control rather than a renderer" since the fifty-fourth session.
//!
//! # What is deliberately not here
//!
//! **Nothing here turns the presentation on**, and nothing here draws one. §12.4.4.2's NOTE 3
//! says "[a]n interactive PDF processor needs to respect navigation nodes only when in
//! presentation mode", and this crate has no clock to run one with. So what a page *states* about
//! a presentation is read here and a caller with a clock drives all three of these from the
//! values.
//!
//! **A caller does drive all three, since the hundred-and-fiftieth session, the
//! three-hundred-and-ninety-third and the four-hundred-and-eighty-first.** `viewer_core::viewer`
//! advances a `/Dur` on `Command::Tick` (ADR 0135); `viewer_core::transition` shapes the frames of
//! a `/Trans` at a fraction of the way through, which `viewer-ui` draws (ADR 0230) — seven of
//! Table 164's twelve styles, with the other five reported by name; and
//! `viewer_core::presentation` is §12.4.4.2's own state machine, the current navigation node that
//! clause opens by requiring, moved by `Command::GoTo` while `Command::Present` says a
//! presentation is running (ADR 0316).
//!
//! That division is the same one Table 99's `/Order` and `/ListMode` are read under (§8.11.4.3):
//! the document's own words belong to this crate, and what a window does with them does not.
//!
//! NOTE 2 states the obligation that comes with driving it, and it is the reason
//! `ViewState` is a value a caller owns rather than something this crate hides: "[i]nteractive
//! PDF processors need to save the state of optional content groups when a user enters
//! presentation mode and restore it when presentation mode ends." A `ViewState` is `Clone`, and
//! that is the whole of what saving and restoring it takes.

use pdf_syntax::{Dictionary, Document, Name, Object, ObjectId};

use crate::action::Action;

/// Table 164's `/S`: which visual transition moves to this page.
///
/// The twelve the table names, and a thirteenth variant for the name it does not. A style is an
/// animation's *identity* rather than its artwork — the table describes each one in a sentence
/// and states no timing curve, no band width for `Glitter` and no line count for `Blinds` — so
/// what this crate can say is which of the twelve a file asked for.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Style {
    /// `Split`: two lines sweep across the screen, along [`Transition::dimension`] and moving as
    /// [`Transition::motion`] says.
    Split,
    /// `Blinds`: evenly spaced lines sweep in one direction, along [`Transition::dimension`].
    Blinds,
    /// `Box`: a rectangular box sweeps inward or outward, as [`Transition::motion`] says.
    Box,
    /// `Wipe`: one line sweeps across the screen, in [`Transition::direction`].
    Wipe,
    /// `Dissolve`: the old page dissolves gradually to reveal the new one.
    Dissolve,
    /// `Glitter`: a dissolve in a band moving in [`Transition::direction`].
    Glitter,
    /// `R`, the default: the new page replaces the old one with no effect.
    ///
    /// Named for what the table says it does rather than for its one-letter spelling, and it is
    /// the one style whose row states an obligation rather than an appearance — "the D entry
    /// shall be ignored", which is why [`Transition::duration`] answers zero for it.
    Replace,
    /// `Fly`: changes fly to or from offscreen, scaled between [`Transition::scale`] and 1.
    Fly,
    /// `Push`: the old page slides off while the new one pushes it out.
    Push,
    /// `Cover`: the new page slides on over the old one.
    Cover,
    /// `Uncover`: the old page slides off, uncovering the new one.
    Uncover,
    /// `Fade`: the new page gradually becomes visible through the old one.
    Fade,
    /// A name Table 164 does not define, kept as the file wrote it.
    ///
    /// The table's twelve are the styles the clause defines, so a thirteenth name names an
    /// effect no processor can know. Answering `Replace` would be the right thing to *draw* and
    /// a false statement about the file, which is the distinction [`Action::Refused`] draws for
    /// §12.6.4.1's unrecognised action types: a caller that cannot animate an unknown style
    /// falls back to the table's own default and can say that it did.
    Unrecognised(Name),
}

/// Table 164's `/Dm`: which way the lines of a `Split` or `Blinds` run.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Dimension {
    /// `H`, the default.
    Horizontal,
    /// `V`.
    Vertical,
}

/// Table 164's `/M`: which way a `Split`, `Box` or `Fly` moves.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Motion {
    /// `I`, the default: inward from the edges of the page.
    Inward,
    /// `O`: outward from the centre of the page.
    Outward,
}

/// Table 164's `/Di`: the direction a `Wipe`, `Glitter`, `Fly`, `Cover`, `Uncover` or `Push`
/// moves in.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Direction {
    /// Degrees counterclockwise from a left-to-right direction.
    ///
    /// The table's own warning is worth carrying to whatever draws this: the angle "differs from
    /// the page object's Rotate entry, which is measured clockwise from the top". It enumerates
    /// five values — 0, 90, 180, 270 and 315 — and each is legal for only some of the styles;
    /// the number is kept as the file states it, because a file stating a sixth angle has stated
    /// a direction and not a style.
    ///
    /// **An `f32` where the corrected table says integer, deliberately.** ISO 32000-2 errata
    /// issue #36, state Review/Completed, changes this entry's type column from *number* to
    /// *integer* and its sentence with it. Holding the value as stated is the more permissive of
    /// the two readings, and it is the one kept: a fractional angle still names a direction, and
    /// refusing a page over the type of a number that decides only which way a wipe travels would
    /// refuse a picture in order to enforce a grammar.
    Degrees(f32),
    /// The name `None`, "which is relevant only for the Fly transition when the value of SS is
    /// not 1.0".
    None,
}

/// §12.4.4.1's `/Trans`: how a presentation moves *to* this page. Table 164's dictionary.
///
/// Every entry carries the table's default where the file states none, so a caller reads a
/// complete transition or no transition at all. The one entry with no stated default is `/Di`,
/// and it is a conversion artefact rather than a silence — `doc/md/`'s table drops the line, and
/// the standard's own PDF gives it a default of 0.
///
/// NOTE 2 is the sentence a caller needs beside a clock: the duration here "governs the
/// transition to that page from another page; the transition from the page is governed by the
/// next page's transition duration". A presentation asks the page it is going *to*.
#[derive(Debug, Clone, PartialEq)]
pub struct Transition {
    /// `/S`, defaulting to [`Style::Replace`].
    pub style: Style,
    /// `/D`, the effect's own length in seconds, defaulting to 1.
    ///
    /// Zero for [`Style::Replace`], whose row says "the D entry shall be ignored" — so a caller
    /// that waits this long before showing the page waits for exactly as long as the clause
    /// says the effect lasts.
    pub duration: f32,
    /// `/Dm`, defaulting to [`Dimension::Horizontal`]. Read for every style; meaningful for
    /// `Split` and `Blinds`.
    pub dimension: Dimension,
    /// `/M`, defaulting to [`Motion::Inward`]. Meaningful for `Split`, `Box` and `Fly`.
    pub motion: Motion,
    /// `/Di`, defaulting to zero degrees. Meaningful for `Wipe`, `Glitter`, `Fly`, `Cover`,
    /// `Uncover` and `Push`.
    pub direction: Direction,
    /// `/SS`, the scale a `Fly` starts or ends at, defaulting to 1.
    pub scale: f32,
    /// `/B`, whether a `Fly`'s flown-in area is rectangular and opaque. Defaults to false.
    pub opaque: bool,
}

/// The transition a presentation plays when it arrives at this page, if the page states one.
///
/// `None` for a page with no `/Trans`, which is every page of every corpus document — measured
/// by walking every object of all 964 openable ones, where neither a `/Trans` nor a `/Dur` entry
/// occurs. This is the specification track and says so.
#[must_use]
pub fn transition(document: &Document, page: &Dictionary) -> Option<Transition> {
    let dict = document.resolve(page.get("Trans")?);
    let dict = dict.as_dict()?;
    let name = |key: &str| {
        document
            .resolve(dict.get(key).unwrap_or(&Object::Null))
            .as_name()
            .cloned()
    };
    let number = |key: &str| {
        document
            .resolve(dict.get(key).unwrap_or(&Object::Null))
            .as_number()
    };
    let style = name("S").map_or(Style::Replace, |style| match style.as_bytes() {
        b"Split" => Style::Split,
        b"Blinds" => Style::Blinds,
        b"Box" => Style::Box,
        b"Wipe" => Style::Wipe,
        b"Dissolve" => Style::Dissolve,
        b"Glitter" => Style::Glitter,
        b"R" => Style::Replace,
        b"Fly" => Style::Fly,
        b"Push" => Style::Push,
        b"Cover" => Style::Cover,
        b"Uncover" => Style::Uncover,
        b"Fade" => Style::Fade,
        _ => Style::Unrecognised(style),
    });
    // "The new page simply replaces the old one with no special transition effect; the D entry
    // shall be ignored." A replacement takes no time, whatever /D says.
    let duration = if style == Style::Replace {
        0.0
    } else {
        number("D").map_or(1.0, narrow)
    };
    let direction = match document.resolve(dict.get("Di").unwrap_or(&Object::Null)) {
        Object::Name(name) if name.as_bytes() == b"None" => Direction::None,
        other => Direction::Degrees(other.as_number().map_or(0.0, narrow)),
    };
    Some(Transition {
        style,
        duration,
        dimension: match name("Dm").as_ref().map(Name::as_bytes) {
            Some(b"V") => Dimension::Vertical,
            _ => Dimension::Horizontal,
        },
        motion: match name("M").as_ref().map(Name::as_bytes) {
            Some(b"O") => Motion::Outward,
            _ => Motion::Inward,
        },
        direction,
        scale: number("SS").map_or(1.0, narrow),
        opaque: matches!(
            document.resolve(dict.get("B").unwrap_or(&Object::Null)),
            Object::Boolean(true)
        ),
    })
}

/// §12.4.4.1's `/Dur`: the seconds this page is shown before a presentation advances by itself.
///
/// > If no Dur entry is specified in the page object, the page shall not advance automatically.
///
/// which is what `None` means here, and it is not the same as zero. NOTE 1 says the other half:
/// "[t]he user can advance the page manually before the specified time has expired", so this is
/// a maximum rather than a schedule.
#[must_use]
pub fn display_duration(document: &Document, page: &Dictionary) -> Option<f32> {
    Some(narrow(document.resolve(page.get("Dur")?).as_number()?))
}

/// Narrows a PDF number to the precision this crate carries.
///
/// A transition's numbers are a duration in seconds, an angle in degrees and a scale factor,
/// none of which the standard bounds. Every one of them is a quantity a person perceives, so
/// `f32`'s seven digits are past the resolution of the thing being measured; a value beyond the
/// type's range becomes infinity, which is a wait no caller finishes and a scale nothing draws.
#[expect(
    clippy::cast_possible_truncation,
    reason = "a duration, an angle and a scale, each read to more precision than a screen or a \
              clock can show"
)]
fn narrow(value: f64) -> f32 {
    value as f32
}

/// Longest chain of `/Next` links followed.
///
/// The nodes "form a doubly linked list by means of their Next and Prev entries", and both are
/// references a document controls, so the list may be a ring. Each node is visited once, which
/// is what makes the walk terminate; this bounds the *length* as well, because a file may state
/// a very long list as cheaply as a short one.
const MAX_NODES: usize = 4096;

/// One of §12.4.4.2's navigation nodes. Table 165.
#[derive(Debug, Clone, PartialEq)]
pub struct Node {
    /// Table 165's `/NA`: what to perform "when a user navigates forward".
    ///
    /// A list rather than one action because the entry is "an action (which may be the first in
    /// a sequence of actions)" — §12.6.2's `/Next` chain, already flattened into the order it is
    /// performed in.
    pub forward: Vec<Action>,
    /// Table 165's `/PA`: what to perform "when a user navigates backward".
    pub backward: Vec<Action>,
    /// Table 165's `/Dur` (§12.4.4.2): the seconds after which a presentation advances to the
    /// next node.
    ///
    /// > The maximum number of seconds before the interactive PDF processor shall automatically
    /// > advance forward to the next navigation node. If this entry is not specified, no
    /// > automatic advance shall occur.
    ///
    /// So `None` is the clause's "no automatic advance shall occur" and is not zero — the same
    /// distinction [`display_duration`] draws one clause up, and the same relationship between
    /// the two: this one advances a *node* and that one advances the page.
    pub duration: Option<f32>,
}

/// A page's navigation nodes, from its `/PresSteps`, in `/Next` order.
///
/// §12.4.4.2:
///
/// > The primary node on a page shall be determined by the optional PresSteps entry in a page
/// > dictionary
///
/// Empty for a page with no `/PresSteps`, which is every page of every corpus document: neither
/// `PresSteps` nor `NavNode` occurs in the raw bytes of any of the 974, which is a lower bound
/// and enough — this is the specification track rather than the demand one, and says so.
///
/// Only `/Next` is walked. `/Prev` is the same list read the other way, and Table 165 makes the
/// two a doubly linked list rather than two lists: reading both would either produce the same
/// nodes twice or disagree, and a file whose `/Prev` chain is not the reverse of its `/Next`
/// chain has stated two orders with no rule to choose between them.
#[must_use]
pub fn steps(document: &Document, page: &Dictionary) -> Vec<Node> {
    let mut out = Vec::new();
    let mut seen: Vec<ObjectId> = Vec::new();
    let mut entry = page.get("PresSteps").cloned().unwrap_or(Object::Null);
    while out.len() < MAX_NODES {
        if let Object::Reference(id) = entry {
            if seen.contains(&id) {
                break;
            }
            seen.push(id);
        }
        let resolved = document.resolve(&entry);
        let Some(node) = resolved.as_dict() else {
            break;
        };
        out.push(Node {
            forward: crate::action::read(document, node.get("NA").unwrap_or(&Object::Null)),
            backward: crate::action::read(document, node.get("PA").unwrap_or(&Object::Null)),
            duration: node
                .get("Dur")
                .map(|entry| document.resolve(entry))
                .and_then(|entry| entry.as_number())
                .map(narrow),
        });
        entry = node.get("Next").cloned().unwrap_or(Object::Null);
    }
    out
}

#[cfg(test)]
mod tests {
    use super::{Dimension, Direction, Motion, Style, display_duration, steps, transition};
    use crate::action::Action;
    use crate::view::ViewState;
    use pdf_syntax::Document;

    fn document(objects: &[&str]) -> Document {
        use std::fmt::Write as _;
        let mut out = String::from("%PDF-1.7\n");
        let mut offsets = Vec::new();
        for (index, body) in objects.iter().enumerate() {
            offsets.push(out.len());
            let _ = write!(out, "{} 0 obj\n{body}\nendobj\n", index.saturating_add(1));
        }
        let xref_at = out.len();
        let _ = write!(
            out,
            "xref\n0 {}\n0000000000 65535 f \n",
            objects.len().saturating_add(1)
        );
        for offset in &offsets {
            let _ = writeln!(out, "{offset:010} 00000 n ");
        }
        let _ = write!(
            out,
            "trailer\n<< /Size {} /Root 1 0 R >>\nstartxref\n{xref_at}\n%%EOF\n",
            objects.len().saturating_add(1)
        );
        Document::open(out.into_bytes()).expect("a valid file")
    }

    /// The clause's own NOTE 1, built as a document: bullets that turn on one at a time.
    ///
    /// Two navigation nodes, each turning a layer on when a user goes forward and off when they
    /// go back, which is what makes this the row the ledger called "a control rather than a
    /// renderer": every part of it below the list already existed.
    #[test]
    fn a_page_of_bullets_walks_its_states() {
        let doc = document(&[
            "<< /Type /Catalog /Pages 2 0 R /OCProperties << /OCGs [8 0 R 9 0 R] \
             /D << /BaseState /OFF >> >> >>",
            "<< /Type /Pages /Count 1 /Kids [3 0 R] >>",
            "<< /Type /Page /Parent 2 0 R /MediaBox [0 0 100 100] /PresSteps 4 0 R >>",
            "<< /Type /NavNode /NA 6 0 R /PA 7 0 R /Next 5 0 R >>",
            "<< /Type /NavNode /NA 10 0 R /Prev 4 0 R >>",
            "<< /S /SetOCGState /State [/ON 8 0 R] >>",
            "<< /S /SetOCGState /State [/OFF 8 0 R] >>",
            "<< /Type /OCG /Name (bullet one) >>",
            "<< /Type /OCG /Name (bullet two) >>",
            "<< /S /SetOCGState /State [/ON 9 0 R] >>",
        ]);
        let pages = crate::page::Pages::new(&doc);
        let page = pages.get(0).expect("one page");
        let nodes = steps(&doc, &page.dict);
        assert_eq!(nodes.len(), 2, "the /Next chain, in order");
        assert!(matches!(
            nodes.first().map(|node| node.forward.as_slice()),
            Some([Action::SetOcgState(_)])
        ));

        let id = |number| pdf_syntax::ObjectId {
            number,
            generation: 0,
        };
        let mut state = ViewState::of(&doc);
        let on = |state: &ViewState, group| state.optional_content().and_then(|oc| oc.state(group));
        assert_eq!(on(&state, id(8)), Some(false), "/BaseState OFF");

        for node in &nodes {
            state.perform_all(&doc, &node.forward);
        }
        assert_eq!(on(&state, id(8)), Some(true));
        assert_eq!(on(&state, id(9)), Some(true));

        if let Some(first) = nodes.first() {
            state.perform_all(&doc, &first.backward);
        }
        assert_eq!(on(&state, id(8)), Some(false), "and back again");
    }

    /// A ring of `/Next` links terminates.
    ///
    /// Table 165 makes the nodes a doubly linked list and nothing forbids a file from closing
    /// it into a ring; each node is visited once, so the walk ends where it began.
    #[test]
    fn a_ring_of_navigation_nodes_terminates() {
        let doc = document(&[
            "<< /Type /Catalog /Pages 2 0 R >>",
            "<< /Type /Pages /Count 1 /Kids [3 0 R] >>",
            "<< /Type /Page /Parent 2 0 R /MediaBox [0 0 100 100] /PresSteps 4 0 R >>",
            "<< /Type /NavNode /Next 5 0 R >>",
            "<< /Type /NavNode /Next 4 0 R >>",
        ]);
        let pages = crate::page::Pages::new(&doc);
        let page = pages.get(0).expect("one page");
        assert_eq!(steps(&doc, &page.dict).len(), 2);
    }

    /// §12.4.4.1's own EXAMPLE, byte for byte, and every value it states read back.
    ///
    /// The clause describes what it wrote: "a page to be displayed for 5 seconds. Before the
    /// page is displayed, there is a 3.5-second transition in which two vertical lines sweep
    /// outward from the centre to the edges of the page."
    #[test]
    fn the_clauses_own_example_page_states_a_split_and_five_seconds() {
        let doc = document(&[
            "<< /Type /Catalog /Pages 2 0 R >>",
            "<< /Type /Pages /Count 1 /Kids [3 0 R] >>",
            "<< /Type /Page /Parent 2 0 R /MediaBox [0 0 100 100] /Dur 5 \
             /Trans << /Type /Trans /D 3.5 /S /Split /Dm /V /M /O >> >>",
        ]);
        let pages = crate::page::Pages::new(&doc);
        let page = pages.get(0).expect("one page");
        assert_eq!(display_duration(&doc, &page.dict), Some(5.0));
        let trans = transition(&doc, &page.dict).expect("the example states one");
        assert_eq!(
            trans.style,
            Style::Split,
            "two lines sweep across the screen"
        );
        assert!((trans.duration - 3.5).abs() < f32::EPSILON, "3.5 seconds");
        assert_eq!(trans.dimension, Dimension::Vertical, "vertical lines");
        assert_eq!(trans.motion, Motion::Outward, "outward from the centre");
        // Nothing in the example states these three, so they are the table's defaults.
        assert_eq!(trans.direction, Direction::Degrees(0.0));
        assert!((trans.scale - 1.0).abs() < f32::EPSILON);
        assert!(!trans.opaque);
    }

    /// A page states no transition and no advance timing, which is every corpus page.
    ///
    /// The two answers are different: no `/Trans` is no effect to play, and no `/Dur` is the
    /// clause's "the page shall not advance automatically" rather than an advance of zero.
    #[test]
    fn a_page_stating_neither_entry_says_nothing_about_either() {
        let doc = document(&[
            "<< /Type /Catalog /Pages 2 0 R >>",
            "<< /Type /Pages /Count 1 /Kids [3 0 R] /Dur 5 /Trans << /S /Wipe >> >>",
            "<< /Type /Page /Parent 2 0 R /MediaBox [0 0 100 100] >>",
        ]);
        let pages = crate::page::Pages::new(&doc);
        let page = pages.get(0).expect("one page");
        // The /Pages node above states both, and §7.7.3.4 does not make either inheritable.
        assert_eq!(transition(&doc, &page.dict), None);
        assert_eq!(display_duration(&doc, &page.dict), None);
    }

    /// `R` ignores its own `/D`, and a name Table 164 does not define is kept as written.
    #[test]
    fn a_replacement_takes_no_time_and_an_unknown_style_keeps_its_name() {
        let doc = document(&[
            "<< /Type /Catalog /Pages 2 0 R >>",
            "<< /Type /Pages /Count 2 /Kids [3 0 R 4 0 R] >>",
            "<< /Type /Page /Parent 2 0 R /MediaBox [0 0 100 100] \
             /Trans << /S /R /D 9 >> >>",
            "<< /Type /Page /Parent 2 0 R /MediaBox [0 0 100 100] \
             /Trans << /S /Swirl /D 2 /Di /None >> >>",
        ]);
        let pages = crate::page::Pages::new(&doc);
        let first = pages.get(0).expect("two pages");
        let replace = transition(&doc, &first.dict).expect("a /Trans");
        assert_eq!(replace.style, Style::Replace);
        assert!(
            replace.duration.abs() < f32::EPSILON,
            "\"the D entry shall be ignored\""
        );

        let second = pages.get(1).expect("two pages");
        let unknown = transition(&doc, &second.dict).expect("a /Trans");
        assert!(
            matches!(&unknown.style, Style::Unrecognised(name) if name.as_bytes() == b"Swirl"),
            "a thirteenth name is the file's, not a style"
        );
        assert!((unknown.duration - 2.0).abs() < f32::EPSILON);
        assert_eq!(unknown.direction, Direction::None);
    }

    /// Table 165's `/Dur` (§12.4.4.2), and the difference between stating it and not.
    ///
    /// > The maximum number of seconds before the interactive PDF processor shall automatically
    /// > advance forward to the next navigation node. If this entry is not specified, no automatic
    /// > advance shall occur.
    ///
    /// Two nodes differing in that one entry, which is the only way to assert the second sentence:
    /// `None` is a node that waits for a person and zero would be a node that advances at once.
    #[test]
    fn a_navigation_node_states_its_own_advance_timing_or_states_none() {
        let doc = document(&[
            "<< /Type /Catalog /Pages 2 0 R >>",
            "<< /Type /Pages /Count 1 /Kids [3 0 R] >>",
            "<< /Type /Page /Parent 2 0 R /MediaBox [0 0 100 100] /PresSteps 4 0 R >>",
            "<< /Type /NavNode /Dur 2.5 /Next 5 0 R >>",
            "<< /Type /NavNode /Prev 4 0 R >>",
        ]);
        let pages = crate::page::Pages::new(&doc);
        let page = pages.get(0).expect("one page");
        let nodes = steps(&doc, &page.dict);
        assert_eq!(nodes.len(), 2);
        let stated = nodes.first().and_then(|node| node.duration);
        assert!(stated.is_some_and(|seconds| (seconds - 2.5).abs() < f32::EPSILON));
        assert_eq!(
            nodes.get(1).and_then(|node| node.duration),
            None,
            "\"no automatic advance shall occur\""
        );
    }

    /// A page with no `/PresSteps` has no states to walk, which is every corpus page.
    #[test]
    fn a_page_without_pres_steps_has_no_states() {
        let doc = document(&[
            "<< /Type /Catalog /Pages 2 0 R >>",
            "<< /Type /Pages /Count 1 /Kids [3 0 R] >>",
            "<< /Type /Page /Parent 2 0 R /MediaBox [0 0 100 100] >>",
        ]);
        let pages = crate::page::Pages::new(&doc);
        let page = pages.get(0).expect("one page");
        assert!(steps(&doc, &page.dict).is_empty());
    }
}
