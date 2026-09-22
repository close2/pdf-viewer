//! Which `DCTDecode` images Table 13's `/ColorTransform` actually decides, and what it says.
//!
//! §7.4.8's Table 13 states one rule in three cases, and only one of them is a departure here:
//!
//! > If the encoding algorithm has inserted the Adobe-defined marker code in the encoded data
//! > indicating the ColorTransform value, then the colours shall be transformed, or not, after
//! > the DCT decoding has been performed according to the value provided in the encoded data
//! > and the value of this dictionary entry shall be ignored. If the Adobe-defined marker code
//! > in the encoded data indicating the ColorTransform value is not present then the value
//! > specified in this dictionary entry will be used. If the Adobe-defined marker code (APP14)
//! > in the encoded data indicating the ColorTransform value is not present and this dictionary
//! > entry is not present in the filter dictionary then the default value of ColorTransform
//! > shall be 1 if the image has three components and 0 otherwise.
//!
//! and one sentence above them all removes two component counts from the question outright:
//!
//! > This option shall be ignored if the image has one or two colour components.
//!
//! So the *only* images the entry decides are those with three or four components, no Adobe
//! APP14 marker, and the entry written. This tree does not read the entry (ADR 0036): the
//! codestream decides, in both directions. That decision was priced over the 974 curated
//! documents and has never been asked of the crawl, which is what this census asks.
//!
//! # What is counted, and what the classification is derived from
//!
//! Every stream whose `/Filter` chain ends in `DCTDecode` — an image `XObject`, a `/SMask`, a
//! thumbnail, anything the cross-reference table names — plus §8.9.7's inline images, which
//! carry the same parameters in Table 93's `/DP` and are reached through
//! [`pdf_model::inline_image::scan`]. The codestream's own facts are read off its ISO/IEC
//! 10918-1 marker segments: the frame header's component count and component identifiers, and
//! the APP14 segment's transform byte. The *cases* are Table 13's, written out above; nothing
//! here asks another renderer what it does.
//!
//! The second column of the deciding case is what this tree would draw, and it is not a reading
//! of the clause but a statement about `image::decode_jpeg`: the codestream is handed to
//! `zune-jpeg`, which takes a three-component frame as `YCbCr` unless its component identifiers
//! spell the ASCII letters `R`, `G`, `B`, and a four-component frame with no marker as `CMYK`.
//! Where that answer and the entry's differ, the departure changes a pixel.
//!
//! ```sh
//! cargo build --profile gates -p pdf-model --example colour_transform_census
//! RAYON_NUM_THREADS=4 flock /home/AI/heavy-walk.lock tools/bounded.sh --data 12 --tree 12 -- \
//!     <target-dir>/gates/examples/colour_transform_census @paths.txt
//! ```
//!
//! An argument of the form `@paths.txt` names a file holding one path per line, for a corpus
//! too large for one command line. Documents are walked in parallel, one to a thread, because a
//! `Document` is not `Sync`.

#![expect(
    clippy::print_stdout,
    reason = "an example whose entire output is a measurement"
)]

use std::collections::BTreeMap;

use rayon::iter::{IntoParallelRefIterator as _, ParallelIterator as _};

use pdf_syntax::{Document, Lexer, ObjectId, Stream, Token};

/// How many pages of one document are walked for §8.9.7's inline images.
const MAX_PAGES: usize = 100;

/// What one `DCTDecode` codestream's own marker segments state.
struct Codestream {
    /// The frame header's `Nf`, the number of components per sample.
    components: u8,
    /// Whether the frame's component identifiers are the ASCII letters `R`, `G`, `B`.
    ///
    /// No clause of ISO 32000-2 or of ISO/IEC 10918-1 gives an identifier any meaning; the
    /// convention is `libjpeg`'s, and `zune-jpeg` reads it. It is here because it is half of
    /// what this tree's answer depends on, not because it is half of the clause's.
    identifiers_spell_rgb: bool,
    /// The Adobe APP14 segment's transform byte, where the segment is present.
    app14: Option<u8>,
}

