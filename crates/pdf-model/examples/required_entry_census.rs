//! Which of ISO 32000-2 §8.9.5.1 Table 87's *required* entries each image `XObject` states,
//! and which state one malformed — the census behind `crate::image`'s refusals.
//!
//! Table 87 makes four entries required, each on its own condition: `/Width` and `/Height`
//! always, `/BitsPerComponent` except for image masks and `JPXDecode` images, and `/ColorSpace`
//! for images except `JPXDecode` ones and never for image masks. A reader that refuses a
//! violation of one of these answers the
//! robustness question — what share of the files that exist draw — and this is the instrument
//! that ranks each refusal against the population it fires on (trap 8, `CLAUDE.md`'s two
//! denominators). A refusal no document exercises is untested code; one many exercise is worth
//! its sentence.
//!
//! It reads every image dictionary in every object of every file named — the cheap, honest
//! denominator for *how many images state what*, decoding nothing. It counts one *optional*
//! entry beside the four required ones — Table 87's PDF 2.0 `/AF` — because §8.9.5.1's row turns
//! on how many images carry associated files, which `attachment::associated` reads off any
//! dictionary and which no in-scope clause makes a rendering processor consume. `/ImageMask` and the last
//! codec of the `/Filter` chain decide which entries a given image is *required* to state, so
//! the malformed counts are conditioned exactly as Table 87 conditions the requirement.
//!
//! ```sh
//! cargo run --release -p pdf-model --example required_entry_census -- doc/pdf.js/test/pdfs/*.pdf
//! ```

use std::path::PathBuf;

use pdf_syntax::{Dictionary, Document, Object, ObjectId};

/// The five component widths Table 87 permits, and nothing else.
const PERMITTED_DEPTHS: [i64; 5] = [1, 2, 4, 8, 16];

/// One image dictionary's standing against Table 87's required entries.
#[derive(Default)]
struct Tally {
    /// Image dictionaries seen.
    images: u64,
    /// Of them, image masks (`/ImageMask true`).
    masks: u64,
    /// Of them, `JPXDecode` images.
    jpx: u64,
    /// `/Width` absent or not a positive integer.
    bad_width: u64,
    /// `/Height` absent or not a positive integer.
    bad_height: u64,
    /// `/BitsPerComponent` required (not a mask, not `JPXDecode`) and absent.
    missing_bpc: u64,
    /// `/BitsPerComponent` present with a value outside Table 87's five.
    out_of_range_bpc: u64,
    /// `/ColorSpace` required (not a mask, not `JPXDecode`) and absent.
    missing_colour_space: u64,
    /// `/ColorSpace` present on an image mask, which Table 87 forbids.
    colour_space_on_mask: u64,
    /// Image dictionaries stating an `/AF` array — Table 87's one PDF 2.0 associated-files entry.
    ///
    /// Not a required entry and never a refusal: it is counted here because §8.9.5.1's row turns
    /// on how many images carry one. `attachment::associated` reads §14.13's array off any
    /// dictionary, so an image needs no reader of its own, and §6.3.2.2 makes a rendering
    /// processor owe no *consumer* of an image's associated files (interchange, §14.13.1) — the
    /// question the count answers is only whether any document exercises the carrier at all.
    af: u64,
}

impl Tally {
    /// Folds one image dictionary into the tally.
    fn add(&mut self, document: &Document, dict: &Dictionary) {
        self.images = self.images.saturating_add(1);
        let is_mask = matches!(document.get_key(dict, "ImageMask"), Object::Boolean(true));
        let codec = last_codec(document, dict);
        let is_jpx = codec.as_deref() == Some(b"JPXDecode");
        if is_mask {
            self.masks = self.masks.saturating_add(1);
        }
        if is_jpx {
            self.jpx = self.jpx.saturating_add(1);
        }

        if positive_integer(document, dict, "Width").is_none() {
            self.bad_width = self.bad_width.saturating_add(1);
        }
        if positive_integer(document, dict, "Height").is_none() {
            self.bad_height = self.bad_height.saturating_add(1);
        }

        // `/BitsPerComponent`: required except for masks and JPXDecode. A mask's is optional
        // and, if present, shall be 1; JPXDecode ignores it and takes the depth from the
        // codestream (§7.4.9). So the "missing when required" count excludes both.
        match document.get_key(dict, "BitsPerComponent") {
            Object::Integer(value) if PERMITTED_DEPTHS.contains(&value) => {}
            Object::Null if is_mask || is_jpx => {}
            Object::Null => self.missing_bpc = self.missing_bpc.saturating_add(1),
            _ => self.out_of_range_bpc = self.out_of_range_bpc.saturating_add(1),
        }

        // `/ColorSpace`: required except for JPXDecode; not permitted for masks.
        let colour_space = document.get_key(dict, "ColorSpace");
        if is_mask {
            if !matches!(colour_space, Object::Null) {
                self.colour_space_on_mask = self.colour_space_on_mask.saturating_add(1);
            }
        } else if !is_jpx && matches!(colour_space, Object::Null) {
            self.missing_colour_space = self.missing_colour_space.saturating_add(1);
        }

        if matches!(document.get_key(dict, "AF"), Object::Array(_)) {
            self.af = self.af.saturating_add(1);
        }
    }

