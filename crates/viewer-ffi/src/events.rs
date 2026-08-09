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

use viewer_core::{Event, RenderRequest};

use crate::kinds::EventKind;
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
            Event::Extracted { name, bytes, .. } => {
                format!("the embedded file {name:?} is {} byte(s)", bytes.len())
            }
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
