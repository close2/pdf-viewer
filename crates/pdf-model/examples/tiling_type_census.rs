//! Table 74's `/TilingType`: which code a corpus asks for, and what the lattice does about it.
//!
//! ISO 32000-2 §8.7.3.1 makes the entry "( Required )" of every Type 1 pattern dictionary and
//! calls it "[a] code that controls adjustments to the spacing of tiles relative to the device
//! pixel grid". Codes 1 and 3 ask for cells "spaced consistently - that is, by a multiple of a
//! device pixel"; code 2 asks for the cell to be left alone and lets the spacing vary instead.
//! `pdf_render::snap_lattice` is the arithmetic and `pdf_model::content::pattern` is where the
//! device pixel comes from — `ViewState`'s magnification, the one thing in this tree that knows
//! how large a page is being drawn (ADR 1080).
//!
//! **This census exists because the departure is not reported per page** (ADR 1031): a report
//! here would fire on almost every patterned page in the world, and a signal that always fires is
//! not a signal (trap 39). So the population is printed by a command instead (`CLAUDE.md`, *Where
//! knowledge lives*), and since ADR 1080 the command also measures what the lattice *did*.
//!
//! What is counted, and why each number is here:
//!
//! - **Type 1 pattern dictionaries**, found by `/PatternType 1` over every object of the file.
//!   That is the population the entry is required of.
//! - **How many state each of Table 74's three codes**, so that 1 and 3 — the two that ask for the
//!   grid — can be told from 2, which asks to be left off it.
//! - **How many state the entry as something else, or not at all**, kept apart from the three
//!   because it is a different claim: a required entry absent or outside its enumeration is a
//!   malformed file, and this tree gives it code 2's treatment as a documented choice.
//! - **The documents, by name, with the codes each asks for**, because a population of one is a
//!   witness and a population of forty is a rule, and only the list says which.
//! - **How many of their first pages the lattice actually moves**, at two magnifications, by
//!   comparing the display list's geometry digest against the same page interpreted with no
//!   magnification stated. A dictionary the content stream never names as a colour reaches no
//!   lattice at all, so this is the narrower number and the one that says what the snap is worth.
//! - **A control**: the same comparison over the documents that hold *no* code 1 or 3 pattern.
//!   It is **not** silent, and that is what it is for: §12.5.3's `NoZoom` is the other thing a
//!   stated magnification places, so the control measures how much of the movement is somebody
//!   else's. What separates them is the third column — a page that carries no `NoZoom` annotation
//!   has nothing but the lattice for a magnification to move, and that is the number to read
//!   (trap 13).
//! - **The ink each mover moved**, as the share of the raster's pixels that differ between the two
//!   interpretations rasterised at the same scale, printed as a distribution with the extremes
//!   named so that ten of them can be opened and looked at (trap 1).
//!
//! ```sh
//! cargo run --release -p pdf-model --example tiling_type_census -- \
//!     doc/pdf.js/test/pdfs/*.pdf
//! ```
//!
//! One process per directory is the surveys' own method, for the surveys' own reason: this parses
//! hostile input, and an abort would take every other verdict in the process with it.

#![expect(
    clippy::print_stdout,
    reason = "an example whose entire output is a measurement"
)]
#![expect(
    clippy::arithmetic_side_effects,
    reason = "counters over a corpus four orders of magnitude below what a usize counts, and \
              this is a measurement rather than a shipped path"
)]

use pdf_render::{Rasterizer, TargetSpec};
use pdf_syntax::{Document, Object, ObjectId};
use render_cpu::CpuRasterizer;

/// Magnifications the lattice is measured at, in device pixels per default user space unit.
///
/// Two rather than one because the snap is a function of the scale: a step that is already whole
/// at one magnification has a fraction to lose at another, and a census taken at a single scale
/// would report the lattice of that scale as though it were the lattice.
const MAGNIFICATIONS: [f32; 2] = [1.0, 2.0];

/// Pixel cap for the ink measurement, which rasterises a mover twice.
const MAX_PIXELS: u64 = 40_000_000;

/// Most pages of one document the lattice is measured over.
///
/// Every page of a document that asks for the grid, up to this many: a pattern dictionary the
/// file states on page one may be painted on page forty, and a census of first pages alone would
/// report the lattice of the documents whose producer happened to pattern their cover.
const MAX_PAGES: usize = 64;

