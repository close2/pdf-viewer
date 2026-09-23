//! Image decodes started before the `Do` that draws them, so that a page's photographs are
//! decoded beside each other and beside the rest of its interpretation (ADR 1321).
//!
//! # What this may change, and what it may not
//!
//! **Only when a decode happens.** The display list is built in content-stream order on the
//! interpreter's own thread exactly as before, and a `Do` that meets an image whose decode is
//! still running waits for it: nothing is presented, and nothing is added to the list, before
//! the picture it belongs to exists. That is the owner's answer to `doc/questions/Q121` — a
//! page turn does not present a page whose photograph has not arrived — and it is what keeps
//! `interpret` a pure function of the bytes and the view state, which the cross-backend oracle
//! rests on.
//!
//! **An answer from here is the answer [`super::decode_parts`] gives on the interpreter's
//! thread**, because it is that call, handed the inputs [`super::RasterCache`] would have handed
//! it: the stream, the resource dictionary's §7.8.3 Table 34 `/ColorSpace` entry alone, and the
//! conversion. The fill colour is the one input of that key a decode ahead does not carry, and
//! it may not: [`super::decode_parts`] reads it only for §8.9.6.2's stencil, whose samples say
//! where the current colour is painted rather than carrying a colour of their own, and no stencil
//! is decoded ahead. A slot answers a `Do` only where all
//! three inputs agree; anywhere else the interpreter decodes as it always did, and the work done
//! here was wasted rather than wrong.
//!
//! Two things differ between an answer from here and one made on the interpreter's thread, and
//! neither is in the picture. The decode reads masks through a [`super::MaskCache`] of its own,
//! so two images sharing one `/SMask` object may each read it where the interpreter's cache would
//! have read it once — equal values, two allocations. And the document's memo of §7.4's filter
//! chains is filled in another order, which its own module argues cannot change an answer.
//!
//! # Who waits for whom
//!
//! A slot is [`Progress::Queued`] until a pool thread starts it and [`Progress::Running`] until
//! it has an answer. The interpreter never waits on a queued slot — it takes the slot back and
//! decodes on its own thread — so a pool with no thread to spare costs this nothing but a lock,
//! and a wait is only ever on a decode another thread is already executing. A decode that
//! panics leaves its slot [`Progress::Abandoned`] on the way out, so the waiter decodes for
//! itself rather than waiting for ever; rayon carries the panic to the scope.

use std::sync::{Arc, Condvar, Mutex, PoisonError};

use pdf_syntax::{Dictionary, Document, Object, Stream};

use crate::colour::Conversion;

use super::{COLOUR_SPACES, ImageError, MaskCache, Parts, decode_parts};

/// How many bytes of rasters decoded ahead may wait for their `Do` at once.
///
/// Twice [`super::RASTER_BUDGET`], because the witness this exists for —
/// `22060_A1_01_Plans.pdf`, four 2480 × 2630 photographs each with a soft mask — is four rasters
/// of 26.1 MB, and a budget under their sum would hold back the fourth until the first had been
/// drawn, which is the serial decode again. Charged at issue, from the dictionary's `/Width`
/// and `/Height` at four bytes a sample, and released when the interpreter takes the raster:
/// from then on the bytes are the display list's, which [`super::RASTER_BUDGET`]'s own argument
/// says it was going to spend anyway. What this bounds is the waste — rasters decoded for a `Do`
/// that never comes.
pub(crate) const AHEAD_BUDGET: usize = 128 << 20;

/// The smallest image decoded ahead, in samples.
///
/// Measured on this tree (ADR 1321): a round trip through rayon's pool and a condition variable
/// costs 13.8 µs on average, and a 64 × 64 grey image decodes in 12.8 µs — so at a quarter of this
/// floor the hand-off is the decode's own price, and at the floor itself (128 × 128, 46.9 µs grey,
/// 97.4 µs RGB) it is a small part of it. Below it the interpreter's own thread is the cheaper
/// place for the work.
pub(crate) const AHEAD_FLOOR: u64 = 1 << 14;

/// The table of decodes started ahead of their `Do`, shared between the interpreter and the pool.
#[derive(Debug, Default)]
pub(crate) struct DecodesAhead {
    /// The slots, and whether the interpretation still wants any.
    state: Mutex<Slots>,
    /// Signalled whenever a slot leaves [`Progress::Running`].
    arrived: Condvar,
}

