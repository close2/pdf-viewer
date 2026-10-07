//! Which face a font the document did not embed is drawn in, against what its font descriptor
//! says of it.
//!
//! ISO 32000-2 §9.8.1 gives the descriptor its role in substitution:
//!
//! > These font metrics provide information that enables a PDF processor to synthesise a
//! > substitute font or select a similar font when the font program is unavailable.
//!
//! and Table 120 and Table 121 state what "similar" can be measured by: `/FontWeight`, `/StemV`,
//! `/ItalicAngle`, `/FontStretch` and the `/Flags` bits `FixedPitch`, Serif and Italic. This walks
//! every font dictionary whose descriptor holds no `/FontFile`, `/FontFile2` or `/FontFile3` (and
//! every simple font with no descriptor, which §9.6.2.2's fourteen may omit), loads it as
//! `pdf-font` loads it for a page, and reads the chosen face's own `OS/2` and `post` tables. A
//! font counts as **disagreeing** where the face's weight class is more than one hundred from
//! the weight the descriptor states or implies (`pdf_font::substitute::Style::derive`, ADR 1441),
//! where its slope differs from `/ItalicAngle` ≠ 0 or the Italic flag, where its `post`
//! `isFixedPitch` differs from the `FixedPitch` flag, or where its serif classification (`OS/2`
//! `sFamilyClass` or PANOSE, where the face states one) differs from the Serif flag.
//!
//! One line per font goes to the file named by `SUBSTITUTION_CENSUS_TSV`, where it is set, so
//! that two runs can be compared font by font. Walks `doc/pdf.js/test/pdfs` and every `.pdf`
//! under `doc/corpora/`, or the directories named on the command line; an encrypted document is
//! opened with the password `tests/support/corpus_passwords.rs` publishes for it (ADR 1377).
//!
//! ```sh
//! RAYON_NUM_THREADS=4 tools/bounded.sh --lock --round <session> -- \
//!   cargo run --release -p pdf-model --example substitution_census
//! ```
#![expect(
    clippy::print_stdout,
    clippy::expect_used,
    clippy::arithmetic_side_effects,
    reason = "a measurement whose output is its purpose: it stops loudly where the corpus is \
              missing, and its counters over the corpus's fonts are far below what a usize counts"
)]

use std::collections::BTreeMap;
use std::fmt::Write as _;
use std::path::{Path, PathBuf};

use pdf_font::LoadedFont;
use pdf_font::substitute::{Family, Request, Style};
use pdf_syntax::{Dictionary, Document, Limits, Object, ObjectId};
use read_fonts::TableProvider;

/// The published passwords of the corpus's encrypted documents (ADR 1377).
#[path = "../tests/support/corpus_passwords.rs"]
#[expect(
    dead_code,
    reason = "the references' spelling of a password is the oracle's; this census hands a \
              password to this tree alone"
)]
mod corpus_passwords;

fn corpus() -> Vec<PathBuf> {
    let tree = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    let named: Vec<PathBuf> = std::env::args().skip(1).map(PathBuf::from).collect();
    let roots = if named.is_empty() {
        vec![tree.join("doc/pdf.js/test/pdfs"), tree.join("doc/corpora")]
    } else {
        named
    };
    let mut files = Vec::new();
    for root in roots {
        collect(&root, &mut files);
    }
    files.sort();
    files
}

/// Every `.pdf` under `root`, however deep.
fn collect(root: &Path, into: &mut Vec<PathBuf>) {
    let entries = std::fs::read_dir(root).expect("the corpus directory is on the disk");
    for path in entries.filter_map(|entry| entry.ok().map(|entry| entry.path())) {
        if path.is_dir() {
            collect(&path, into);
        } else if path.extension().is_some_and(|e| e == "pdf") {
            into.push(path);
        }
    }
}

/// Table 121's bits this census reads, numbered as §9.8.2 numbers them.
const FIXED_PITCH: i64 = 1 << 0;
const SERIF: i64 = 1 << 1;
const ITALIC: i64 = 1 << 6;

/// What a face states about itself.
#[derive(Clone)]
struct Face {
    name: String,
    weight: u16,
    italic: bool,
    width: u16,
    fixed: bool,
    serif: Option<bool>,
}

