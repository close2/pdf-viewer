//! An owned batch of events, and the accessors a C caller reads it with.
//!
//! **Owned, and that is the whole design.** [`viewer_core::Viewer::handle`] returns an iterator
//! that borrows the viewer; a C caller holding one and calling back in is `&mut Viewer` held
//! twice, which nothing on this side of the boundary would notice. So a command drains its events
//! into a `Vec` before the entry point returns, the viewer's borrow ends there, and re-entrancy
//! stops being a rule anybody has to keep.
//!
//! Everything in this module is safe Rust. [`crate::abi`] is the only place that turns an index
//! and a pointer into a call on one of these.

use pdf_model::navigation::{Dimension, Direction, Motion, Style};
use viewer_core::{Event, Extraction, RenderRequest};

use crate::kinds::{EventKind, PurposeKind};
use crate::status::Status;

/// What a step of a document-wide search reported, flattened for C.
///
/// The Rust event carries an `Option`, which C has not got, so `found` is what says whether
/// `page`, `from` and `to` mean anything — the same shape `pdfv_frame_info` already uses for a
/// frame that is not there.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Searched {
    /// Whether this step found an occurrence.
    pub found: bool,
    /// The zero-based page it is on, where `found`.
    pub page: usize,
    /// The first byte of the page's readback it covers, where `found`.
    pub from: usize,
    /// One past the last, where `found`.
    pub to: usize,
    /// How many pages are still to be read before the search has an answer.
    pub remaining: usize,
    /// Whether the scan came round to the beginning of the document.
    pub wrapped: bool,
}

/// Everything one command produced.
#[derive(Debug)]
pub struct Events {
    /// In the order the viewer produced them, which is the order they must be acted on.
    events: Vec<Event>,
}

impl Events {
    /// Takes what a command produced.
    #[must_use]
    pub fn new(events: Vec<Event>) -> Self {
        Self { events }
    }

    /// How many there are.
    #[must_use]
    pub fn len(&self) -> usize {
        self.events.len()
    }