/// [`DecodesAhead`]'s state, under its lock.
#[derive(Debug, Default)]
struct Slots {
    /// Set when the content stream has been read: nothing queued afterwards is started.
    closed: bool,
    /// One per stream offered. Few: a page offers only what it draws, and the budget bounds it.
    entries: Vec<Slot>,
    /// What the slots not yet taken are charged against [`AHEAD_BUDGET`].
    held: usize,
}

/// One decode, the inputs it was started with, and where it has got to.
#[derive(Debug)]
struct Slot {
    /// The image, held so that its address names it for as long as the slot lives — the
    /// argument [`super::RasterCache`] makes for its own entries.
    stream: Arc<Stream>,
    /// §7.8.3 Table 34's `/ColorSpace` entry of the resource dictionary the image was found in,
    /// or [`Object::Null`].
    colour_spaces: Object,
    /// The conversion the decode is made under.
    into: Conversion,
    /// What this slot charges against [`AHEAD_BUDGET`] until it is taken.
    bytes: usize,
    /// Where it has got to.
    progress: Progress,
}

/// Where one slot has got to.
#[derive(Debug)]
enum Progress {
    /// Offered, and not yet started by a pool thread.
    Queued,
    /// A pool thread is decoding it.
    Running,
    /// Decoded, and waiting for its `Do`.
    Done(Result<Parts, ImageError>),
    /// Nothing more will come of it: the interpreter took the answer, or decoded the image
    /// itself, or the interpretation ended before the decode began.
    Settled,
    /// The decode unwound without an answer.
    Abandoned,
}

