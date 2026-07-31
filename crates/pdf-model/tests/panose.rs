//! ISO 32000-2 §9.8.3.2's `/Style` `/Panose`, measured over the corpus.
//!
//! The question a substitution hint has to answer is not "can it be read" but **"does it change
//! anything"** — and there is one way to find out that does not involve guessing: derive each
//! font's substitution request twice, once from the descriptor as the file wrote it and once from
//! a copy with `/Style` taken out, and count the fonts where the two disagree.
//!
//! That is trap 8's method — measure a rule by breaking it deliberately — applied to a hint
//! rather than to a requirement, and it is only possible because §12.3.4's thumbnails needed
//! `Dictionary::remove` two sessions ago.

use std::path::{Path, PathBuf};

use pdf_font::substitute::Request;
use pdf_syntax::{Document, Object, ObjectId};

/// The pdf.js corpus, or `None` when the submodule is not checked out.
fn corpus() -> Option<Vec<PathBuf>> {
    let dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../doc/pdf.js/test/pdfs");
    let mut files: Vec<_> = std::fs::read_dir(dir)
        .ok()?
        .filter_map(|entry| entry.ok().map(|e| e.path()))
        .filter(|path| path.extension().is_some_and(|ext| ext == "pdf"))
        .collect();
    files.sort();
    Some(files)
}

/// What §9.8.3.2's classification is worth, in fonts whose request it changes.
#[expect(
    clippy::too_many_lines,
    reason = "one measurement over the corpus, whose counts read better together"
)]
#[test]
fn a_panose_number_changes_the_substitute_for_some_fonts_and_not_others() {
    let Some(files) = corpus() else {
        println!("skipped: the doc/pdf.js submodule is not checked out");
        return;
    };

    let mut with_a_style = 0usize;
    let mut readable = 0usize;
    let mut embedded = 0usize;
    let mut changed = Vec::new();
    let mut unreadable = Vec::new();
    for path in &files {
        let Ok(bytes) = std::fs::read(path) else {
            continue;
        };
        let Ok(document) = Document::open(bytes) else {
            continue;
        };
        let name = path.file_name().unwrap_or_default().to_string_lossy();
        for number in document.xref().object_numbers() {
            let object = document.get(ObjectId {
                number,
                generation: 0,
            });
            let Some(font) = object.as_dict() else {
                continue;
            };
            let is_font = document
                .get_key(font, "Type")
                .as_name()
                .is_some_and(|kind| kind.as_bytes() == b"Font");
            if !is_font {
                continue;
            }
            // A composite font's descriptor hangs off its descendant, which is where a
            // `/Style` lives: Table 122 is a *CIDFont's* descriptor.
            let descendant = document
                .get_key(font, "DescendantFonts")
                .as_array()
                .and_then(<[Object]>::first)
                .map(|entry| document.resolve(entry));
            let owner = descendant
                .as_ref()
                .and_then(|value| value.as_dict().cloned())
                .unwrap_or_else(|| font.clone());
            let Some(descriptor) = document
                .get_key(&owner, "FontDescriptor")
                .as_dict()
                .cloned()
            else {
                continue;
            };
            if descriptor.get("Style").is_none() {
                continue;
            }
            with_a_style = with_a_style.saturating_add(1);
            if ["FontFile", "FontFile2", "FontFile3"]
                .iter()
                .any(|key| descriptor.get(key).is_some())
            {
                // The clause's own limit on what this is for: a font that carries its program
                // is drawn from it, and no classification is consulted.
                embedded = embedded.saturating_add(1);
            }
            let style = document.get_key(&descriptor, "Style");
            match style
                .as_dict()
                .map(|style| document.get_key(style, "Panose"))
                .and_then(|value| value.as_string().map(<[u8]>::to_vec))
            {
                Some(bytes) if pdf_font::panose::Panose::read(&bytes).is_some() => {
                    readable = readable.saturating_add(1);
                }
                other => unreadable.push(format!("{name} object {number}: {other:?}")),
            }

            let stated = Request::derive(&document, font, Some(&descriptor));
            let mut without = descriptor.clone();
            without.remove("Style");
            let ignored = Request::derive(&document, font, Some(&without));
            if stated != ignored {
                changed.push(format!("{name} object {number}: {ignored:?} → {stated:?}"));
            }
        }
    }

    println!(
        "{with_a_style} fonts reach a descriptor with a /Style, {readable} a readable /Panose"
    );
    println!("  not twelve bytes: {unreadable:?}");
    println!("  {embedded} of them also embed the font program, which never asks");
    println!("  {} requests change because of it:", changed.len());
    for entry in &changed {
        println!("    {entry}");
    }

    assert_eq!(with_a_style, 46, "fonts whose descriptor carries a /Style");
    assert_eq!(
        readable, 44,
        "twelve bytes, as §9.8.3.2 requires; the other two are a producer's own length"
    );

    // The ratchet, and the finding. Both documents embed nothing and name **MS-Gothic** — a
    // Japanese sans-serif face — while their `/Flags` claim serif *and* fixed pitch *and* both
    // Symbolic and Nonsymbolic at once. §9.8.3.2's classification says Latin Text, Normal Sans,
    // Medium, Monospaced, and the font's own name agrees with it. So this is the case the hint
    // exists for: a descriptor whose flags are careless and whose classification is not.
    let names: Vec<&str> = changed
        .iter()
        .filter_map(|entry| entry.split(':').next())
        .collect();
    assert_eq!(
        names,
        [
            "noembed-eucjp.pdf object 6",
            "noembed-eucjp.pdf object 7",
            "noembed-sjis.pdf object 6",
            "noembed-sjis.pdf object 7"
        ],
        "requests the classification changes: {changed:?}"
    );
}

