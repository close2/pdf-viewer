//! Which of OpenType's vertical features the faces *on this machine* actually state, and how
//! much of a character collection's vertical forms the best of them can supply.
//!
//! # Why this exists
//!
//! [`pdf_font::vertical`] recovers the shape a vertical `CMap`'s CID named (ISO 32000-2
//! §9.7.5.1's NOTE) by asking the substituted face's `vert` and `vrt2` features. Where the face
//! states neither for a glyph, the page draws the character upright and
//! `pdf_model::content::Shortfall::without_a_vertical_form` counts it (ADR 0764). Two questions
//! followed from that and both were *written down* rather than measured:
//!
//! - whether a **second registered feature** is worth consulting for the forms `vert` misses;
//! - whether any face here states a `vert` lookup that is **not** a single substitution, which
//!   `doc/todo/21` records as the condition under which that refusal would be revisited.
//!
//! Both are claims about this machine's font catalogue and nothing else — §9.5's NOTE 5 puts the
//! choice of face outside the standard — so both decay the moment a font is installed. This is
//! the command that answers them, which is what `CLAUDE.md` asks for in place of a sentence.
//!
//! # What it prints
//!
//! One line per face that can draw あ — the character [`pdf_font::substitute::installed_covering`]
//! judges an Adobe-Japan1 substitute by — carrying every vertical feature tag the face states,
//! the `GSUB` lookup types under `vert`/`vrt2`, and how many of Adobe-Japan1's own vertical forms
//! the face supplies. That last number is the collection's rather than a list of ours: Table 116
//! publishes `UniJIS-UCS2-H` and its vertical counterpart, and a character the two send to
//! different CIDs is one the collection says has a distinct vertical form
//! ([`pdf_font::predefined::is_vertical_form`] asks the same pair the same way).
//!
//! ```sh
//! cargo run --release -p pdf-font --example vertical_feature_census
//! ```

#![allow(clippy::print_stdout, missing_docs)]
#![allow(
    clippy::arithmetic_side_effects,
    reason = "counters over this machine's font files and one 16-bit code space; a measurement \
              rather than a shipped path"
)]

use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};

use skrifa::MetadataProvider;
use skrifa::raw::TableProvider;
use skrifa::raw::tables::gsub::{SingleSubst, SubstitutionSubtables};
use skrifa::{FontRef, Tag};

/// The vertical features OpenType registers, in the order its feature-tag registry lists them.
///
/// `vert` and `vrt2` are the two [`pdf_font::vertical`] reads. The other four are here because
/// the question this census answers is whether one of *them* would supply what `vert` misses,
/// and a census that only looked for the two already read could not answer it.
const VERTICAL_TAGS: [&str; 6] = ["valt", "vert", "vhal", "vkna", "vpal", "vrt2"];

/// The character a face has to draw to stand in for Adobe-Japan1, as `substituted::script_sample`
/// picks it.
const JAPAN1_SAMPLE: char = '\u{3042}';

/// Table 116's Unicode `CMap` pair for Adobe-Japan1, by the table's own names.
const JAPAN1_PAIR: (&str, &str) = ("UniJIS-UCS2-H", "UniJIS-UCS2-V");