impl DecodesAhead {
    /// The lock, whatever a panicking decode left it holding: every transition below leaves the
    /// state whole, and [`Progress::Abandoned`] is how a slot says it has no answer.
    fn slots(&self) -> std::sync::MutexGuard<'_, Slots> {
        self.state.lock().unwrap_or_else(PoisonError::into_inner)
    }

    /// Queues a decode of `stream`, returning whether the caller is now to start it.
    ///
    /// Declined once the interpretation has ended, for a stream already offered or already
    /// decoded by the interpreter itself, and where the budget would be exceeded.
    pub(crate) fn offer(
        &self,
        stream: &Arc<Stream>,
        colour_spaces: Object,
        into: Conversion,
        bytes: usize,
    ) -> bool {
        let mut state = self.slots();
        if state.closed
            || state
                .entries
                .iter()
                .any(|slot| Arc::ptr_eq(&slot.stream, stream))
        {
            return false;
        }
        let held = state.held.saturating_add(bytes);
        if held > AHEAD_BUDGET {
            return false;
        }
        state.held = held;
        state.entries.push(Slot {
            stream: Arc::clone(stream),
            colour_spaces,
            into,
            bytes,
            progress: Progress::Queued,
        });
        true
    }

    /// Runs the decode [`Self::offer`] queued for `stream`, on the calling pool thread.
    ///
    /// Does nothing where the slot is no longer queued — the interpreter took it back, or the
    /// interpretation ended first.
    pub(crate) fn decode(&self, document: &Document, stream: &Arc<Stream>) {
        let (colour_spaces, into) = {
            let mut state = self.slots();
            let closed = state.closed;
            let Some(slot) = state
                .entries
                .iter_mut()
                .find(|slot| Arc::ptr_eq(&slot.stream, stream))
            else {
                return;
            };
            if closed || !matches!(slot.progress, Progress::Queued) {
                return;
            }
            slot.progress = Progress::Running;
            (slot.colour_spaces.clone(), slot.into.clone())
        };
        let mut unwinding = Unwinding {
            ahead: self,
            stream,
        };
        // The resource dictionary [`super::RasterCache::parts`] hands the decode: the one entry
        // its key holds, so that the two routes read exactly the same inputs.
        let mut read = Dictionary::new();
        if !matches!(colour_spaces, Object::Null) {
            read.insert(
                pdf_syntax::Name::new(COLOUR_SPACES.as_bytes()),
                colour_spaces,
            );
        }
        let answer = decode_parts(
            document,
            stream,
            &read,
            pdf_render::Color::BLACK,
            &into,
            &mut MaskCache::default(),
        );
        unwinding.settle(Progress::Done(answer));
    }

    /// The answer to a `Do` of `stream` under these inputs, if one was decoded ahead.
    ///
    /// Waits only for a decode already running. `None` tells the interpreter to decode for
    /// itself, and settles the stream so that no decode of it is started afterwards: a slot
    /// still queued is taken back, and a stream nobody offered is recorded as the
    /// interpreter's, because an offer that arrives after the `Do` is work nobody will read.
    pub(crate) fn take(
        &self,
        stream: &Arc<Stream>,
        colour_spaces: &Object,
        into: &Conversion,
    ) -> Option<Result<Parts, ImageError>> {
        let mut state = self.slots();
        loop {
            let Some(at) = state
                .entries
                .iter()
                .position(|slot| Arc::ptr_eq(&slot.stream, stream))
            else {
                state.entries.push(Slot {
                    stream: Arc::clone(stream),
                    colour_spaces: Object::Null,
                    into: into.clone(),
                    bytes: 0,
                    progress: Progress::Settled,
                });
                return None;
            };
            {
                let Slots { entries, held, .. } = &mut *state;
                let slot = entries.get_mut(at)?;
                let agrees = slot.colour_spaces == *colour_spaces && slot.into == *into;
                match &slot.progress {
                    // The one wait, and it is on a decode another thread is executing.
                    Progress::Running if agrees => {}
                    Progress::Done(_) if agrees => {
                        let Progress::Done(answer) =
                            std::mem::replace(&mut slot.progress, Progress::Settled)
                        else {
                            return None;
                        };
                        *held = held.saturating_sub(slot.bytes);
                        slot.bytes = 0;
                        return Some(answer);
                    }
                    Progress::Queued => {
                        slot.progress = Progress::Settled;
                        *held = held.saturating_sub(slot.bytes);
                        slot.bytes = 0;
                        return None;
                    }
                    Progress::Running
                    | Progress::Done(_)
                    | Progress::Settled
                    | Progress::Abandoned => return None,
                }
            }
            state = self
                .arrived
                .wait(state)
                .unwrap_or_else(PoisonError::into_inner);
        }
    }

    /// Says the content stream has been read: no queued decode is started after this.
    ///
    /// A decode already running is left to finish, because it cannot be stopped part way; the
    /// scope that started it waits for it, and ADR 1321 measures what that costs.
    pub(crate) fn close(&self) {
        let mut state = self.slots();
        state.closed = true;
        let Slots { entries, held, .. } = &mut *state;
        for slot in entries.iter_mut() {
            if matches!(slot.progress, Progress::Queued) {
                slot.progress = Progress::Settled;
                *held = held.saturating_sub(slot.bytes);
                slot.bytes = 0;
            }
        }
    }

    /// Whether the interpretation has ended, for a walk that finds work to offer.
    pub(crate) fn closed(&self) -> bool {
        self.slots().closed
    }
}

/// Settles a running slot on the way out of [`DecodesAhead::decode`], by answer or by unwinding.
struct Unwinding<'a> {
    /// The table.
    ahead: &'a DecodesAhead,
    /// The slot's stream.
    stream: &'a Arc<Stream>,
}

impl Unwinding<'_> {
    /// Records the slot's answer and wakes whoever is waiting for it.
    fn settle(&mut self, progress: Progress) {
        {
            let mut state = self.ahead.slots();
            if let Some(slot) = state
                .entries
                .iter_mut()
                .find(|slot| Arc::ptr_eq(&slot.stream, self.stream))
                && matches!(slot.progress, Progress::Running)
            {
                slot.progress = progress;
            }
        }
        self.ahead.arrived.notify_all();
    }
}

