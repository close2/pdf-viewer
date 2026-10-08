//! Which colour specifications a set of documents' `JPXDecode` images state, per T.801 method and
//! enumeration, and how many of them §7.4.9 lets decide anything.
//!
//! ISO 32000-2 §7.4.9 asks a reader to "support the JPX baseline set of enumerated colour spaces",
//! and three of them — e-sRGB (20), e-sYCC (24) and CIE Jab (19) — are defined by texts this
//! project does not hold; CIE Lab (14) is drawn under whichever illuminant it states (ADR 1713),
//! and is counted here by illuminant for the same reason as the others. This is the
//! command that says whether any document in reach states one where it decides the colour: a
//! `/ColorSpace` on the image dictionary sets every `colr` box aside ("the colour space
//! specifications in the JPEG 2000 data shall be ignored"), so each box is counted twice — once
//! overall, once among images with no `/ColorSpace`.
//!
//! Only the headers are read (`pdf_model::jpeg2000::Headers`), never a sample.
//!
//! ```sh
//! cargo run --release -p pdf-model --example jpx_colour_census -- <file.pdf>…
//! cargo run --release -p pdf-model --example jpx_colour_census -- --list <paths.txt>
//! ```
//!
//! `--list` reads the paths one per line, for a population too large for one argument list.
#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic, missing_docs)]
#![allow(clippy::print_stdout, clippy::print_stderr)]
#![allow(
    clippy::arithmetic_side_effects,
    reason = "counters over a set of documents; a census rather than a shipped path"
)]

use std::collections::{BTreeMap, BTreeSet};

use pdf_model::jpeg2000::{ColourSpecification, Headers, Illuminant};
use pdf_syntax::{Document, Object, ObjectId};

/// `RL`, `OL`, `RA`, `OA`, `RB` and `OB`, four bytes each, before `IL` (T.801 Table M.30).
const LAB_RANGES: usize = 24;

/// What one `colr` box is, as a row label: method, and enumeration where it has one.
fn label(colour: &ColourSpecification<'_>) -> String {
    match (colour.method, colour.enumerated) {
        (ColourSpecification::ENUMERATED, Some(ColourSpecification::CIELAB)) => {
            let illuminant = colour.cielab_illuminant().unwrap_or(Illuminant::D50);
            let how = if colour.parameters.len() >= LAB_RANGES + 4 {
                "stated"
            } else {
                "by default"
            };
            if illuminant.white_point().is_some() {
                format!("enumerated 14 (CIELab), illuminant {illuminant:?} {how}")
            } else {
                format!("enumerated 14 (CIELab), illuminant {illuminant:?}, no white point")
            }
        }
        (ColourSpecification::ENUMERATED, Some(code)) => {
            let name = match code {
                12 => "CMYK",
                16 => "sRGB",
                17 => "greyscale",
                18 => "sYCC",
                19 => "CIEJab",
                20 => "e-sRGB",
                21 => "ROMM-RGB",
                24 => "e-sYCC",
                _ => "outside the JPX baseline",
            };
            format!("enumerated {code} ({name})")
        }
        (ColourSpecification::RESTRICTED_ICC, _) => "method 2 (restricted ICC)".to_owned(),
        (ColourSpecification::ANY_ICC, _) => "method 3 (any ICC)".to_owned(),
        (method, _) => format!("method {method}"),
    }
}

/// One row's counts.
#[derive(Default)]
struct Tally {
    /// `colr` boxes stating it, over every image.
    boxes: usize,
    /// Of those, on images with no `/ColorSpace`, where the box can decide the colour.
    deciding: usize,
    /// The documents it occurs in, by file name.
    documents: BTreeSet<String>,
}

fn main() {
    let mut rows: BTreeMap<String, Tally> = BTreeMap::new();
    let (mut documents, mut images, mut stated_space, mut bare, mut unread) = (0, 0, 0, 0, 0);
    let mut paths: Vec<String> = std::env::args().skip(1).collect();
    if paths.first().is_some_and(|first| first == "--list") {
        let list = paths.get(1).expect("--list takes a file of paths");
        paths = std::fs::read_to_string(list)
            .expect("the list is readable")
            .lines()
            .map(str::to_owned)
            .collect();
    }
    for path in paths {
        let name = std::path::Path::new(&path)
            .file_name()
            .map_or_else(|| path.clone(), |name| name.to_string_lossy().into_owned());
        let Ok(bytes) = std::fs::read(&path) else {
            continue;
        };
        let Ok(document) = Document::open(bytes) else {
            continue;
        };
        documents += 1;
        for number in document.xref().object_numbers() {
            let object = document.get(ObjectId::new(number, 0));
            let Some(stream) = object.as_stream() else {
                continue;
            };
            let Some(image) = document.image_stream(stream) else {
                continue;
            };
            if image.codec.as_deref() != Some(b"JPXDecode".as_slice()) {
                continue;
            }
            images += 1;
            let has_space = !matches!(document.get_key(&stream.dict, "ColorSpace"), Object::Null);
            stated_space += usize::from(has_space);
            let Ok(headers) = Headers::parse(&image.data) else {
                unread += 1;
                continue;
            };
            if headers.colour.is_empty() {
                bare += 1;
            }
            for colour in &headers.colour {
                let tally = rows.entry(label(colour)).or_default();
                tally.boxes += 1;
                tally.deciding += usize::from(!has_space);
                tally.documents.insert(name.clone());
            }
        }
    }

    println!(
        "{documents} document(s) opened, {images} JPXDecode image(s): {stated_space} with a \
         /ColorSpace, {bare} with no colr box (a bare codestream), {unread} whose headers did \
         not read"
    );
    println!();
    println!("  boxes  deciding  documents  specification");
    for (label, tally) in &rows {
        println!(
            "  {:>5}  {:>8}  {:>9}  {label}",
            tally.boxes,
            tally.deciding,
            tally.documents.len()
        );
    }
    for (label, tally) in &rows {
        let undefined = ["(CIEJab)", "(e-sRGB)", "(e-sYCC)", "no white point"]
            .iter()
            .any(|marker| label.contains(marker));
        if undefined {
            println!();
            println!("{label}, in:");
            for document in &tally.documents {
                println!("  {document}");
            }
        }
    }
}
