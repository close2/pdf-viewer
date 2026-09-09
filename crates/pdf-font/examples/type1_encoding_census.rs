//! How many bare Type 1 programs claim to encode codes their own `/Encoding` array never names.
//!
//! The instrument behind ADR 0932. `read_fonts::ps::type1` builds a custom encoding as a vector
//! **pre-filled with `GlyphId::NOTDEF`**, so `Encoding::map` answers `Some(0)` for every code in
//! the array's length — assigned or not — and a code the program never encoded selected glyph 0
//! and drew the designer's `.notdef`, commonly a filled box, where nothing should be drawn.
//!
//! `doc/todo/53` recorded that defect without a witness and said what would change the answer: a
//! page where an unencoded code draws a box. This counts the population that could hold one,
//! which is the honest first question — a defect no document carries is still a defect
//! (`CLAUDE.md`'s "a count that does not move is not evidence that nothing happened"), but its
//! size decides what a fix is worth.
//!
//! Two numbers per program, and the gap between them is the finding: how many codes the resolved
//! map claims, and how many the array in the cleartext header actually assigns.
//!
//! ```sh
//! cargo run --release -p pdf-font --example type1_encoding_census -- doc doc/pdf.js
//! ```
//!
//! A program with a *predefined* encoding is counted apart and is not affected: that form
//! resolves through the charstring names and already answers `None` for a code it does not map.

#![expect(
    clippy::print_stdout,
    reason = "an example whose whole product is a census"
)]

use std::collections::BTreeMap;

use pdf_syntax::{Document, FileBytes, Object};
use skrifa::raw::ps::type1::Type1Font;

fn main() {
    let roots: Vec<String> = std::env::args().skip(1).collect();
    let roots = if roots.is_empty() {
        vec!["doc".to_owned()]
    } else {
        roots
    };
    let mut files = Vec::new();
    for root in &roots {
        collect(std::path::Path::new(root), &mut files);
    }
    files.sort();

    let mut documents = 0usize;
    let mut programs = 0usize;
    let mut predefined = 0usize;
    let mut custom = 0usize;
    let mut overclaiming: BTreeMap<String, (usize, usize)> = BTreeMap::new();
    let mut codes = 0usize;

    for path in &files {
        let Ok(bytes) = FileBytes::on_disk(path) else {
            continue;
        };
        let Ok(document) = Document::open(bytes) else {
            continue;
        };
        documents = documents.saturating_add(1);
        for number in document.xref().object_numbers() {
            let object = document.get(pdf_syntax::ObjectId::new(number, 0));
            let Some(stream) = object.as_stream() else {
                continue;
            };
            // ISO 32000-2 §9.9's Table 125: a bare Type 1 program is the value of `/FontFile`
            // and is the only font program that states `/Length1` with a cleartext section.
            if matches!(document.get_key(&stream.dict, "Length1"), Object::Null) {
                continue;
            }
            let Some(data) = document.decoded_stream_data(stream) else {
                continue;
            };
            let Ok(font) = Type1Font::new(&data) else {
                continue;
            };
            programs = programs.saturating_add(1);
            let Some(encoding) = font.encoding() else {
                continue;
            };
            if encoding.predefined().is_some() {
                predefined = predefined.saturating_add(1);
                continue;
            }
            custom = custom.saturating_add(1);
            let claimed = (0u16..=255)
                .filter_map(|code| u8::try_from(code).ok())
                .filter(|code| encoding.map(*code).is_some())
                .count();
            let Ok(program) = pdf_font::type1::Program::parse(&data) else {
                continue;
            };
            let Ok(keyed) = program.code_to_glyph() else {
                continue;
            };
            let assigned = keyed.builtin.iter().filter(|slot| slot.is_some()).count();
            if claimed > assigned {
                codes = codes.saturating_add(claimed.saturating_sub(assigned));
                let name = path
                    .file_name()
                    .unwrap_or_default()
                    .to_string_lossy()
                    .into_owned();
                let entry = overclaiming.entry(name).or_insert((0, 0));
                entry.0 = entry.0.saturating_add(1);
                entry.1 = entry.1.saturating_add(claimed.saturating_sub(assigned));
            }
        }
    }

    println!("{} files, {documents} opened", files.len());
    println!("{programs} bare Type 1 programs: {predefined} predefined encoding, {custom} custom");
    println!(
        "{} documents hold a program whose resolved map claims a code its array never assigns, \
         {codes} such codes in all",
        overclaiming.len()
    );
    for (name, (found, extra)) in overclaiming.iter().take(20) {
        println!("  {name}: {found} program(s), {extra} codes");
    }
}

/// Every `.pdf` under `dir`, recursively.
fn collect(dir: &std::path::Path, out: &mut Vec<std::path::PathBuf>) {
    let Ok(entries) = std::fs::read_dir(dir) else {
        return;
    };
    for entry in entries.flatten() {
        let path = entry.path();
        if path.is_dir() {
            collect(&path, out);
        } else if path.extension().is_some_and(|ext| ext == "pdf") {
            out.push(path);
        }
    }
}
