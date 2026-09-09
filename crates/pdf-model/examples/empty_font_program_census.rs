//! How many embedded font programs decode to no bytes at all, and what each one is.
//!
//! Written for one question, off `doc/todo/00` step 7's ink sweep in the
//! nine-hundred-and-forty-third session: `bug866395.pdf` draws no ink where four reference
//! renderers draw a line of text, and its `/FontFile3` is a ten-byte `FlateDecode` stream that
//! decodes, whole and undamaged, to **zero bytes**.
//!
//! ISO 32000-2 §9.8.1's Table 120 makes `/FontFile3`
//!
//! > A stream containing a font program whose format is specified by the Subtype entry in the
//! > stream dictionary
//!
//! and §9.9's Table 124 requires of each of the three subtypes that "[t]he font program provided
//! as the value of this key shall conform to" the format it names. No byte string of length zero
//! is a Type 1 program, a CFF program or an sfnt, so a descriptor whose font-program stream
//! decodes to nothing has provided none — which is the state §9.8.1's descriptor is *for*, and
//! the state this reader already substitutes for when the key is simply absent.
//!
//! What is counted, and why each number is here:
//!
//! - **Font-program streams**, by key, so the three of Table 120 can be told apart: `/FontFile`
//!   is a bare Type 1 program, `/FontFile2` an sfnt, `/FontFile3` a CFF or an OpenType file.
//! - **How many decode to zero bytes**, split by whether the decode reached the filter's own
//!   end-of-data or stopped at damage. The two are different claims about the file — the first is
//!   a producer that wrote an empty stream, the second a stream whose first byte is already
//!   unreadable — and they reach the same fact: not one byte of the program is available.
//! - **How many decode to bytes that are too few to be any program at all**, printed beside the
//!   empty ones rather than folded into them, because "no bytes" is a claim this census can make
//!   from the standard and "too few bytes" is a claim about three format specifications.
//! - **The documents each empty one is in, by name**, because a population of one is a witness
//!   and a population of forty is a rule, and only the list says which.
//!
//! ```sh
//! cargo run --release -p pdf-model --example empty_font_program_census -- \
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

use pdf_syntax::{Document, ObjectId};

/// The three keys Table 120 gives a font descriptor for a program, in the order it lists them.
const KEYS: [&str; 3] = ["FontFile", "FontFile2", "FontFile3"];

/// Everything this census keeps.
#[derive(Default)]
struct Census {
    /// Documents opened.
    documents: usize,
    /// Font-program streams seen, per key of [`KEYS`].
    streams: [usize; 3],
    /// Of those, the ones that decoded to zero bytes with no damage, per key.
    empty_whole: [usize; 3],
    /// Of those, the ones that decoded to zero bytes and stopped at damage, per key.
    empty_damaged: [usize; 3],
    /// Of those, the ones whose decode refused outright, per key.
    refused: [usize; 3],
    /// Streams that decoded to between one and eleven bytes, which no format can be.
    ///
    /// Eleven is chosen as the largest of the three formats' own minima and is a lower bound in
    /// every direction: a CFF header is four bytes and its Name INDEX two more before any string,
    /// an sfnt table directory is twelve bytes before its first entry, and a Type 1 program's
    /// shortest legal prefix is longer than either. Nothing is decided on this number — it is
    /// printed so that a later round knows whether the empty population has a neighbour.
    nearly_empty: usize,
    /// Every empty one, as `document: /Key (state)`.
    witnesses: Vec<String>,
}

impl Census {
    /// Counts one document's font-program streams.
    fn count(&mut self, name: &str, document: &Document) {
        self.documents += 1;
        for number in document.xref().object_numbers() {
            let object = document.get(ObjectId {
                number,
                generation: 0,
            });
            let Some(dict) = object.as_dict() else {
                continue;
            };
            for (slot, key) in KEYS.iter().enumerate() {
                let entry = document.get_key(dict, key);
                let Some(stream) = entry.as_stream() else {
                    continue;
                };
                self.streams[slot] += 1;
                match document.decoded_stream_data_reported(stream) {
                    Err(_) => {
                        self.refused[slot] += 1;
                        self.witnesses.push(format!("{name}: /{key} (refused)"));
                    }
                    Ok(decoded) if decoded.data.is_empty() => {
                        let state = if decoded.damage.is_none() {
                            self.empty_whole[slot] += 1;
                            "empty, decode whole"
                        } else {
                            self.empty_damaged[slot] += 1;
                            "empty, decode damaged"
                        };
                        self.witnesses.push(format!("{name}: /{key} ({state})"));
                    }
                    Ok(decoded) if decoded.data.len() < 12 => {
                        self.nearly_empty += 1;
                        self.witnesses
                            .push(format!("{name}: /{key} ({} bytes)", decoded.data.len()));
                    }
                    Ok(_) => {}
                }
            }
        }
    }

    /// Prints the measurement.
    fn report(&self) {
        println!("documents opened: {}", self.documents);
        for (slot, key) in KEYS.iter().enumerate() {
            println!(
                "  /{key}: {} streams, {} empty whole, {} empty at damage, {} refused",
                self.streams[slot],
                self.empty_whole[slot],
                self.empty_damaged[slot],
                self.refused[slot],
            );
        }
        println!("  under twelve decoded bytes: {}", self.nearly_empty);
        for witness in &self.witnesses {
            println!("  {witness}");
        }
    }
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