/// Everything this census keeps.
#[derive(Default)]
struct Census {
    /// Documents opened.
    documents: usize,
    /// Type 1 pattern dictionaries seen.
    patterns: usize,
    /// Of those, how many state each of Table 74's codes: index 0 is code 1.
    stated: [usize; 3],
    /// Of those, how many state an integer outside 1 to 3.
    outside: usize,
    /// Of those, how many state no `/TilingType` at all, or a value that is not an integer.
    unstated: usize,
    /// Every document holding a pattern that asks for 1 or 3, as `document: codes`.
    witnesses: Vec<String>,
    /// First pages whose geometry a stated magnification moved, per entry of [`MAGNIFICATIONS`].
    ///
    /// Split by whether the document holds a code 1 or 3 pattern at all: the second slot is the
    /// control, and a number in it is the census measuring §12.5.3 rather than §8.7.3.1.
    moved: [(usize, usize); MAGNIFICATIONS.len()],
    /// Pages compared at all, and how many of those are in a document holding a code 1 or 3
    /// pattern.
    compared: (usize, usize),
    /// Pages whose list the lattice alone moved, per entry of [`MAGNIFICATIONS`].
    ///
    /// A subset of [`Self::moved`]'s first slot: only the pages [`Self::placed`] counts, which
    /// carry no §12.5.3 annotation and therefore have nothing else a magnification could move.
    /// A page carrying both is left out rather than attributed, which undercounts on purpose.
    moved_by_the_lattice: [usize; MAGNIFICATIONS.len()],
    /// Pages on which a constant-spacing tiling was actually placed.
    ///
    /// The number that says whether the arithmetic ran: a dictionary no content stream names as a
    /// colour reaches no lattice, and a lattice already on the grid is laid and moves nothing.
    /// Without this a quiet census reads as "the snap does nothing" when it may mean "no page here
    /// paints one" (trap 13).
    placed: usize,
    /// What each mover moved: its name and the share of its pixels that differ.
    ink: Vec<(f64, String)>,
}

impl Census {
    /// Counts one document's Type 1 pattern dictionaries.
    fn count(&mut self, name: &str, document: &Document) {
        self.documents += 1;
        let mut asked: Vec<i64> = Vec::new();
        for number in document.xref().object_numbers() {
            let object = document.get(ObjectId {
                number,
                generation: 0,
            });
            // A tiling pattern is a *stream*, so its dictionary is reached through the stream —
            // Issue #294 against this subclause inserts "stream" into Table 74's own caption. A
            // plain dictionary is read as well, because a malformed file may write one.
            let dict = match &object {
                Object::Stream(stream) => Some(&stream.dict),
                other => other.as_dict(),
            };
            let Some(dict) = dict else {
                continue;
            };
            if document.get_key(dict, "PatternType").as_integer() != Some(1) {
                continue;
            }
            self.patterns += 1;
            match document.get_key(dict, "TilingType").as_integer() {
                Some(code @ 1..=3) => {
                    #[expect(
                        clippy::indexing_slicing,
                        reason = "the arm's own pattern bounds the code to 1..=3, so the index is \
                                  0..=2 and the array is three long"
                    )]
                    {
                        self.stated[usize::try_from(code - 1).unwrap_or(0)] += 1;
                    }
                    if code != 2 {
                        asked.push(code);
                    }
                }
                Some(_) => self.outside += 1,
                None => self.unstated += 1,
            }
        }
        if !asked.is_empty() {
            asked.sort_unstable();
            asked.dedup();
            let codes: Vec<String> = asked.iter().map(ToString::to_string).collect();
            self.witnesses
                .push(format!("{name}: /TilingType {}", codes.join(" and ")));
        }
        self.measure(name, document, !asked.is_empty());
    }

    /// Prints the measurement.
    fn report(&self) {
        println!("documents opened: {}", self.documents);
        println!("type 1 pattern dictionaries: {}", self.patterns);
        for (slot, count) in self.stated.iter().enumerate() {
            println!("  /TilingType {}: {count}", slot + 1);
        }
        println!("  an integer outside 1 to 3: {}", self.outside);
        println!("  stated as nothing readable: {}", self.unstated);
        println!(
            "documents whose patterns ask for the device pixel grid: {}",
            self.witnesses.len()
        );
        for witness in &self.witnesses {
            println!("  {witness}");
        }
        println!(
            "pages compared: {} ({} of them in a document asking for the grid)",
            self.compared.0, self.compared.1
        );
        println!(
            "pages on which a constant-spacing tiling was placed: {}",
            self.placed
        );
        for (slot, magnification) in MAGNIFICATIONS.iter().enumerate() {
            let (asking, control) = self.moved[slot];
            println!(
                "  at {magnification} px/unit {asking} moved, {} of them by the lattice alone; \
                 the control moved {control}",
                self.moved_by_the_lattice[slot]
            );
        }
        if self.ink.is_empty() {
            return;
        }
        let mut ink = self.ink.clone();
        ink.sort_by(|a, b| b.0.partial_cmp(&a.0).unwrap_or(std::cmp::Ordering::Equal));
        let count = ink.len();
        println!("pixels moved, over {count} movers rasterised at 2 px/unit:");
        for (share, position) in [
            (0, "largest"),
            (count / 4, "upper quartile"),
            (count / 2, "median"),
            (count * 3 / 4, "lower quartile"),
            (count - 1, "smallest"),
        ] {
            if let Some((fraction, name)) = ink.get(share) {
                println!("  {position}: {:.4}% ({name})", fraction * 100.0);
            }
        }
        println!("the ten largest:");
        for (fraction, name) in ink.iter().take(10) {
            println!("  {:.4}%  {name}", fraction * 100.0);
        }
        println!("the ten smallest:");
        for (fraction, name) in ink.iter().rev().take(10) {
            println!("  {:.4}%  {name}", fraction * 100.0);
        }
    }

    /// Interprets a document's pages with and without a magnification, and records what moved.
    ///
    /// Every page of a document that asks for the grid, up to [`MAX_PAGES`]; the first page alone
    /// of every other, which is the control and has nothing to find.
    fn measure(&mut self, name: &str, document: &Document, asking: bool) {
        let pages = pdf_model::Pages::new(document);
        let count = if asking {
            pages.len().min(MAX_PAGES)
        } else {
            1
        };
        for index in 0..count {
            let Some(page) = pages.get(index) else {
                continue;
            };
            self.measure_page(
                &format!("{name} page {}", index + 1),
                document,
                &page,
                asking,
            );
        }
    }

    /// One page, interpreted with and without a magnification.
    fn measure_page(
        &mut self,
        name: &str,
        document: &Document,
        page: &pdf_model::page::Page,
        asking: bool,
    ) {
        let plain = pdf_model::content::interpret(document, page);
        self.compared.0 += 1;
        if asking {
            self.compared.1 += 1;
        }
        for (slot, magnification) in MAGNIFICATIONS.iter().enumerate() {
            let mut view = pdf_model::view::ViewState::of(document);
            view.set_magnification(Some(*magnification));
            let magnified = pdf_model::content::interpret_with(document, page, &view);
            // Whether the arithmetic ran at all. `view_dependent` is raised by §8.7.3.1's lattice
            // and by §12.5.3's `NoZoom`, so the plain interpretation — which no magnification
            // reaches — is what separates them: it already carries the annotation's half.
            let lattice_alone = asking && magnified.view_dependent && !plain.view_dependent;
            if slot == 0 && lattice_alone {
                self.placed += 1;
            }
            // The whole list rather than `DisplayList::geometry_digest`, and that is a
            // correction rather than a preference: the digest hashes each command's variant,
            // clip, mask, blend and path *length* and not the transform on it, so a lattice
            // that moved every site and changed nothing else is exactly what it cannot see.
            // Measured: `issue16038.pdf` page 1 moves 12.8% of its pixels and the digests agree
            // (trap 13 — an instrument that comes back clean is a sentence about the
            // instrument).
            if magnified.display_list == plain.display_list {
                continue;
            }
            if asking {
                self.moved[slot].0 += 1;
            } else {
                self.moved[slot].1 += 1;
            }
            if lattice_alone {
                self.moved_by_the_lattice[slot] += 1;
            }
            // The ink is measured once, at the larger magnification, and only where the
            // geometry moved: a page whose two lists are the same makes the same pixels.
            if slot + 1 == MAGNIFICATIONS.len()
                && lattice_alone
                && let Some(share) = moved_pixels(&plain, &magnified, *magnification)
            {
                self.ink.push((share, name.to_owned()));
            }
        }
    }
}