fn main() {
    let forms = collection_forms();
    println!(
        "Adobe-Japan1 states {} character(s) with a distinct vertical form ({} against {})",
        forms.len(),
        JAPAN1_PAIR.0,
        JAPAN1_PAIR.1
    );

    let files = catalogue();
    println!(
        "{} font file(s) in the directories this crate walks",
        files.len()
    );

    let mut covering = 0_usize;
    let mut with_a_vertical_feature = 0_usize;
    let mut tags_present: BTreeMap<&'static str, usize> = BTreeMap::new();
    let mut lookup_types: BTreeSet<&'static str> = BTreeSet::new();
    let mut lines: Vec<String> = Vec::new();

    for path in &files {
        let Ok(bytes) = std::fs::read(path) else {
            continue;
        };
        let Ok(font) = FontRef::new(&bytes) else {
            continue;
        };
        let charmap = font.charmap();
        if charmap.map(JAPAN1_SAMPLE).is_none() {
            continue;
        }
        covering += 1;

        let read = Vertical::read(&font);
        if read.by_tag.is_empty() {
            continue;
        }
        with_a_vertical_feature += 1;
        for tag in read.by_tag.keys() {
            *tags_present.entry(tag).or_default() += 1;
        }
        lookup_types.extend(read.lookup_shapes.iter().copied());

        let supplied = forms
            .iter()
            .filter(|character| {
                charmap.map(**character).is_some_and(|glyph| {
                    let glyph = u16::try_from(glyph.to_u32()).unwrap_or(u16::MAX);
                    read.by_tag
                        .values()
                        .any(|substitutions| substitutions.contains_key(&glyph))
                })
            })
            .count();
        let tags: Vec<String> = read
            .by_tag
            .iter()
            .map(|(tag, substitutions)| format!("{tag}({})", substitutions.len()))
            .collect();
        lines.push(format!(
            "  {:>6} cmap  {:<38}  {:<24}  {supplied} of {} Adobe-Japan1 form(s)",
            charmap.mappings().count(),
            path.file_name().unwrap_or_default().to_string_lossy(),
            tags.join(" "),
            forms.len()
        ));
    }

    println!(
        "\n{covering} face(s) can draw {JAPAN1_SAMPLE:?}; {with_a_vertical_feature} of them state \
         a vertical feature:"
    );
    for line in &lines {
        println!("{line}");
    }

    println!("\nvertical feature tags present on this machine, of the six OpenType registers:");
    for tag in VERTICAL_TAGS {
        println!("  {tag}: {}", tags_present.get(tag).copied().unwrap_or(0));
    }
    println!(
        "\nGSUB lookup shapes under vert/vrt2 here: {}",
        if lookup_types.is_empty() {
            "none".to_owned()
        } else {
            lookup_types.iter().copied().collect::<Vec<_>>().join(", ")
        }
    );
}

/// Every character Adobe-Japan1's own `CMap` pair gives a distinct vertical form.
///
/// The same comparison [`pdf_font::predefined::is_vertical_form`] makes, run over the whole of the
/// UCS-2 code space rather than for one character. §9.7.5.2 puts a name containing `UCS2` in
/// UCS-2 encoding, so the pair is keyed by a 16-bit code and this loop is its entire domain.
fn collection_forms() -> Vec<char> {
    let (Some(horizontal), Some(vertical)) = (
        pdf_font::predefined::cmap(JAPAN1_PAIR.0),
        pdf_font::predefined::cmap(JAPAN1_PAIR.1),
    ) else {
        return Vec::new();
    };
    let mut found = Vec::new();
    for code in 0..=u16::MAX {
        let Some(character) = char::from_u32(u32::from(code)) else {
            continue;
        };
        let bytes = code.to_be_bytes();
        let upright = horizontal.cid(horizontal.next_code(&bytes));
        let downward = vertical.cid(vertical.next_code(&bytes));
        if downward.is_some() && upright.is_some() && downward != upright {
            found.push(character);
        }
    }
    found
}

/// What one face states about vertical setting.
#[derive(Default)]
struct Vertical {
    /// Every registered vertical tag the face states, with its single substitutions.
    by_tag: BTreeMap<&'static str, BTreeMap<u16, u16>>,
    /// Which `GSUB` lookup shapes appear under `vert` and `vrt2`, named rather than numbered.
    lookup_shapes: BTreeSet<&'static str>,
}