/// §9.8.3.3's `/FD`, and the sentence in it that no file can satisfy.
///
/// One corpus document states an `/FD`, and it names the class the clause itself recommends:
/// "[t]he FD dictionary should contain at least the metrics for the proportional Latin glyphs".
/// Its descriptor then holds ten entries, **every one of them Table 120's** — which the same
/// subclause forbids in the same breath as requiring metrics. The assertion below is that
/// contradiction written down against the only file that exercises it.
#[test]
fn the_corpuss_one_glyph_class_dictionary_states_what_the_clause_forbids() {
    let Some(files) = corpus() else {
        println!("skipped: the doc/pdf.js submodule is not checked out");
        return;
    };

    let mut found = Vec::new();
    for path in &files {
        let Ok(bytes) = std::fs::read(path) else {
            continue;
        };
        let Ok(document) = Document::open(bytes) else {
            continue;
        };
        let name = path.file_name().unwrap_or_default().to_string_lossy();
        for number in document.xref().object_numbers() {
            let object = document.get(ObjectId {
                number,
                generation: 0,
            });
            let Some(descriptor) = object.as_dict() else {
                continue;
            };
            let classes = pdf_font::substitute::glyph_classes(&document, descriptor);
            for (class, overrides) in classes {
                let mut keys: Vec<String> = overrides
                    .iter()
                    .map(|(key, _)| String::from_utf8_lossy(key.as_bytes()).into_owned())
                    .collect();
                keys.sort();
                found.push(format!("{name}: /{class} overriding {keys:?}"));
            }
        }
    }

    println!("glyph class dictionaries across the corpus:");
    for entry in &found {
        println!("  {entry}");
    }

    assert_eq!(found.len(), 1, "descriptors stating an /FD");
    let only = &found[0];
    assert!(only.starts_with("issue13147.pdf: /Proportional"), "{only}");
    for table_120 in [
        "Ascent",
        "CapHeight",
        "Descent",
        "Flags",
        "FontBBox",
        "FontName",
        "ItalicAngle",
        "StemH",
        "StemV",
        "XHeight",
    ] {
        assert!(
            only.contains(table_120),
            "/{table_120} is Table 120's, which §9.8.3.3 forbids here and which this file writes"
        );
    }
}
