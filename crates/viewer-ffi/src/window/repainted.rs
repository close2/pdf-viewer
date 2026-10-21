//! What a `QUORRA_OWED_REPAINT` record changed: the pictures' pixels, where they are shown, the
//! marks over them — compared with what the caller was last told, so a caller copies only what is
//! new and a call that changed nothing owes no repaint at all (ADR 2168).
//!
//! **Compared rather than inferred**, for `shown`'s reason (ADR 0244 finding 5): a picture's pixels
//! come from five places — the viewer's frames, a page still being drawn, the retained page, the
//! window over an oversized page and the look taken while page one is read — and an event that
//! said "these pixels changed" would have to be raised by each of them. So each picture is
//! remembered by what it is and where it is, and its bytes by a digest of them.
//!
//! **The digest is SipHash-1-3 under a key drawn for this window**, `std`'s own
//! [`RandomState`](std::collections::hash_map::RandomState): no dependency, and a document cannot
//! arrange two different frames to collide under a key it never sees. Two different frames collide
//! with a probability of about 2⁻⁶⁴, and then the previous frame stays on the screen until the next
//! change — the cost of not keeping a second copy of every pixel shown, written down here as
//! principle 1 asks. Measured on this machine at 6.3 ms for 32 MiB (`-O`, one thread), against a
//! copy across the boundary, a conversion to Windows' premultiplied BGRA in C# and an upload to the
//! compositor, which the record now spares every repaint whose pixels did not move.

use std::hash::BuildHasher;

use super::Window;
use super::owed::Owed;
use crate::{Matches, Quads, Status};

/// What of the page area a repaint changed: `quorra_owed_repaint`'s word and
/// `quorra_owed_repaint_fresh`'s list.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Repainted {
    /// A word of the `QUORRA_REPAINT_*` bits.
    changed: u32,
    /// For each picture the page area now shows, whether its pixels differ from those of the
    /// picture at the same index the caller was last told of.
    fresh: Vec<bool>,
}

impl Repainted {
    /// `QUORRA_REPAINT_FRAMES`: a picture's pixels changed, or a picture was added or taken away.
    pub const FRAMES: u32 = 1;
    /// `QUORRA_REPAINT_PLACES`: a picture moved, or is shown at another extent.
    pub const PLACES: u32 = 2;
    /// `QUORRA_REPAINT_MARKS`: the selection, the find bar's occurrences, Annex O's highlight,
    /// §12.5.1's focus or §12.9's traced path moved.
    pub const MARKS: u32 = 4;

    /// The `QUORRA_REPAINT_*` bits.
    #[must_use]
    pub const fn changed(&self) -> u32 {
        self.changed
    }

    /// Whether the picture at `frame` has pixels the caller has not been handed.
    ///
    /// # Errors
    ///
    /// [`Status::OutOfRange`] past the pictures the page area shows.
    pub fn fresh(&self, frame: usize) -> Result<bool, Status> {
        self.fresh.get(frame).copied().ok_or(Status::OutOfRange)
    }

    /// Two repaints owed in one batch, as the one the caller draws after both: a bit either set,
    /// and a picture either found new. A picture past the first record's list did not exist when
    /// it was made, so the second's word for it is the whole answer.
    pub(super) fn merge(&mut self, later: Self) {
        self.changed |= later.changed;
        self.fresh = later
            .fresh
            .into_iter()
            .enumerate()
            .map(|(index, fresh)| fresh || self.fresh.get(index).copied().unwrap_or(false))
            .collect();
    }
}

/// What the caller was last told the page area shows.
#[derive(Debug, Default)]
pub(super) struct Painted {
    /// This window's digest key.
    keys: std::collections::hash_map::RandomState,
    /// Each picture's pixels, by what they are.
    pixels: Vec<Pixels>,
    /// Each picture's place.
    places: Vec<Place>,
    /// The marks over the pictures.
    marks: Option<Marks>,
}

/// One picture's pixels, by what they are: its page, its raster's facts and a digest of its bytes.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct Pixels {
    /// The page.
    page: usize,
    /// The raster's width and height.
    size: (u32, u32),
    /// The raster's layout.
    format: pdf_render::RasterFormat,
    /// Whether the pixels are the retained page rather than a frame.
    retained: bool,
    /// SipHash-1-3 of the bytes under the window's key.
    digest: u64,
}