    /// Whether there are none, which is what a command that changed nothing produces.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.events.is_empty()
    }

    /// Which kind the event at `index` is.
    ///
    /// # Errors
    ///
    /// [`Status::OutOfRange`] where there is no such event.
    pub fn kind(&self, index: usize) -> Result<EventKind, Status> {
        self.events
            .get(index)
            .map(EventKind::of)
            .ok_or(Status::OutOfRange)
    }

    /// One sentence about the event at `index`, whatever kind it is.
    ///
    /// **The half of this ABI that answers "a variant added later".** A C caller switching on a
    /// kind it does not know can still print this, so an event added to `viewer-core` after the
    /// caller was compiled is *logged* rather than dropped in silence — which is trap 5 in the
    /// only form C leaves available. Every arm says something specific, because a sentence
    /// reading "an event" would be the silence with extra steps.
    ///
    /// # Errors
    ///
    /// [`Status::OutOfRange`] where there is no such event.
    pub fn describe(&self, index: usize) -> Result<String, Status> {
        let event = self.events.get(index).ok_or(Status::OutOfRange)?;
        Ok(match event {
            Event::Opened { document, pages } => {
                format!("document {} opened, {pages} page(s)", document.0)
            }
            Event::OpenFailed { document, reason } => {
                format!("document {} could not be opened: {reason}", document.0)
            }
            Event::PasswordRequired { document } => {
                format!(
                    "document {} needs a password (ISO 32000-2 §7.6.4.1)",
                    document.0
                )
            }
            Event::Closed(document) => format!("document {} closed", document.0),
            Event::PageChanged {
                index, label, of, ..
            } => match label {
                Some(label) => {
                    format!("page {} of {of}, labelled {label}", index.saturating_add(1))
                }
                None => format!("page {} of {of}", index.saturating_add(1)),
            },
            Event::NeedsRender(request) => format!(
                "page {} needs rasterising at {}x{}",
                request.page.saturating_add(1),
                request.target.width,
                request.target.height
            ),
            Event::Damage(rect) => format!(
                "the viewport changed within [{}, {}, {}, {}]",
                rect.min.x, rect.min.y, rect.max.x, rect.max.y
            ),
            Event::OpenUri { uri, .. } => format!("a link asks for {uri}"),
            Event::NeedsFile { name, purpose, .. } => {
                format!("the document asks for the file {name:?} ({purpose:?})")
            }
            Event::Transition { transition, .. } => {
                format!("§12.4.4 asks for the transition {:?}", transition.style)
            }
            Event::Dirty { dirty, .. } => {
                if *dirty {
                    "the document differs from the file it was opened from".to_owned()
                } else {
                    "the document matches the file it was opened from".to_owned()
                }
            }
            Event::Saved { bytes, .. } => format!("the saved file is {} byte(s)", bytes.len()),
            Event::Searched {
                found,
                remaining,
                wrapped,
                ..
            } => match found {
                Some(found) => format!(
                    "a search found an occurrence on page {}{}",
                    found.page.saturating_add(1),
                    if *wrapped {
                        ", after coming round to the beginning"
                    } else {
                        ""
                    }
                ),
                None if *remaining == 0 => "a search found nothing in the document".to_owned(),
                None => format!("a search has {remaining} page(s) left to read"),
            },
            // The provenance is in the sentence rather than in an accessor, because this ABI has
            // no structured answer for an extraction at all — a C caller learns the kind and reads
            // this. §O.2.1 is why it matters: a URI naming a file is not a person asking for one,
            // and a caller writing bytes to disk needs to know which it has (ADR 0310).
            Event::Extracted {
                asked,
                name,
                bytes,
                fragment,
                ..
            } => format!(
                "the embedded file {name:?} is {} byte(s), asked for by {}{}",
                bytes.len(),
                match asked {
                    Extraction::Asked => "a person",
                    Extraction::Fragment => "the URI's fragment",
                },
                // §O.2.1's remaining parameters, **said rather than answered**: they reach a Rust
                // host as `Event::Extracted`'s `fragment` and this ABI has no accessor for them,
                // so a C caller cannot yet open the file at the place the URI named. Trap 5 —
                // named here at runtime rather than passed over in silence, and an entry point is
                // what it needs (ADR 0431).
                match fragment {
                    Some(rest) => format!(
                        ", and the URI's fragment continues `{rest}` for it — this ABI states \
                         that but cannot hand it to you yet"
                    ),
                    None => String::new(),
                }
            ),
            Event::Refused {
                operation, notes, ..
            } => format!("{operation:?} was refused: {}", notes.join(" ")),
            Event::Reported { page, notes, .. } => match page {
                Some(page) => format!("page {}: {}", page.saturating_add(1), notes.join(" ")),
                None => notes.join(" "),
            },
        })
    }

    /// [`Event::Opened`]: the document's identity and how many pages it has.
    ///
    /// # Errors
    ///
    /// [`Status::OutOfRange`] where there is no such event, [`Status::WrongKind`] where it is
    /// not this one — never zeroes, because zero is a page count.
    pub fn opened(&self, index: usize) -> Result<(u64, usize), Status> {
        match self.events.get(index).ok_or(Status::OutOfRange)? {
            Event::Opened { document, pages } => Ok((document.0, *pages)),
            _ => Err(Status::WrongKind),
        }
    }

    /// [`Event::PageChanged`]: the zero-based index and how many pages there are.
    ///
    /// # Errors
    ///
    /// [`Status::OutOfRange`] or [`Status::WrongKind`], as [`Self::opened`].
    pub fn page_changed(&self, index: usize) -> Result<(usize, usize), Status> {
        match self.events.get(index).ok_or(Status::OutOfRange)? {
            Event::PageChanged { index, of, .. } => Ok((*index, *of)),
            _ => Err(Status::WrongKind),
        }
    }

    /// [`Event::Searched`]: what a step of a document-wide search found, and what is left.
    ///
    /// Four numbers rather than a struct, because C has no `Option`: `found` says whether the
    /// first two mean anything, `remaining` says whether to pump again, and the page and the
    /// range are the occurrence — which is by then the selection.
    ///
    /// # Errors
    ///
    /// [`Status::OutOfRange`] or [`Status::WrongKind`], as [`Self::opened`].
    pub fn searched(&self, index: usize) -> Result<Searched, Status> {
        match self.events.get(index).ok_or(Status::OutOfRange)? {
            Event::Searched {
                found,
                remaining,
                wrapped,
                ..
            } => Ok(Searched {
                found: found.is_some(),
                page: found.map_or(0, |found| found.page),
                from: found.map_or(0, |found| found.range.0),
                to: found.map_or(0, |found| found.range.1),
                remaining: *remaining,
                wrapped: *wrapped,
            }),
            _ => Err(Status::WrongKind),
        }
    }

    /// [`Event::NeedsRender`]: the request, copied out so the caller may keep it.
    ///
    /// Cloned rather than lent, and the clone is cheap by construction: [`RenderRequest`] holds
    /// the display list behind an `Arc`, which is what lets a zoom re-rasterise without
    /// re-interpreting and what lets this handle go to another thread.
    ///
    /// # Errors
    ///
    /// [`Status::OutOfRange`] or [`Status::WrongKind`].
    pub fn render_request(&self, index: usize) -> Result<RenderRequest, Status> {
        match self.events.get(index).ok_or(Status::OutOfRange)? {
            Event::NeedsRender(request) => Ok(request.clone()),
            _ => Err(Status::WrongKind),
        }
    }

    /// Which document the event at `index` is about.
    ///
    /// **Exhaustive over [`Event`] with no catch-all**, which is what makes it worth having as one
    /// accessor rather than a field on every other: fifteen of the sixteen kinds name a document,
    /// and the sixteenth is [`Event::Damage`], which is about the *viewport*. A message added to
    /// `viewer-core` fails to compile here until somebody says which of the two it is.
    ///
    /// # Errors
    ///
    /// [`Status::OutOfRange`] where there is no such event; [`Status::WrongKind`] for
    /// [`Event::Damage`], which names no document.
    pub fn document(&self, index: usize) -> Result<u64, Status> {
        Ok(match self.events.get(index).ok_or(Status::OutOfRange)? {
            Event::Opened { document, .. }
            | Event::OpenFailed { document, .. }
            | Event::PasswordRequired { document }
            | Event::Closed(document)
            | Event::PageChanged { document, .. }
            | Event::OpenUri { document, .. }
            | Event::NeedsFile { document, .. }
            | Event::Transition { document, .. }
            | Event::Dirty { document, .. }
            | Event::Saved { document, .. }
            | Event::Extracted { document, .. }
            | Event::Refused { document, .. }
            | Event::Reported { document, .. }
            | Event::Searched { document, .. } => document.0,
            Event::NeedsRender(request) => request.document.0,
            Event::Damage(_) => return Err(Status::WrongKind),
        })
    }

    /// The bytes of an [`Event::Saved`] or an [`Event::Extracted`].
    ///
    /// **One accessor for two kinds, and that is the point rather than a shortcut.** `doc/todo/30`
    /// named this as wanting "a byte-buffer accessor rather than a string one": both events carry a
    /// `Vec<u8>` that is a *file*, and a file is not text — §7.5.6's update is a PDF and an
    /// embedded one may be anything at all, so passing either through the NUL-terminated idiom
    /// `pdfv_events_describe` uses would truncate it at the first zero byte.
    ///
    /// # Errors
    ///
    /// [`Status::OutOfRange`] or [`Status::WrongKind`], as [`Self::opened`].
    pub fn bytes(&self, index: usize) -> Result<&[u8], Status> {
        match self.events.get(index).ok_or(Status::OutOfRange)? {
            Event::Saved { bytes, .. } | Event::Extracted { bytes, .. } => Ok(bytes),
            _ => Err(Status::WrongKind),
        }
    }

    /// [`Event::Extracted`]: the file's name, and whether a person asked for it.
    ///
    /// The second is §O.2.1's distinction and not decoration: a URI's `ef` parameter extracts a
    /// file that nobody pressed anything for, and the annex says "a PDF processor may choose to
    /// prompt the user or even prevent opening of the file" for exactly that case (ADR 0310). A
    /// caller writing bytes to disk needs to know which of the two it has.
    ///
    /// # Errors
    ///
    /// [`Status::OutOfRange`] or [`Status::WrongKind`].
    pub fn extracted(&self, index: usize) -> Result<(&str, bool), Status> {
        match self.events.get(index).ok_or(Status::OutOfRange)? {
            Event::Extracted { name, asked, .. } => {
                Ok((name.as_str(), matches!(asked, Extraction::Asked)))
            }
            _ => Err(Status::WrongKind),
        }
    }

    /// [`Event::OpenUri`]: the resolved URI a §12.6.4.8 link asks for.
    ///
    /// # Errors
    ///
    /// [`Status::OutOfRange`] or [`Status::WrongKind`].
    pub fn open_uri(&self, index: usize) -> Result<&str, Status> {
        match self.events.get(index).ok_or(Status::OutOfRange)? {
            Event::OpenUri { uri, .. } => Ok(uri.as_str()),
            _ => Err(Status::WrongKind),
        }
    }

    /// [`Event::NeedsFile`]: what the bytes are wanted for, and the document's own words for the
    /// file.
    ///
    /// **A name and not a path.** Resolving it — or refusing to — is the caller's decision, for
    /// rule 2's reason: a document naming a file is a document asking this machine for something.
    ///
    /// # Errors
    ///
    /// [`Status::OutOfRange`] or [`Status::WrongKind`].
    pub fn needs_file(&self, index: usize) -> Result<(PurposeKind, &str), Status> {
        match self.events.get(index).ok_or(Status::OutOfRange)? {
            Event::NeedsFile { purpose, name, .. } => {
                Ok((PurposeKind::of(*purpose), name.as_str()))
            }
            _ => Err(Status::WrongKind),
        }
    }

    /// [`Event::Damage`]: the part of the viewport that no longer shows what it should.
    ///
    /// `[x0, y0, x1, y1]` in device pixels. A bound on what changed rather than a promise that
    /// everything inside it did.
    ///
    /// # Errors
    ///
    /// [`Status::OutOfRange`] or [`Status::WrongKind`].
    pub fn damage(&self, index: usize) -> Result<[f32; 4], Status> {
        match self.events.get(index).ok_or(Status::OutOfRange)? {
            Event::Damage(rect) => Ok([rect.min.x, rect.min.y, rect.max.x, rect.max.y]),
            _ => Err(Status::WrongKind),
        }
    }

    /// [`Event::Dirty`]: whether the document now differs from the file it was opened from.
    ///
    /// # Errors
    ///
    /// [`Status::OutOfRange`] or [`Status::WrongKind`].
    pub fn dirty(&self, index: usize) -> Result<bool, Status> {
        match self.events.get(index).ok_or(Status::OutOfRange)? {
            Event::Dirty { dirty, .. } => Ok(*dirty),
            _ => Err(Status::WrongKind),
        }
    }

    /// [`Event::Transition`]: Table 164's numbers, without the style.
    ///
    /// # Errors
    ///
    /// [`Status::OutOfRange`] or [`Status::WrongKind`].
    pub fn transition(&self, index: usize) -> Result<TransitionNumbers, Status> {
        match self.events.get(index).ok_or(Status::OutOfRange)? {
            Event::Transition { transition, .. } => Ok(TransitionNumbers {
                seconds: transition.duration,
                vertical: matches!(transition.dimension, Dimension::Vertical),
                outward: matches!(transition.motion, Motion::Outward),
                directed: matches!(transition.direction, Direction::Degrees(_)),
                degrees: match transition.direction {
                    Direction::Degrees(degrees) => degrees,
                    Direction::None => 0.0,
                },
                scale: transition.scale,
                opaque: transition.opaque,
            }),
            _ => Err(Status::WrongKind),
        }
    }

    /// [`Event::Transition`]: Table 164's `/S`, as the table spells it.
    ///
    /// **A name rather than a number this ABI invented**, unlike every other enumeration here, and
    /// the reason is in the entry: `/S` *is* a name in the file, and Table 164's thirteenth case is
    /// `pdf_model::navigation::Style::Unrecognised` — "[a] name Table 164 does not define, kept as
    /// the file wrote it". A number would have had to lose that one, and losing it is exactly what
    /// ADR 0230 refuses: a caller that cannot animate an unknown style falls back to the table's
    /// own default and can say which style it could not play.
    ///
    /// # Errors
    ///
    /// [`Status::OutOfRange`] or [`Status::WrongKind`].
    pub fn transition_style(&self, index: usize) -> Result<String, Status> {
        match self.events.get(index).ok_or(Status::OutOfRange)? {
            Event::Transition { transition, .. } => Ok(style_name(&transition.style)),
            _ => Err(Status::WrongKind),
        }
    }
}