/// The compiled-in face `bytes` is, by content: [`pdf_font::standard::face`]'s answer is copied
/// into the loaded font, so it is found by comparing the programs.
fn compiled_in(bytes: &[u8]) -> Option<Face> {
    for family in [
        Family::SansSerif,
        Family::Serif,
        Family::Monospace,
        Family::Symbol,
        Family::ZapfDingbats,
    ] {
        for (bold, italic) in [(false, false), (true, false), (false, true), (true, true)] {
            let request = Request {
                family,
                bold,
                italic,
                standard: true,
            };
            let (face, _) = pdf_font::standard::face(request);
            if face == bytes {
                return Some(Face {
                    name: format!(
                        "compiled-in {family:?}{}{}",
                        if bold { " Bold" } else { "" },
                        if italic { " Italic" } else { "" }
                    ),
                    weight: if bold { 700 } else { 400 },
                    italic,
                    width: 5,
                    fixed: family == Family::Monospace,
                    serif: match family {
                        Family::Serif => Some(true),
                        Family::SansSerif | Family::Monospace => Some(false),
                        _ => None,
                    },
                });
            }
        }
    }
    None
}

/// A machine face's own statement of its style.
fn machine(bytes: &[u8]) -> Option<Face> {
    let font = read_fonts::FontRef::new(bytes).ok()?;
    let os2 = font.os2().ok()?;
    let fixed = font.post().is_ok_and(|post| post.is_fixed_pitch() != 0);
    let selection = os2.fs_selection().bits();
    let italic = selection & 1 != 0 || selection & (1 << 9) != 0;
    let class = os2.s_family_class().to_be_bytes();
    let mut panose = [0_u8; 12];
    panose[..2].copy_from_slice(&class);
    panose[2..].copy_from_slice(os2.panose_10());
    let serif = match class[0] {
        1..=5 | 7 => Some(true),
        8 => Some(false),
        _ => pdf_font::panose::Panose::read(&panose).and_then(pdf_font::panose::Panose::is_serif),
    };
    let name = font
        .name()
        .ok()
        .and_then(|table| {
            table.name_record().iter().find_map(|record| {
                (record.name_id() == read_fonts::types::NameId::FULL_NAME)
                    .then(|| record.string(table.string_data()).ok())
                    .flatten()
                    .map(|string| string.chars().collect::<String>())
            })
        })
        .unwrap_or_default();
    Some(Face {
        name,
        weight: os2.us_weight_class(),
        italic,
        width: os2.us_width_class(),
        fixed,
        serif,
    })
}

/// What the descriptor says, as far as this census compares it.
#[derive(Default)]
struct Stated {
    flags: Option<i64>,
    font_weight: Option<f64>,
    stem_v: Option<f64>,
    italic_angle: Option<f64>,
    stretch: Option<String>,
}

fn stated(document: &Document, descriptor: Option<&Dictionary>) -> Stated {
    let Some(descriptor) = descriptor else {
        return Stated::default();
    };
    let number = |key: &str| document.get_key(descriptor, key).as_number();
    Stated {
        flags: match document.get_key(descriptor, "Flags") {
            Object::Integer(value) => Some(value),
            _ => None,
        },
        font_weight: number("FontWeight"),
        stem_v: number("StemV"),
        italic_angle: number("ItalicAngle"),
        stretch: document
            .get_key(descriptor, "FontStretch")
            .as_name()
            .map(|name| String::from_utf8_lossy(name.as_bytes()).into_owned()),
    }
}

/// Whether a descriptor holds a font program.
fn embeds(document: &Document, descriptor: &Dictionary) -> bool {
    ["FontFile", "FontFile2", "FontFile3"]
        .iter()
        .any(|key| !matches!(document.get_key(descriptor, key), Object::Null))
}

/// A font dictionary's subtype and the dictionary its descriptor hangs from: the font's own, or
/// a composite font's descendant (§9.7.1). `None` for a Type 3 font or anything malformed.
fn described(document: &Document, dict: &Dictionary) -> Option<(Vec<u8>, Dictionary)> {
    let subtype = document.get_key(dict, "Subtype");
    let subtype = subtype.as_name().map(|name| name.as_bytes().to_vec())?;
    let described = match subtype.as_slice() {
        b"Type1" | b"MMType1" | b"TrueType" => dict.clone(),
        b"Type0" => {
            let descendants = document.get_key(dict, "DescendantFonts");
            descendants
                .as_array()
                .and_then(|array| array.first().map(|item| document.resolve(item)))
                .and_then(|item| item.as_dict().cloned())?
        }
        _ => return None,
    };
    Some((subtype, described))
}

#[derive(Default)]
struct Census {
    documents: usize,
    fonts: usize,
    with_descriptor: usize,
    compiled_in: usize,
    weight: usize,
    weight_stated: usize,
    slope: usize,
    spacing: usize,
    serif: usize,
    serif_unknown: usize,
    any: usize,
    faces: BTreeMap<String, usize>,
    lines: String,
}