impl Codestream {
    /// What Table 13 decides for this codestream and this dictionary entry.
    fn case(&self, entry: Option<i64>) -> Case {
        if self.components == 1 || self.components == 2 {
            return Case::Ignored;
        }
        match (self.app14, entry) {
            (Some(marker), _) => Case::MarkerWins { marker, entry },
            (None, Some(value)) => Case::EntryDecides { value },
            (None, None) => Case::Default,
        }
    }

    /// Whether this tree transforms this codestream's samples out of a luminance-chrominance
    /// space, which is what `image::decode_jpeg` gets from `zune-jpeg` for a frame carrying no
    /// APP14 marker.
    ///
    /// Three components are `YCbCr` unless the identifiers spell `RGB`; four are `CMYK`, which
    /// is no transform at all. A count of one or two never reaches here — Table 13's own
    /// sentence takes them out.
    fn this_tree_transforms(&self) -> bool {
        self.components == 3 && !self.identifiers_spell_rgb
    }
}

/// Which of Table 13's three cases one image falls in.
#[derive(Clone, Copy)]
enum Case {
    /// One or two components: "This option shall be ignored".
    Ignored,
    /// An APP14 marker is present, so the entry "shall be ignored".
    MarkerWins {
        /// The marker's own transform byte.
        marker: u8,
        /// The dictionary entry, where one is written — which the clause discards.
        entry: Option<i64>,
    },
    /// No APP14 marker and the entry is written: the case the clause hands to the dictionary.
    EntryDecides {
        /// The entry's value.
        value: i64,
    },
    /// Neither, so the clause's stated default governs and there is nothing to disagree with.
    Default,
}

impl Case {
    /// The column this image is counted in.
    fn name(self) -> &'static str {
        match self {
            Self::Ignored => "one or two components (the clause ignores the entry)",
            Self::MarkerWins { .. } => "APP14 present (the clause ignores the entry)",
            Self::EntryDecides { .. } => "no APP14, entry written (the entry decides)",
            Self::Default => "no APP14, no entry (the clause's default)",
        }
    }
}

/// One image in the deciding case, which is what a round has to read against the clause.
struct Witness {
    /// The document this image is in.
    document: String,
    /// Where in that document: an object number, or the page an inline image stands on.
    site: String,
    /// The entry's value.
    value: i64,
    /// The frame's component count.
    components: u8,
    /// Whether the component identifiers spell `RGB`.
    identifiers_spell_rgb: bool,
    /// Which dictionary the name was written in.
    written: Written,
    /// The APP14 transform byte, where the codestream carries one.
    app14: Option<u8>,
    /// Whether reading this entry would change what this tree draws.
    ///
    /// For [`Written::InTheParameterDictionary`] that is the departure's price: the clause reads
    /// the entry there and this tree does not. For [`Written::InTheStreamDictionary`] the clause
    /// reads nothing, so it is the price of honouring a producer's evident intent instead — a
    /// different question, kept in a different list for that reason.
    changes_a_pixel: bool,
}

/// What one document said.
#[derive(Default)]
struct Says {
    /// The document opened.
    opened: bool,
    /// How many `DCTDecode` images were found at all.
    images: usize,
    /// How many of those have a codestream whose markers could not be read.
    unreadable: usize,
    /// How many fell in each of Table 13's cases, by [`Case::name`].
    cases: BTreeMap<&'static str, usize>,
    /// How many APP14 images also write the entry, and how many of those agree with the marker.
    marker_with_entry: usize,
    /// How many of *those* write the same value the marker states.
    marker_agrees: usize,
    /// Every image in the deciding case.
    witnesses: Vec<Witness>,
    /// Every image writing the name where the clause does not read it.
    misplaced: Vec<Witness>,
}