/// Table 164's numbers for one transition, flattened for C.
#[expect(
    clippy::struct_excessive_bools,
    reason = "one field per entry Table 164 states, which is what the table is. Folding them into \
              enums would invent a taxonomy the standard does not have and would make a reader \
              look up which of this crate's names /Dm went into — the reason \
              `pdf_model::form::TextControl` gives for the same shape"
)]
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct TransitionNumbers {
    /// `/D`, the effect's own length in seconds. Zero for `R`, whose row says "the D entry shall
    /// be ignored".
    pub seconds: f32,
    /// `/Dm`: `V` rather than the default `H`.
    pub vertical: bool,
    /// `/M`: `O` rather than the default `I`.
    pub outward: bool,
    /// Whether `/Di` states an angle at all, as against the name `None`.
    pub directed: bool,
    /// `/Di` in degrees counterclockwise from a left-to-right direction, where `directed`.
    ///
    /// The table's own warning, worth carrying: the angle "differs from the page object's Rotate
    /// entry, which is measured clockwise from the top".
    pub degrees: f32,
    /// `/SS`, the scale a `Fly` starts or ends at.
    pub scale: f32,
    /// `/B`, whether a `Fly`'s flown-in area is rectangular and opaque.
    pub opaque: bool,
}

/// Table 164's own spelling of a style.
fn style_name(style: &Style) -> String {
    match style {
        Style::Split => "Split".to_owned(),
        Style::Blinds => "Blinds".to_owned(),
        Style::Box => "Box".to_owned(),
        Style::Wipe => "Wipe".to_owned(),
        Style::Dissolve => "Dissolve".to_owned(),
        Style::Glitter => "Glitter".to_owned(),
        Style::Replace => "R".to_owned(),
        Style::Fly => "Fly".to_owned(),
        Style::Push => "Push".to_owned(),
        Style::Cover => "Cover".to_owned(),
        Style::Uncover => "Uncover".to_owned(),
        Style::Fade => "Fade".to_owned(),
        Style::Unrecognised(name) => String::from_utf8_lossy(name.as_bytes()).into_owned(),
    }
}