impl Vertical {
    fn read(font: &FontRef) -> Self {
        let mut read = Self::default();
        let Ok(gsub) = font.gsub() else {
            return read;
        };
        let (Ok(features), Ok(lookups)) = (gsub.feature_list(), gsub.lookup_list()) else {
            return read;
        };
        for wanted in VERTICAL_TAGS {
            let tag = Tag::new(wanted.as_bytes().try_into().unwrap_or(b"vert"));
            for record in features
                .feature_records()
                .iter()
                .filter(|record| record.feature_tag() == tag)
            {
                let Ok(feature) = record.feature(features.offset_data()) else {
                    continue;
                };
                let entry = read.by_tag.entry(wanted).or_default();
                for index in feature.lookup_list_indices() {
                    let Ok(lookup) = lookups.lookups().get(usize::from(index.get())) else {
                        continue;
                    };
                    let Ok(subtables) = lookup.subtables() else {
                        continue;
                    };
                    if matches!(wanted, "vert" | "vrt2") {
                        read.lookup_shapes.insert(shape(&subtables));
                    }
                    let SubstitutionSubtables::Single(subtables) = subtables else {
                        continue;
                    };
                    for subtable in subtables.iter().flatten() {
                        collect_single(&subtable, entry);
                    }
                }
            }
        }
        read
    }
}

/// The name of a `GSUB` lookup's shape, which is what says whether it is one glyph for one glyph.
fn shape(subtables: &SubstitutionSubtables<'_>) -> &'static str {
    match subtables {
        SubstitutionSubtables::Single(_) => "single",
        SubstitutionSubtables::Multiple(_) => "multiple",
        SubstitutionSubtables::Alternate(_) => "alternate",
        SubstitutionSubtables::Ligature(_) => "ligature",
        SubstitutionSubtables::Contextual(_) => "contextual",
        SubstitutionSubtables::ChainContextual(_) => "chain contextual",
        SubstitutionSubtables::Reverse(_) => "reverse chain",
        SubstitutionSubtables::EmptyExtension => "empty extension",
    }
}

/// One single-substitution subtable, read exactly as [`pdf_font::vertical`] reads it.
fn collect_single(subtable: &SingleSubst<'_>, into: &mut BTreeMap<u16, u16>) {
    match subtable {
        SingleSubst::Format1(table) => {
            let Ok(coverage) = table.coverage() else {
                return;
            };
            let delta = table.delta_glyph_id();
            for glyph in coverage.iter() {
                let _ = into.insert(
                    glyph.to_u16(),
                    glyph.to_u16().wrapping_add(delta.cast_unsigned()),
                );
            }
        }
        SingleSubst::Format2(table) => {
            let Ok(coverage) = table.coverage() else {
                return;
            };
            let substitutes = table.substitute_glyph_ids();
            for (at, glyph) in coverage.iter().enumerate() {
                let Some(substitute) = substitutes.get(at) else {
                    break;
                };
                let _ = into.insert(glyph.to_u16(), substitute.get().to_u16());
            }
        }
    }
}

/// The font files this machine offers, in the directories `substitute::font_directories` walks.
///
/// Re-derived here rather than borrowed, because that function is private and a census that
/// asked the chooser what the chooser thinks would measure the reader rather than the machine —
/// the same reason `hollow_glyph_census` reads a `loca` by hand.
fn catalogue() -> Vec<PathBuf> {
    /// Bounds the walk the way the chooser's own does.
    const MAX_DEPTH: u32 = 8;

    fn walk(dir: &Path, depth: u32, into: &mut Vec<PathBuf>) {
        if depth == 0 {
            return;
        }
        let Ok(entries) = std::fs::read_dir(dir) else {
            return;
        };
        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_dir() {
                walk(&path, depth - 1, into);
            } else if path.extension().is_some_and(|extension| {
                let extension = extension.to_string_lossy().to_ascii_lowercase();
                matches!(extension.as_str(), "ttf" | "otf" | "ttc" | "otc")
            }) {
                into.push(path);
            }
        }
    }

    let mut files = Vec::new();
    for dir in [
        "/usr/share/fonts",
        "/usr/local/share/fonts",
        "/usr/share/X11/fonts",
        "/System/Library/Fonts",
        "/Library/Fonts",
    ] {
        walk(Path::new(dir), MAX_DEPTH, &mut files);
    }
    if let Some(home) = std::env::var_os("HOME") {
        let home = PathBuf::from(home);
        walk(&home.join(".local/share/fonts"), MAX_DEPTH, &mut files);
        walk(&home.join(".fonts"), MAX_DEPTH, &mut files);
        walk(&home.join("Library/Fonts"), MAX_DEPTH, &mut files);
    }
    files.sort();
    files.dedup();
    files
}