impl Says {
    /// Folds one document's answers into a running total.
    fn absorb(&mut self, other: Self) {
        self.opened |= other.opened;
        self.images = self.images.saturating_add(other.images);
        self.unreadable = self.unreadable.saturating_add(other.unreadable);
        for (case, count) in other.cases {
            let slot = self.cases.entry(case).or_default();
            *slot = slot.saturating_add(count);
        }
        self.marker_with_entry = self
            .marker_with_entry
            .saturating_add(other.marker_with_entry);
        self.marker_agrees = self.marker_agrees.saturating_add(other.marker_agrees);
        self.witnesses.extend(other.witnesses);
        self.misplaced.extend(other.misplaced);
    }

    /// Counts one image.
    fn count(
        &mut self,
        document: &str,
        site: String,
        codestream: &Codestream,
        entry: Option<i64>,
        misplaced: Option<i64>,
    ) {
        self.images = self.images.saturating_add(1);
        if let Some(value) = misplaced {
            self.misplaced.push(Witness {
                document: document.to_owned(),
                site: site.clone(),
                value,
                components: codestream.components,
                identifiers_spell_rgb: codestream.identifiers_spell_rgb,
                written: Written::InTheStreamDictionary,
                app14: codestream.app14,
                // Where the clause reads no entry, the case with none is the one it decides,
                // and this tree follows the codestream: a three-component frame with no marker
                // transforms, which is the clause's own stated default.
                changes_a_pixel: (value == 1) != codestream.this_tree_transforms()
                    && codestream.app14.is_none()
                    && codestream.components >= 3,
            });
        }
        let case = codestream.case(entry);
        let slot = self.cases.entry(case.name()).or_default();
        *slot = slot.saturating_add(1);
        match case {
            Case::MarkerWins { marker, entry } => {
                if let Some(value) = entry {
                    self.marker_with_entry = self.marker_with_entry.saturating_add(1);
                    if value == i64::from(marker) {
                        self.marker_agrees = self.marker_agrees.saturating_add(1);
                    }
                }
            }
            Case::EntryDecides { value } => {
                // The clause's sentence, applied to this frame: 1 transforms out of the
                // luminance-chrominance space, 0 does nothing, and any other value is neither
                // of Table 13's two codes.
                let clause_transforms = value == 1;
                self.witnesses.push(Witness {
                    document: document.to_owned(),
                    site,
                    value,
                    components: codestream.components,
                    identifiers_spell_rgb: codestream.identifiers_spell_rgb,
                    written: Written::InTheParameterDictionary,
                    app14: codestream.app14,
                    changes_a_pixel: clause_transforms != codestream.this_tree_transforms(),
                });
            }
            Case::Ignored | Case::Default => {}
        }
    }
}

/// ISO/IEC 10918-1's `SOI`, which every JPEG datastream begins with.
const SOI: [u8; 2] = [0xFF, 0xD8];

/// Whether this marker code is one of ISO/IEC 10918-1's frame headers.
///
/// `SOF0`–`SOF3`, `SOF5`–`SOF7`, `SOF9`–`SOF11` and `SOF13`–`SOF15`; `C4`, `C8` and `CC` are
/// `DHT`, `JPG` and `DAC` and are not frames.
fn is_frame_header(marker: u8) -> bool {
    matches!(marker, 0xC0..=0xCF) && !matches!(marker, 0xC4 | 0xC8 | 0xCC)
}