#[cfg(test)]
mod tests {
    use viewer_core::{Command, DocumentId, Viewer};

    use super::Events;
    use crate::kinds::EventKind;
    use crate::status::Status;

    /// A batch out of a real open, so that the accessors are exercised on events a viewer made.
    fn opened() -> Events {
        let bytes = std::fs::read(
            std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../doc/PDF20_AN001-BPC.pdf"),
        )
        .expect("the application note is in the tree");
        let mut viewer = Viewer::new(800, 1000, 1.0);
        Events::new(
            viewer
                .handle(Command::Open {
                    id: DocumentId(7),
                    bytes,
                    password: None,
                    fragment: None,
                })
                .collect(),
        )
    }

    /// Every event in a real batch describes itself, and the typed accessors agree with the kinds.
    #[test]
    fn a_real_batch_names_and_describes_every_event_it_holds() {
        let events = opened();
        assert!(!events.is_empty(), "opening a document produces events");
        let mut sentences = Vec::new();
        for index in 0..events.len() {
            let kind = events.kind(index).expect("the index is in range");
            let said = events.describe(index).expect("the index is in range");
            assert!(!said.is_empty(), "{kind:?} described itself as nothing");
            sentences.push(format!("{}: {said}", kind.name().trim_end_matches('\0')));
            if kind == EventKind::Opened {
                assert_eq!(events.opened(index), Ok((7, 5)));
            }
        }
        println!("{}", sentences.join("\n"));
    }

    /// An accessor asked about the wrong kind refuses rather than answering zero.
    ///
    /// The property the whole error design rests on: a page count of zero and "this is not an
    /// `Opened`" have to be two answers, because C has no third channel to tell them apart in.
    #[test]
    fn an_accessor_asked_about_the_wrong_kind_says_so() {
        let events = opened();
        let needs_render = (0..events.len())
            .find(|index| events.kind(*index) == Ok(EventKind::NeedsRender))
            .expect("opening a document asks for its first page");
        assert_eq!(events.opened(needs_render), Err(Status::WrongKind));
        assert_eq!(events.page_changed(needs_render), Err(Status::WrongKind));
        assert!(events.render_request(needs_render).is_ok());
        assert_eq!(events.kind(events.len()), Err(Status::OutOfRange));
        assert_eq!(events.describe(events.len()), Err(Status::OutOfRange));
    }
}
