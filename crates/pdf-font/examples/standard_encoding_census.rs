//! Which fonts name `StandardEncoding`, and which of them draw differently once the name is read
//! as Table D.1 requires.
//!
//! ISO 32000-2 §D.1, Table D.1:
//!
//! > PDF processors shall not have a predefined encoding named StandardEncoding.
//!
//! Table 109 lists the names an `/Encoding` may hold and `StandardEncoding` is not one of them,
//! so a font naming it — as `/Encoding` or as an encoding dictionary's
//! `/BaseEncoding` — has stated no base encoding, and §9.6.5's built-in encoding of its program
//! stands. Where that built-in encoding is the standard one, nothing a page shows moves; where a
//! program carries its own, the codes it places differently draw other glyphs. This is the
//! instrument behind `doc/questions/Q1082`: it loads every such simple font twice — with the
//! column the name named spelled out as `/Differences`, and with the name taken out — and counts
//! the single-byte codes whose glyph differs; and a third time as written, the reading the
//! binary implements, which for a `TrueType` font is not always the name's absence.
//!
//! ```sh
//! cargo run --release -p pdf-font --example standard_encoding_census -- doc
//! ```
//!
//! Type 3 fonts are not counted: they have no program, so no built-in encoding to stand.

#![expect(
    clippy::print_stdout,
    reason = "an example whose whole product is a census"
)]

use pdf_font::encoding::BaseEncoding;
use pdf_font::{Code, LoadedFont};
use pdf_syntax::object::{Dictionary, Name};
use pdf_syntax::{Document, FileBytes, Object};

/// Where the name was found.
#[derive(Clone, Copy)]
enum Site {
    /// `/Encoding /StandardEncoding`.
    AsEncoding,
    /// `/Encoding << /BaseEncoding /StandardEncoding … >>`.
    AsBaseEncoding,
}

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
    let mut naming = 0usize;
    let mut fonts = 0usize;
    let mut unloadable = 0usize;
    let mut moved: Vec<String> = Vec::new();

    for path in &files {
        let Ok(bytes) = FileBytes::on_disk(path) else {
            continue;
        };
        let Ok(document) = Document::open(bytes) else {
            continue;
        };
        documents = documents.saturating_add(1);
        let mut named_here = false;
        for number in document.xref().object_numbers() {
            let object = document.get(document.defined_id(number));
            let Some(font) = object.as_dict() else {
                continue;
            };
            let Some(site) = names_standard(&document, font) else {
                continue;
            };
            named_here = true;
            fonts = fonts.saturating_add(1);
            let as_the_table =
                LoadedFont::load(&document, &spelled_out(&document, font, site), "F");
            let as_absent =
                LoadedFont::load(&document, &without_the_name(&document, font, site), "F");
            let as_written = LoadedFont::load(&document, font, "F");
            let (Ok(as_the_table), Ok(as_absent), Ok(as_written)) =
                (as_the_table, as_absent, as_written)
            else {
                unloadable = unloadable.saturating_add(1);
                continue;
            };
            let differing: Vec<u8> = (0u8..=255)
                .filter(|code| !BaseEncoding::Standard.glyph_name(*code).is_empty())
                .filter(|code| {
                    let code = Code::single_byte(*code);
                    as_the_table.glyph_index(code) != as_absent.glyph_index(code)
                        || as_the_table.glyph_index(code) != as_written.glyph_index(code)
                })
                .collect();
            if !differing.is_empty() {
                let base = match document.get_key(font, "BaseFont") {
                    Object::Name(name) => String::from_utf8_lossy(name.as_bytes()).into_owned(),
                    _ => String::new(),
                };
                let subtype = match document.get_key(font, "Subtype") {
                    Object::Name(name) => String::from_utf8_lossy(name.as_bytes()).into_owned(),
                    _ => String::new(),
                };
                let site = match site {
                    Site::AsEncoding => "/Encoding",
                    Site::AsBaseEncoding => "/BaseEncoding",
                };
                moved.push(format!(
                    "{} object {number}: {subtype} {base} via {site}: {} code(s) differ, first {:?}",
                    path.display(),
                    differing.len(),
                    &differing[..differing.len().min(12)]
                ));
            }
        }
        if named_here {
            naming = naming.saturating_add(1);
        }
    }

    println!("{} files, {documents} opened", files.len());
    println!(
        "{naming} documents hold {fonts} simple font(s) naming StandardEncoding; {unloadable} \
         did not load one way or the other"
    );
    println!("{} font(s) whose glyphs move:", moved.len());
    for line in &moved {
        println!("  {line}");
    }
}

/// Whether `font` is a simple font with a program, naming `StandardEncoding`, and where.
fn names_standard(document: &Document, font: &Dictionary) -> Option<Site> {
    let subtype = document.get_key(font, "Subtype");
    if !matches!(
        subtype.as_name()?.as_bytes(),
        b"Type1" | b"MMType1" | b"TrueType"
    ) {
        return None;
    }
    let encoding = document.get_key(font, "Encoding");
    if encoding
        .as_name()
        .is_some_and(|name| name.as_bytes() == b"StandardEncoding")
    {
        return Some(Site::AsEncoding);
    }
    let dict = encoding.as_dict()?;
    document
        .get_key(dict, "BaseEncoding")
        .as_name()
        .is_some_and(|name| name.as_bytes() == b"StandardEncoding")
        .then_some(Site::AsBaseEncoding)
}

/// `font` with the name taken out: no `/Encoding`, or an encoding dictionary with no
/// `/BaseEncoding`, written inline.
fn without_the_name(document: &Document, font: &Dictionary, site: Site) -> Dictionary {
    let mut font = font.clone();
    match site {
        Site::AsEncoding => {
            font.remove("Encoding");
        }
        Site::AsBaseEncoding => {
            if let Some(mut encoding) = document.get_key(&font, "Encoding").as_dict().cloned() {
                encoding.remove("BaseEncoding");
                font.insert(Name::new(&b"Encoding"[..]), Object::Dictionary(encoding));
            }
        }
    }
    font
}

/// `font` with the name replaced by what reading it as Table D.2's STD column meant: an encoding
/// dictionary with no base, whose `/Differences` give every code that column names its name,
/// followed by the font's own `/Differences` where it had any. Neither dictionary states the
/// name, so the census measures the two readings whichever one the binary implements; only the
/// codes the column names are compared, since a code it leaves empty is the program's in the
/// spelled-out dictionary and nothing's in the table reading.
fn spelled_out(document: &Document, font: &Dictionary, site: Site) -> Dictionary {
    let mut differences = Vec::new();
    for code in 0u8..=255 {
        let name = BaseEncoding::Standard.glyph_name(code);
        if !name.is_empty() {
            differences.push(Object::Integer(i64::from(code)));
            differences.push(Object::Name(Name::new(name.as_bytes())));
        }
    }
    let mut encoding = match site {
        Site::AsEncoding => Dictionary::new(),
        Site::AsBaseEncoding => document
            .get_key(font, "Encoding")
            .as_dict()
            .cloned()
            .unwrap_or_default(),
    };
    encoding.remove("BaseEncoding");
    if let Some(own) = document.get_key(&encoding, "Differences").as_array() {
        differences.extend(own.iter().cloned());
    }
    encoding.insert(Name::new(&b"Differences"[..]), Object::Array(differences));
    let mut font = font.clone();
    font.insert(Name::new(&b"Encoding"[..]), Object::Dictionary(encoding));
    font
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