/// The share of a page's pixels that two interpretations of it disagree about.
///
/// Both are rasterised at `magnification`, which is the scale the magnified one was interpreted
/// for: the question is what the lattice did to the page the viewer would have shown, so the two
/// lists meet the same grid. `None` where either raster is refused.
fn moved_pixels(
    plain: &pdf_model::content::Interpretation,
    magnified: &pdf_model::content::Interpretation,
    magnification: f32,
) -> Option<f64> {
    let mut rasterizer = CpuRasterizer::new();
    let before = TargetSpec::for_page(&plain.display_list, magnification, MAX_PIXELS).ok()?;
    let after = TargetSpec::for_page(&magnified.display_list, magnification, MAX_PIXELS).ok()?;
    let before = rasterizer.rasterize(&plain.display_list, before).ok()?;
    let after = rasterizer.rasterize(&magnified.display_list, after).ok()?;
    if before.data.len() != after.data.len() || before.data.is_empty() {
        return None;
    }
    let differing = before
        .data
        .chunks_exact(4)
        .zip(after.data.chunks_exact(4))
        .filter(|(one, other)| one != other)
        .count();
    let pixels = before.data.len() / 4;
    Some(f64::from(u32::try_from(differing).ok()?) / f64::from(u32::try_from(pixels).ok()?))
}

fn main() {
    let mut census = Census::default();
    for path in std::env::args().skip(1) {
        let Ok(bytes) = std::fs::read(&path) else {
            continue;
        };
        let Ok(document) = Document::open(bytes) else {
            continue;
        };
        let name = path.rsplit('/').next().unwrap_or(&path).to_owned();
        census.count(&name, &document);
    }
    census.report();
}