    /// Adds another tally into this one.
    fn absorb(&mut self, other: &Self) {
        self.images = self.images.saturating_add(other.images);
        self.masks = self.masks.saturating_add(other.masks);
        self.jpx = self.jpx.saturating_add(other.jpx);
        self.bad_width = self.bad_width.saturating_add(other.bad_width);
        self.bad_height = self.bad_height.saturating_add(other.bad_height);
        self.missing_bpc = self.missing_bpc.saturating_add(other.missing_bpc);
        self.out_of_range_bpc = self.out_of_range_bpc.saturating_add(other.out_of_range_bpc);
        self.missing_colour_space = self
            .missing_colour_space
            .saturating_add(other.missing_colour_space);
        self.colour_space_on_mask = self
            .colour_space_on_mask
            .saturating_add(other.colour_space_on_mask);
        self.af = self.af.saturating_add(other.af);
    }

    /// Prints the census.
    fn report(&self, documents: usize, opened: usize) {
        println!("documents named:  {documents}");
        println!("documents opened: {opened}");
        println!("image dictionaries: {}", self.images);
        println!("  of them masks:    {}", self.masks);
        println!("  of them JPXDecode:{}", self.jpx);
        println!("malformed required entries:");
        println!(
            "  /Width absent or not a positive integer:  {}",
            self.bad_width
        );
        println!(
            "  /Height absent or not a positive integer: {}",
            self.bad_height
        );
        println!(
            "  /BitsPerComponent required and absent:    {}",
            self.missing_bpc
        );
        println!(
            "  /BitsPerComponent out of {{1,2,4,8,16}}:     {}",
            self.out_of_range_bpc
        );
        println!(
            "  /ColorSpace required and absent:          {}",
            self.missing_colour_space
        );
        println!(
            "  /ColorSpace present on an image mask:     {}",
            self.colour_space_on_mask
        );
        println!("images stating an /AF array:        {}", self.af);
    }
}

/// The last codec of the `/Filter` chain, or `None` when there is none.
fn last_codec(document: &Document, dict: &Dictionary) -> Option<Vec<u8>> {
    match document.get_key(dict, "Filter") {
        Object::Name(name) => Some(name.as_bytes().to_vec()),
        Object::Array(items) => items
            .last()
            .map(|item| document.resolve(item))
            .and_then(|item| item.as_name().map(|name| name.as_bytes().to_vec())),
        _ => None,
    }
}

/// `/Width` or `/Height` read as Table 87 types it, `None` when it names no grid.
fn positive_integer(document: &Document, dict: &Dictionary, key: &str) -> Option<u32> {
    match document.get_key(dict, key) {
        Object::Integer(value) => u32::try_from(value).ok().filter(|count| *count > 0),
        _ => None,
    }
}

/// Every image dictionary in `document`, folded into `tally`.
fn walk(document: &Document, tally: &mut Tally) {
    for number in document.xref().object_numbers() {
        let object = document.get(ObjectId {
            number,
            generation: 0,
        });
        let Some(stream) = object.as_stream() else {
            continue;
        };
        let dict = &stream.dict;
        if document
            .get_key(dict, "Subtype")
            .as_name()
            .is_none_or(|name| name.as_bytes() != b"Image")
        {
            continue;
        }
        tally.add(document, dict);
    }
}

fn main() {
    let paths: Vec<PathBuf> = std::env::args_os().skip(1).map(PathBuf::from).collect();
    let mut total = Tally::default();
    let mut opened = 0usize;
    for path in &paths {
        let Ok(bytes) = std::fs::read(path) else {
            continue;
        };
        let Ok(document) = Document::open(bytes) else {
            continue;
        };
        opened = opened.saturating_add(1);
        let mut tally = Tally::default();
        walk(&document, &mut tally);
        total.absorb(&tally);
    }
    total.report(paths.len(), opened);
}