/// Where one picture is shown and at what extent, compared by the bits of the origin so that a
/// place is equal to itself whatever it holds.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct Place {
    /// The origin's two coordinates, as bits.
    origin: (u32, u32),
    /// The extent it is shown at.
    extent: (u32, u32),
}

/// The marks a repaint draws over the pictures, as the caller reads them.
#[derive(Debug, Clone, PartialEq)]
struct Marks {
    /// `quorra_window_selection_quads`.
    selection: Result<Quads, Status>,
    /// `quorra_window_find_matches`.
    matches: Result<Matches, Status>,
    /// `quorra_window_highlight_quads`.
    highlight: Result<Quads, Status>,
    /// `quorra_window_focused_annotation`.
    focus: Result<((u32, u16), [f32; 8]), Status>,
    /// `quorra_window_traced_path`.
    traced: Vec<[f32; 2]>,
}

impl Window {
    /// Owes a repaint that says what changed since the caller was last told, and none where
    /// nothing did — which is every required repaint of [`viewer_host::native_window::NativeWindow`]
    /// and the one at the end of every drain (ADR 2168).
    pub(super) fn owe_the_repaint(&mut self) {
        let keys = &self.shown.painted.keys;
        let frames = self.frames();
        let pixels: Vec<Pixels> = frames
            .iter()
            .map(|placed| {
                let raster = placed.frame.raster;
                Pixels {
                    page: placed.frame.page,
                    size: (raster.width, raster.height),
                    format: raster.format,
                    retained: placed.retained,
                    digest: keys.hash_one(raster.data.as_slice()),
                }
            })
            .collect();
        let places: Vec<Place> = frames
            .iter()
            .map(|placed| {
                let (x, y) = placed.shown_at();
                Place {
                    origin: (x.to_bits(), y.to_bits()),
                    extent: placed.extent,
                }
            })
            .collect();
        drop(frames);
        let reads = self.reads();
        let marks = Marks {
            selection: reads.selection_quads(),
            matches: self.matches(),
            highlight: reads.highlight_quads(),
            focus: reads.focused_annotation(),
            traced: self.state.showing.measuring.points().to_vec(),
        };
        let painted = &mut self.shown.painted;
        let fresh: Vec<bool> = pixels
            .iter()
            .enumerate()
            .map(|(index, now)| painted.pixels.get(index) != Some(now))
            .collect();
        let mut changed = 0;
        if pixels.len() != painted.pixels.len() || fresh.contains(&true) {
            changed |= Repainted::FRAMES;
        }
        if places != painted.places {
            changed |= Repainted::PLACES;
        }
        if painted.marks.as_ref() != Some(&marks) {
            changed |= Repainted::MARKS;
        }
        painted.pixels = pixels;
        painted.places = places;
        painted.marks = Some(marks);
        if changed != 0 {
            self.owing.owe(Owed::Repaint(Repainted { changed, fresh }));
        }
    }

    /// §12.9's traced path in device pixels of the viewport, as flat `x, y` pairs: the points a
    /// person has put down while measuring, which the caller draws as a path in its own colours
    /// over the page, and none while the mode is off — `viewer-qt`'s `measuring` (ADR 1264).
    #[must_use]
    pub fn traced_path(&self) -> Vec<f32> {
        self.state
            .showing
            .measuring
            .points()
            .iter()
            .flat_map(|point| [point[0], point[1]])
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::Repainted;

    /// Two repaints in one batch say every bit either said, and a picture new to either is new.
    #[test]
    fn two_repaints_in_one_batch_are_the_one_the_caller_draws() {
        let mut first = Repainted {
            changed: Repainted::FRAMES,
            fresh: vec![true, false],
        };
        first.merge(Repainted {
            changed: Repainted::MARKS,
            fresh: vec![false, false, true],
        });
        assert_eq!(first.changed(), Repainted::FRAMES | Repainted::MARKS);
        assert_eq!(first.fresh(0), Ok(true));
        assert_eq!(first.fresh(1), Ok(false));
        assert_eq!(first.fresh(2), Ok(true));
        assert_eq!(first.fresh(3), Err(crate::Status::OutOfRange));
    }
}