impl Census {
    fn count(&mut self, file: &str, document: &Document) {
        self.documents += 1;
        for number in document.xref().object_numbers() {
            let object = document.get(ObjectId {
                number,
                generation: 0,
            });
            let Some(dict) = object.as_dict() else {
                continue;
            };
            if document
                .get_key(dict, "Type")
                .as_name()
                .is_none_or(|name| name.as_bytes() != b"Font")
            {
                continue;
            }
            let Some((subtype, described)) = described(document, dict) else {
                continue;
            };
            let descriptor = document.get_key(&described, "FontDescriptor");
            let descriptor = descriptor.as_dict();
            if descriptor.is_some_and(|d| embeds(document, d)) {
                continue;
            }
            let base = document
                .get_key(&described, "BaseFont")
                .as_name()
                .map(|name| String::from_utf8_lossy(name.as_bytes()).into_owned())
                .unwrap_or_default();
            let Ok(font) = LoadedFont::load(document, dict, &base) else {
                continue;
            };
            let Some(program) = font.substitute_program() else {
                continue;
            };
            let Some(face) = compiled_in(program).or_else(|| machine(program)) else {
                continue;
            };
            self.fonts += 1;
            *self.faces.entry(face.name.clone()).or_default() += 1;
            if face.name.starts_with("compiled-in") {
                self.compiled_in += 1;
            }
            let said = stated(document, descriptor);
            let style = Style::derive(document, &described, descriptor);
            let mut disagrees = Vec::new();
            if descriptor.is_some() {
                self.with_descriptor += 1;
                if (i32::from(face.weight) - i32::from(style.weight)).abs() > 100 {
                    self.weight += 1;
                    disagrees.push("weight");
                    if said.font_weight.is_some() {
                        self.weight_stated += 1;
                    }
                }
                let flags = said.flags.unwrap_or(0);
                let sloped =
                    said.italic_angle.is_some_and(|angle| angle != 0.0) || flags & ITALIC != 0;
                if sloped != face.italic {
                    self.slope += 1;
                    disagrees.push("slope");
                }
                if (flags & FIXED_PITCH != 0) != face.fixed {
                    self.spacing += 1;
                    disagrees.push("spacing");
                }
                match face.serif {
                    Some(serif) if serif != (flags & SERIF != 0) => {
                        self.serif += 1;
                        disagrees.push("serif");
                    }
                    Some(_) => {}
                    None => self.serif_unknown += 1,
                }
                if !disagrees.is_empty() {
                    self.any += 1;
                }
            }
            let show = |value: Option<f64>| value.map_or_else(|| "-".to_owned(), |v| v.to_string());
            let _ = writeln!(
                self.lines,
                "{file}\t{base}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}",
                String::from_utf8_lossy(&subtype),
                said.flags.map_or_else(|| "-".to_owned(), |f| f.to_string()),
                show(said.font_weight),
                show(said.stem_v),
                show(said.italic_angle),
                said.stretch.as_deref().unwrap_or("-"),
                style.weight,
                style.width,
                face.name,
                face.weight,
                face.italic,
                face.width,
                face.fixed,
                face.serif
                    .map_or("-", |serif| if serif { "serif" } else { "sans" }),
                disagrees.join(","),
            );
        }
    }
}

fn main() {
    let mut census = Census::default();
    for path in corpus() {
        let Ok(bytes) = std::fs::read(&path) else {
            continue;
        };
        let name = path
            .file_name()
            .map(|name| name.to_string_lossy().into_owned())
            .unwrap_or_default();
        let password = corpus_passwords::corpus_password(&name).map_or("", |known| known.password);
        let Ok(document) = Document::open_with_password(bytes, Limits::default(), password) else {
            continue;
        };
        census.count(&name, &document);
    }
    if let Some(out) = std::env::var_os("SUBSTITUTION_CENSUS_TSV") {
        std::fs::write(out, &census.lines).expect("the line file is writable");
    }
    println!(
        "{} documents; {} substituted fonts, {} with a descriptor, {} drawn from a compiled-in face",
        census.documents, census.fonts, census.with_descriptor, census.compiled_in
    );
    println!("of those with a descriptor, the face disagrees on:");
    println!(
        "  weight (more than one step from the descriptor's): {} ({} where /FontWeight is stated)",
        census.weight, census.weight_stated
    );
    println!(
        "  slope (/ItalicAngle or the Italic flag):          {}",
        census.slope
    );
    println!(
        "  spacing (the FixedPitch flag):                   {}",
        census.spacing
    );
    println!(
        "  serif-ness (the Serif flag):                      {} ({} faces state no class)",
        census.serif, census.serif_unknown
    );
    println!(
        "  any of the four:                                  {}",
        census.any
    );
    println!("faces drawn, most used first:");
    let mut faces: Vec<_> = census.faces.into_iter().collect();
    faces.sort_by(|a, b| b.1.cmp(&a.1).then_with(|| a.0.cmp(&b.0)));
    for (face, count) in faces.iter().take(25) {
        println!("  {count:>5}  {face}");
    }
}