impl Drop for Unwinding<'_> {
    /// A no-op after [`Self::settle`], which has already moved the slot on; otherwise the decode
    /// unwound and the slot says it has no answer.
    fn drop(&mut self) {
        self.settle(Progress::Abandoned);
    }
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use pdf_syntax::{Dictionary, Document, Name, Object, Stream};

    use super::DecodesAhead;
    use crate::colour::Conversion;

    /// A document to resolve through; the image below is built in the test, not read from it.
    fn document() -> Document {
        let body = "%PDF-1.7\n1 0 obj\n<< /Type /Catalog /Pages 2 0 R >>\nendobj\n\
                    2 0 obj\n<< /Type /Pages /Kids [] /Count 0 >>\nendobj\n\
                    trailer\n<< /Root 1 0 R >>\n%%EOF\n";
        Document::open(body.as_bytes().to_vec()).expect("the fixture opens")
    }

    /// A 2 × 2 grey image with no filter.
    fn image() -> Arc<Stream> {
        let mut dict = Dictionary::new();
        for (key, value) in [
            ("Subtype", Object::Name(Name::new(b"Image".to_vec()))),
            ("Width", Object::Integer(2)),
            ("Height", Object::Integer(2)),
            ("BitsPerComponent", Object::Integer(8)),
            (
                "ColorSpace",
                Object::Name(Name::new(b"DeviceGray".to_vec())),
            ),
        ] {
            dict.insert(Name::new(key.as_bytes().to_vec()), value);
        }
        Arc::new(Stream {
            dict,
            data: Arc::from(&[0u8, 85, 170, 255][..]),
            decryption_failed: false,
        })
    }

    #[test]
    fn a_queued_decode_is_taken_back_rather_than_waited_for() {
        let ahead = DecodesAhead::default();
        let stream = image();
        assert!(ahead.offer(&stream, Object::Null, Conversion::device(), 16));
        assert!(
            ahead
                .take(&stream, &Object::Null, &Conversion::device())
                .is_none()
        );
        // The pool thread arriving late finds nothing to start.
        ahead.decode(&document(), &stream);
        assert!(
            ahead
                .take(&stream, &Object::Null, &Conversion::device())
                .is_none()
        );
        assert_eq!(
            ahead.slots().held,
            0,
            "a slot taken back is no longer charged"
        );
    }

    #[test]
    fn a_decode_answers_only_the_do_that_agrees_with_its_inputs() {
        let document = document();
        let ahead = DecodesAhead::default();
        let stream = image();
        assert!(ahead.offer(&stream, Object::Null, Conversion::device(), 16));
        ahead.decode(&document, &stream);
        let other = Object::Name(Name::new(b"Elsewhere".to_vec()));
        assert!(
            ahead.take(&stream, &other, &Conversion::device()).is_none(),
            "a /ColorSpace entry the decode was not made under is not answered"
        );
        let answer = ahead
            .take(&stream, &Object::Null, &Conversion::device())
            .expect("a decode that agrees answers")
            .expect("the image decodes");
        let direct = super::decode_parts(
            &document,
            &stream,
            &Dictionary::new(),
            pdf_render::Color::BLACK,
            &Conversion::device(),
            &mut super::MaskCache::default(),
        )
        .expect("the image decodes");
        let samples = |parts: &super::Parts| match &parts.picture {
            super::super::Picture::Complete(image) => image.data.to_vec(),
            super::super::Picture::Masked { base, .. } => base.data.to_vec(),
        };
        assert_eq!(samples(&answer), samples(&direct));
        assert!(
            ahead
                .take(&stream, &Object::Null, &Conversion::device())
                .is_none(),
            "an answer is taken once; a second Do is the raster cache's"
        );
    }

    #[test]
    fn nothing_queued_is_started_once_the_stream_is_read() {
        let ahead = DecodesAhead::default();
        let stream = image();
        assert!(ahead.offer(&stream, Object::Null, Conversion::device(), 16));
        ahead.close();
        ahead.decode(&document(), &stream);
        assert!(
            ahead
                .take(&stream, &Object::Null, &Conversion::device())
                .is_none()
        );
        assert!(
            !ahead.offer(&image(), Object::Null, Conversion::device(), 16),
            "nothing is offered after the close"
        );
    }

    #[test]
    fn a_stream_the_interpreter_decoded_itself_is_not_offered_afterwards() {
        let ahead = DecodesAhead::default();
        let stream = image();
        assert!(
            ahead
                .take(&stream, &Object::Null, &Conversion::device())
                .is_none()
        );
        assert!(!ahead.offer(&stream, Object::Null, Conversion::device(), 16));
    }

    #[test]
    fn the_budget_bounds_what_waits() {
        let ahead = DecodesAhead::default();
        assert!(!ahead.offer(
            &image(),
            Object::Null,
            Conversion::device(),
            super::AHEAD_BUDGET + 1
        ));
    }
}