/// Reads a `DCTDecode` codestream's frame header and Adobe APP14 segment.
///
/// Walks ISO/IEC 10918-1's marker segments rather than searching for bytes, for the reason
/// `filter::jpeg_extent` does: an `APPn` segment may carry an entire second JPEG, so a search
/// for `FFEE` or for a frame header can find a thumbnail's instead of the image's. The walk
/// stops at `SOS`, by which point both facts have been stated — §7.4.8 puts the component count
/// in the encoded data, and Table 13 calls the marker one "the encoding algorithm has inserted
/// in the encoded data", which for this segment is a header.
fn read_codestream(data: &[u8]) -> Option<Codestream> {
    if data.get(..2)? != SOI {
        return None;
    }
    let mut at = 2_usize;
    let mut components = None;
    let mut identifiers_spell_rgb = false;
    let mut app14 = None;
    loop {
        // A marker is `FF` followed by a code; `FF` fill bytes may stand in front of it.
        if data.get(at)? != &0xFF {
            return None;
        }
        while data.get(at) == Some(&0xFF) {
            at = at.checked_add(1)?;
        }
        let marker = *data.get(at)?;
        at = at.checked_add(1)?;
        match marker {
            // `SOS`: everything after this is entropy-coded data.
            0xDA => break,
            // The standalone markers, which carry no length.
            0x01 | 0xD0..=0xD9 => continue,
            _ => {}
        }
        let length = usize::from(u16::from_be_bytes([
            *data.get(at)?,
            *data.get(at.checked_add(1)?)?,
        ]));
        // The two-byte length counts itself.
        let payload = data.get(at.checked_add(2)?..at.checked_add(length)?)?;
        if is_frame_header(marker) && components.is_none() {
            // Frame header: precision, `Y`, `X`, `Nf`, then `Nf` three-byte descriptions each
            // beginning with the component identifier.
            let count = *payload.get(5)?;
            components = Some(count);
            identifiers_spell_rgb = count == 3
                && b"RGB".iter().enumerate().all(|(index, letter)| {
                    payload.get(index.saturating_mul(3).saturating_add(6)) == Some(letter)
                });
        }
        if marker == 0xEE && app14.is_none() && payload.get(..5) == Some(b"Adobe") {
            // `Adobe`, a two-byte version, two-byte `flags0`, two-byte `flags1`, then the
            // transform byte.
            app14 = payload.get(11).copied();
        }
        at = at.checked_add(length)?;
    }
    Some(Codestream {
        components: components?,
        identifiers_spell_rgb,
        app14,
    })
}

/// Where a `/ColorTransform` was written, which decides whether it is Table 13's entry at all.
#[derive(Clone, Copy, PartialEq, Eq)]
enum Written {
    /// The filter parameter dictionary, which is the one place the clause reads it from.
    InTheParameterDictionary,
    /// A direct key of the stream's own dictionary, where §7.4.1 does not put a filter's
    /// parameters:
    ///
    /// > These optional parameters shall be specified by the DecodeParms entry in the stream's
    /// > dictionary
    ///
    /// and where §7.4.8 does not either — the parameter "shall be specified in the filter
    /// parameter dictionary". A name written here is a name no clause of this standard gives a
    /// meaning to, which is a different fact from a Table 13 entry this tree declines to read.
    InTheStreamDictionary,
}

/// Table 13's entry, and the entry of that name written where the clause does not read one.
///
/// Both are returned because the difference between them is the finding a count of the name
/// alone cannot make: the clause puts this parameter in the filter parameter dictionary, and a
/// producer may write it anywhere.
fn entries_of(
    document: &Document,
    stream: &Stream,
    parms: Option<&pdf_syntax::Dictionary>,
) -> (Option<i64>, Option<i64>) {
    let in_parms = parms.and_then(|parms| document.get_key(parms, "ColorTransform").as_integer());
    let in_dict = document
        .get_key(&stream.dict, "ColorTransform")
        .as_integer();
    (in_parms, in_dict)
}

/// Counts one stream, if its filter chain ends in `DCTDecode`.
fn count_stream(document: &Document, stream: &Stream, name: &str, site: String, says: &mut Says) {
    let Some(image) = document.image_stream(stream) else {
        return;
    };
    if image.codec.as_deref() != Some(&b"DCTDecode"[..]) {
        return;
    }
    let (entry, misplaced) = entries_of(document, stream, image.parms.as_ref());
    if let Some(codestream) = read_codestream(&image.data) {
        says.count(name, site, &codestream, entry, misplaced);
    } else {
        says.images = says.images.saturating_add(1);
        says.unreadable = says.unreadable.saturating_add(1);
    }
}

/// Walks one document: every stream the cross-reference table names, then every page's inline
/// images.
fn examine(path: &str) -> Says {
    let mut says = Says::default();
    let Ok(bytes) = std::fs::read(path) else {
        return says;
    };
    let Ok(document) = Document::open(bytes) else {
        return says;
    };
    says.opened = true;
    let name = std::path::Path::new(path).file_name().map_or_else(
        || path.to_owned(),
        |file| file.to_string_lossy().into_owned(),
    );

    for number in document.xref().object_numbers() {
        let object = document.get(ObjectId {
            number,
            generation: 0,
        });
        let Some(stream) = object.as_stream() else {
            continue;
        };
        count_stream(
            &document,
            stream,
            &name,
            format!("object {number}"),
            &mut says,
        );
    }

    let pages = pdf_model::Pages::new(&document);
    for index in 0..pages.len().min(MAX_PAGES) {
        let Some(page) = pages.get(index) else {
            continue;
        };
        let content = page.content(&document);
        let mut lexer = Lexer::new(&content);
        while let Some(token) = lexer.next_token() {
            let Token::Keyword(word) = &token else {
                continue;
            };
            if word != b"BI" {
                continue;
            }
            let scanned = pdf_model::inline_image::scan(
                &document,
                content.as_slice(),
                lexer.position(),
                &page.resources,
                true,
            );
            if let Ok(stream) = &scanned.image {
                count_stream(
                    &document,
                    stream,
                    &name,
                    format!("inline image on page {}", index.saturating_add(1)),
                    &mut says,
                );
            }
            lexer.seek(scanned.resume);
        }
    }
    says
}

/// The paths to walk: the arguments, and the lines of any argument beginning with `@`.
fn paths() -> Vec<String> {
    let mut out = Vec::new();
    for argument in std::env::args().skip(1) {
        match argument.strip_prefix('@') {
            Some(list) => match std::fs::read_to_string(list) {
                Ok(text) => out.extend(text.lines().map(str::to_owned)),
                Err(error) => println!("{list}: {error}"),
            },
            None => out.push(argument),
        }
    }
    out
}

fn main() {
    let paths = paths();
    eprintln!("{} PDF(s) in the population", paths.len());

    let says =
        paths
            .par_iter()
            .map(|path| examine(path))
            .reduce(Says::default, |mut total, one| {
                total.absorb(one);
                total
            });

    // What it matched, before the count that summarises it.
    for witness in says.witnesses.iter().chain(&says.misplaced) {
        println!(
            "{}: {} states /ColorTransform {} in {} over {} component(s){}{}; reading it {} what \
             this tree draws",
            witness.document,
            witness.site,
            witness.value,
            match witness.written {
                Written::InTheParameterDictionary => "/DecodeParms",
                Written::InTheStreamDictionary => "the stream dictionary (not Table 13's place)",
            },
            witness.components,
            if witness.identifiers_spell_rgb {
                ", identifiers RGB"
            } else {
                ""
            },
            match witness.app14 {
                Some(marker) => format!(", APP14 transform {marker}"),
                None => String::new(),
            },
            if witness.changes_a_pixel {
                "CHANGES"
            } else {
                "leaves"
            },
        );
    }

    let opened = paths.len();
    println!(
        "\n{opened} path(s), {} DCTDecode image(s), {} whose markers could not be read",
        says.images, says.unreadable
    );
    for (case, count) in &says.cases {
        println!("  {count:>8}  {case}");
    }
    println!(
        "  of the APP14 images, {} also write the entry and {} of those write the marker's own \
         value",
        says.marker_with_entry, says.marker_agrees
    );
    let changed = says
        .witnesses
        .iter()
        .filter(|witness| witness.changes_a_pixel)
        .count();
    println!(
        "  of the {} image(s) Table 13's entry decides, {changed} would draw differently if it \
         were read",
        says.witnesses.len()
    );
    let misplaced_changed = says
        .misplaced
        .iter()
        .filter(|witness| witness.changes_a_pixel)
        .count();
    println!(
        "  {} image(s) write /ColorTransform as a direct key of the stream dictionary, where \
         \u{a7}7.4.1 puts no filter parameter; {misplaced_changed} of those would draw \
         differently if it were honoured there anyway",
        says.misplaced.len()
    );
}
