//! Composite fonts, ISO 32000-2 §9.7, against the documents that exercise each rule.
//!
//! # Why this file exists
//!
//! Until the twentieth session a Type 0 font worked only under `Identity-H` with an identity
//! `/CIDToGIDMap`: an embedded `CMap` stream was refused (14 corpus fonts) and so was any
//! other `/CIDToGIDMap` (41 more). §9.7 is two independent mappings — codes to CIDs
//! (§9.7.6.2) and CIDs to glyph indices (§9.7.4.2) — and the Identity case is the one where
//! both are the identity and neither has to be read.
//!
//! # Why these assertions and not pixels
//!
//! Trap 1 in `doc/HANDOVER.md`: a font that loads and draws the wrong glyphs reports nothing
//! and looks like text. The oracle answers that with pixels somebody else produced, and it is
//! what found the two defects this file's fixtures are named for. What a unit test can add is
//! the *independent* check — the document states a width per CID in `/W` and the font program
//! states an advance per glyph in `hmtx`, through completely separate structures, so they
//! agree only if the CID reached the glyph the producer meant. That is
//! [`the_cid_widths_agree_with_the_font_programs_own_advances`], and it verifies the mapping
//! without consulting the mapping.
//!
//! # One rule is pinned only synthetically, and that is trap 8 rather than an omission
//!
//! §9.7.6.2 matches a code against a codespace range *byte by byte* — `<C280> <DFBF>` admits
//! `C2 80` and not `C2 C0` — which is not the same as comparing the whole code against
//! `0xC280..=0xDFBF`. Replacing the per-byte test with the numeric one and running the whole
//! corpus changes **no** page: every mapping any corpus `CMap` states falls inside its own
//! ranges byte-wise, so no code exists in these files that the two readings disagree about.
//! `cmap.rs`'s `a_codespace_range_is_matched_byte_by_byte` is a synthetic `CMap` and is the only
//! thing in the tree that holds the clause to its words. A corpus finds what documents contain,
//! not what the specification says.

#![expect(
    clippy::panic,
    clippy::indexing_slicing,
    clippy::arithmetic_side_effects,
    reason = "test code: a fixture that cannot exercise the rule must fail loudly rather than \
              pass by doing nothing, and every index here is taken from a length checked on \
              the line above"
)]

use std::path::{Path, PathBuf};

use pdf_syntax::{Dictionary, Document, Object};

/// A corpus document, or `None` when the submodule is not checked out.
///
/// `None` means that and nothing else: a document that is present but does not carry what a
/// test is about is a panic, because a fixture that cannot exercise the rule is a test that
/// passes by doing nothing — which the twelfth session shipped twice.
fn corpus_document(name: &str) -> Option<Document> {
    let path: PathBuf = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../doc/pdf.js/test/pdfs")
        .join(name);
    let bytes = std::fs::read(path).ok()?;
    Some(Document::open(bytes).unwrap_or_else(|e| panic!("{name} does not open: {e}")))
}

/// Page one's `/Font` resources, by resource name.
fn page_one_fonts(document: &Document, name: &str) -> Vec<(String, Dictionary)> {
    let page = pdf_model::Pages::new(document)
        .get(0)
        .unwrap_or_else(|| panic!("{name} has no page one"));
    let fonts = document.get_key(&page.resources, "Font");
    let Some(fonts) = fonts.as_dict() else {
        panic!("{name} page one names no /Font resources");
    };
    fonts
        .iter()
        .filter_map(|(key, value)| {
            let resolved = document.resolve(value);
            let dict = resolved.as_dict()?.clone();
            Some((String::from_utf8_lossy(key.as_bytes()).into_owned(), dict))
        })
        .collect()
}

/// Loads one named font from a document's first page, failing loudly rather than skipping.
fn load(document: &Document, file: &str, resource: &str) -> pdf_font::LoadedFont {
    let fonts = page_one_fonts(document, file);
    let (_, dict) = fonts
        .iter()
        .find(|(key, _)| key == resource)
        .unwrap_or_else(|| panic!("{file} page one has no font /{resource}"));
    pdf_font::LoadedFont::load(document, dict, resource)
        .unwrap_or_else(|e| panic!("{file} /{resource} did not load: {e}"))
}

/// Every code a string decodes to, with its length and whether it produced an outline.
fn decoded(font: &pdf_font::LoadedFont, bytes: &[u8]) -> Vec<(u32, u8, bool)> {
    font.decode(bytes)
        .into_iter()
        .map(|code| (code.value(), code.length(), font.outline(code).is_some()))
        .collect()
}

/// An embedded `CMap` may declare one-byte codes, and then one byte is one code.
///
/// `issue2931.pdf` writes `1 begincodespacerange <20> <76> endcodespacerange` — the whole
/// codespace is a single one-byte range — with `begincidchar` and `begincidrange` sections
/// over it. Under the Identity reading this tree had before, every pair of bytes was one code
/// and the page drew half as many glyphs, from the wrong CIDs.
#[test]
fn a_one_byte_codespace_range_makes_one_byte_one_code() {
    let Some(document) = corpus_document("issue2931.pdf") else {
        println!("skipped: the doc/pdf.js submodule is not checked out");
        return;
    };
    let font = load(&document, "issue2931.pdf", "F1");

    // "ACE" — <41> is the start of `<43> <44> 6`'s neighbour range and <41> is `begincidchar
    // <41> 5`, so the CIDs are 5, 6 and 8 respectively.
    let codes = decoded(&font, b"ACI");
    assert_eq!(
        codes
            .iter()
            .map(|&(_, length, _)| length)
            .collect::<Vec<_>>(),
        vec![1, 1, 1],
        "the codespace has one one-byte range, so every code is one byte"
    );
    assert_eq!(
        codes.iter().map(|&(value, _, _)| value).collect::<Vec<_>>(),
        vec![0x41, 0x43, 0x49]
    );
    assert!(
        codes.iter().all(|&(_, _, outline)| outline),
        "each of those codes has a cidchar or cidrange entry and a glyph: {codes:?}"
    );
}

/// A `CMap` may mix code lengths inside one string, and §9.7.6.2 decides where each ends.
///
/// `issue18117.pdf`'s `CMap` is UTF-8 shaped: `<00> <7F>`, `<C080> <DFBF>`, `<E08080>
/// <EFBFBF>`, `<F0808080> <F7BFBFBF>`. A one-byte code and a three-byte code appear in the
/// same string, and the boundaries come from the ranges rather than from a fixed width.
#[test]
fn a_utf8_shaped_codespace_yields_codes_of_several_lengths() {
    let Some(document) = corpus_document("issue18117.pdf") else {
        println!("skipped: the doc/pdf.js submodule is not checked out");
        return;
    };
    let font = load(&document, "issue18117.pdf", "FT1");

    let codes = decoded(&font, b"a\xe4\xb8\x80b");
    assert_eq!(
        codes
            .iter()
            .map(|&(_, length, _)| length)
            .collect::<Vec<_>>(),
        vec![1, 3, 1],
        "one byte, then three, then one: {codes:?}"
    );
    assert_eq!(codes[1].0, 0xe4_b8_80, "the three bytes are one code");
}

/// §9.7.5.4 c) forbids `bfchar` in an `Encoding` `CMap`; §9.7.6.2 names it among the mappings.
///
/// `bug920426.pdf` writes an `Encoding` `CMap` whose *only* character mappings are `bfchar`
/// lines with two-byte hex destinations. Ignoring them leaves every code undefined and the
/// page blank; reading the destination as the character selector §9.7.5.1 says a `CMap` yields
/// draws "Checkliste Service", which is what three reference renderers draw.
#[test]
fn a_bfchar_in_an_encoding_cmap_selects_a_cid() {
    let Some(document) = corpus_document("bug920426.pdf") else {
        println!("skipped: the doc/pdf.js submodule is not checked out");
        return;
    };
    let font = load(&document, "bug920426.pdf", "F1");

    // `<0043> <0026>` is one of the file's own `bfchar` lines.
    let codes = decoded(&font, b"\x00\x43");
    assert_eq!(codes.len(), 1);
    assert!(
        codes[0].2,
        "the bfchar destination must reach a glyph, not `.notdef`: {codes:?}"
    );
}

/// A `/CIDToGIDMap` stream remaps CIDs, and Table 115 fixes its layout.
///
/// This is the strong check and the one worth the most in this file, because it does not
/// consult the mapping to verify the mapping. The document states a width per CID in `/W`
/// (§9.7.4.3) and the embedded `TrueType` program states an advance per glyph index in `hmtx`;
/// the two are written by the same producer from the same font and travel through completely
/// separate structures, so they agree only if the whole chain — §9.7.6.2's `CMap` from code to
/// CID, then §9.7.4.2's `/CIDToGIDMap` from CID to glyph — landed on the glyph the producer
/// meant. A stream read at the wrong offset, or ignored in favour of the identity, disagrees on
/// nearly every code.
///
/// The `hmtx` table is read here by hand rather than through `skrifa`, which is what the
/// renderer uses: reading it with the same library would remove half of the independence the
/// check rests on.
#[test]
fn the_cid_widths_agree_with_the_font_programs_own_advances() {
    /// Corpus documents whose page-one Type 0 fonts have a `/CIDToGIDMap` stream over an
    /// embedded `TrueType` program, so both halves of §9.7.4.2 are exercised.
    const FILES: &[&str] = &["basicapi.pdf", "issue2074.pdf", "complex_ttf_font.pdf"];

    let mut checked = 0usize;
    let mut disagreed = Vec::new();
    let mut fonts_examined = 0usize;

    for file in FILES {
        let Some(document) = corpus_document(file) else {
            println!("skipped: the doc/pdf.js submodule is not checked out");
            return;
        };
        for (resource, dict) in page_one_fonts(&document, file) {
            let Some(descendant) = descendant_of(&document, &dict) else {
                continue;
            };
            if !matches!(
                document.get_key(&descendant, "CIDToGIDMap"),
                Object::Stream(_)
            ) {
                continue;
            }
            let program = embedded_truetype(&document, &descendant)
                .unwrap_or_else(|| panic!("{file} /{resource} has no embedded TrueType program"));
            let advances = truetype_advances(&program)
                .unwrap_or_else(|| panic!("{file} /{resource}: no hmtx advances"));
            let upem = units_per_em(&document, &descendant, &program);
            // Table 115's default, which the clause gives as 1000.
            let default_width = document
                .get_key(&descendant, "DW")
                .as_number()
                .unwrap_or(1000.0);
            let font = pdf_font::LoadedFont::load(&document, &dict, &resource)
                .unwrap_or_else(|e| panic!("{file} /{resource} did not load: {e}"));
            fonts_examined += 1;

            // Every two-byte code, which is what these `CMap`s define. A code with no `/W`
            // entry falls back to `/DW`, and a default carries no statement about a particular
            // CID to cross-check — so those are skipped, exactly as the bare-CFF version of
            // this check skips a subset font's padding zeros.
            for value in 0..=u32::from(u16::MAX) {
                let bytes = [
                    u8::try_from(value >> 8).unwrap_or(0),
                    u8::try_from(value & 0xFF).unwrap_or(0),
                ];
                let codes = font.decode(&bytes);
                if codes.len() != 1 {
                    continue;
                }
                let code = codes[0];
                let declared = f64::from(font.advance(code)) * 1000.0;
                if (declared - default_width).abs() < 0.5 {
                    continue;
                }
                let Some(glyph) = font.glyph_index(code) else {
                    continue;
                };
                if glyph == 0 {
                    continue;
                }
                let Some(&advance) = advances.get(usize::from(glyph)) else {
                    continue;
                };
                let from_program = f64::from(advance) / f64::from(upem) * 1000.0;
                checked += 1;
                if (from_program - declared).abs() > 1.5 {
                    disagreed.push(format!(
                        "{file} /{resource}: code {value:#06x} is {declared:.0} wide per /W but \
                         glyph {glyph} advances {from_program:.1}"
                    ));
                }
            }
        }
    }

    assert!(
        fonts_examined >= FILES.len(),
        "only {fonts_examined} fonts had a /CIDToGIDMap stream, so this proves little"
    );
    assert!(
        checked > 40,
        "only {checked} widths were comparable, so this proves little"
    );
    // A wrong mapping does not produce a few stragglers, it produces mostly-wrong. A producer
    // may legitimately override one glyph's advance in `/W`, which is why this is a share
    // rather than an exact match.
    assert!(
        disagreed.len() * 20 < checked,
        "{} of {checked} widths disagree with the font program:\n{}",
        disagreed.len(),
        disagreed
            .iter()
            .take(12)
            .cloned()
            .collect::<Vec<_>>()
            .join("\n")
    );
}

/// A predefined `CMap` is resolved from the data this binary carries, on a real document.
///
/// **This test used to assert the opposite.** §9.7.5.2 makes these names *data* rather than an
/// algorithm, the data was not in the tree, and the refusal was what there was to pin. It is in
/// the tree since the hundred-and-fifty-sixth session, so the same fixture now pins the
/// mapping: `90ms_rksj_h_sample.pdf` shows Shift-JIS text through Microsoft Code Page 932, and
/// what says the `CMap` was really consulted is that a *two-byte* code comes back — an
/// unconsulted `CMap` would take the same bytes one at a time.
#[test]
fn a_predefined_cmap_is_resolved_by_name() {
    let Some(document) = corpus_document("90ms_rksj_h_sample.pdf") else {
        println!("skipped: the doc/pdf.js submodule is not checked out");
        return;
    };
    let fonts = page_one_fonts(&document, "90ms_rksj_h_sample.pdf");
    let (_, dict) = fonts
        .iter()
        .find(|(key, _)| key == "F1")
        .expect("page one has /F1");
    let font = pdf_font::LoadedFont::load(&document, dict, "F1")
        .expect("90ms-RKSJ-H is one of the predefined CMaps this binary carries");
    let codes = font.decode(b"\x82\xa0\x41");
    assert_eq!(
        codes.iter().map(|code| code.length()).collect::<Vec<_>>(),
        vec![2, 1],
        "Code Page 932 is a mixed-width encoding and the CMap's codespace is what says so"
    );
}

/// A `CMap` in vertical writing mode selects §9.7.4.3's second set of metrics.
///
/// §9.7.5.1 makes the writing mode a property of the `CMap` and says it "determines which
/// metrics shall be used"; §9.7.4.3 puts those metrics in `/W2` and `/DW2`, and §9.2.4 makes
/// them a displacement `w1` whose horizontal component is 0 and a position vector `v` from
/// the horizontal origin to the vertical one.
///
/// `vertical.pdf` sets two columns down the right edge of a page, and it states no `/W2` at
/// all — so what places its glyphs is Table 115's default `/DW2` of `[880 -1000]`, one em
/// down per glyph and a position vector of half the glyph's width across.
#[test]
#[expect(
    clippy::float_cmp,
    reason = "test code: every value compared is a constant of the clause's or a quotient of \
              two, so the comparison is exact by construction"
)]
fn a_vertical_cmap_takes_the_second_set_of_metrics() {
    let Some(document) = corpus_document("vertical.pdf") else {
        println!("skipped: the doc/pdf.js submodule is not checked out");
        return;
    };
    let fonts = page_one_fonts(&document, "vertical.pdf");
    let (_, dict) = fonts
        .iter()
        .find(|(key, _)| key == "F1")
        .expect("page one has /F1");
    let font = pdf_font::LoadedFont::load(&document, dict, "F1").expect("Identity-V loads");

    assert!(font.is_vertical(), "Identity-V is writing mode 1");
    // The first code of the page's own string, decoded the way the interpreter decodes it.
    let code = *font
        .decode(&[0x00, 0x01])
        .first()
        .expect("a two-byte code decodes");
    let (displacement, position) = font.vertical_metrics(code);
    assert_eq!(
        displacement[0], 0.0,
        "§9.2.4: w1's horizontal component shall be 0"
    );
    assert_eq!(
        displacement[1], -1.0,
        "and /DW2's default puts the next glyph one em below"
    );
    assert_eq!(
        position[1], 0.88,
        "and the position vector's vertical component is /DW2's first number"
    );
    assert_eq!(
        position[0],
        font.advance(code) / 2.0,
        "§9.7.4.3: \"the horizontal component of the position vector shall be half the glyph \
         width\""
    );
}

/// §9.4.4's combined displacement moves the text matrix down the page, and not across it.
///
/// The metrics test above proves the font *states* §9.7.4.3's second set; this proves the
/// interpreter *applies* them, which is a different clause. ISO 32000-2 §9.4.4:
///
/// > After the glyph is painted, the text matrix shall be updated according to the glyph
/// > displacement and any spacing parameters that apply. First, a combined displacement shall be
/// > computed, denoted by t x in horizontal writing mode or t y in vertical writing mode (the
/// > variable corresponding to the other writing mode shall be set to 0)
///
/// The observable is the text layer — one entry per character code, each with the quadrilateral
/// its glyph occupies — so "set to 0" is readable as a fact about where consecutive glyphs of one
/// string landed: the same column, descending. `vertical.pdf` is the corpus's `Identity-V`
/// document and the only one that exercises the branch.
#[test]
fn a_vertical_font_advances_down_the_page_rather_than_across_it() {
    let Some(document) = corpus_document("vertical.pdf") else {
        println!("skipped: the doc/pdf.js submodule is not checked out");
        return;
    };
    let pages = pdf_model::Pages::new(&document);
    let page = pages.get(0).expect("vertical.pdf has a first page");
    let interpretation = pdf_model::content::interpret(&document, &page);
    let placed = &interpretation.text_layer;
    assert!(
        placed.len() >= 2,
        "the page shows a string: {}",
        placed.len()
    );

    // The centre of each glyph's quadrilateral, which is what "where it landed" means without
    // depending on which corner the box starts at.
    let centre = |quad: &[f32; 8]| {
        (
            (quad[0] + quad[2] + quad[4] + quad[6]) / 4.0,
            (quad[1] + quad[3] + quad[5] + quad[7]) / 4.0,
        )
    };
    let first = centre(&placed[0].quad);
    let second = centre(&placed[1].quad);
    assert!(
        (second.0 - first.0).abs() < 1.0,
        "t x is 0 in vertical writing mode: {first:?} then {second:?}"
    );
    assert!(
        second.1 < first.1 - 1.0,
        "and t y carries the next glyph below it in user space: {first:?} then {second:?}"
    );
}

/// The descendant `CIDFont` of a Type 0 font, per §9.7.6.1's one-element `/DescendantFonts`.
fn descendant_of(document: &Document, dict: &Dictionary) -> Option<Dictionary> {
    if document.get_key(dict, "Subtype").as_name()?.as_bytes() != b"Type0" {
        return None;
    }
    document
        .get_key(dict, "DescendantFonts")
        .as_array()
        .and_then(<[Object]>::first)
        .map(|item| document.resolve(item))
        .and_then(|item| item.as_dict().cloned())
}

/// The embedded `TrueType` program from `/FontFile2`.
fn embedded_truetype(document: &Document, descendant: &Dictionary) -> Option<Vec<u8>> {
    let descriptor = document.get_key(descendant, "FontDescriptor");
    let descriptor = descriptor.as_dict()?;
    let object = document.get_key(descriptor, "FontFile2");
    let stream = object.as_stream()?;
    document
        .decoded_stream_data(stream)
        .map(|bytes| bytes.to_vec())
}

/// Advances from a `TrueType` program's `hmtx` table, in font units, by glyph index.
///
/// Read here rather than through `skrifa` on purpose: this test's whole value is that the two
/// statements of a glyph's advance travel through separate code, and reading the font with the
/// same library the renderer uses would remove half of that.
fn truetype_advances(program: &[u8]) -> Option<Vec<u16>> {
    let (hhea, _) = sfnt_table(program, *b"hhea")?;
    let (hmtx, _) = sfnt_table(program, *b"hmtx")?;
    let (maxp, _) = sfnt_table(program, *b"maxp")?;
    let count = u16::from_be_bytes([*program.get(hhea + 34)?, *program.get(hhea + 35)?]);
    let glyphs = u16::from_be_bytes([*program.get(maxp + 4)?, *program.get(maxp + 5)?]);

    let mut advances = Vec::with_capacity(usize::from(glyphs));
    let mut last = 0u16;
    for glyph in 0..usize::from(glyphs) {
        if glyph < usize::from(count) {
            let at = hmtx + glyph * 4;
            last = u16::from_be_bytes([*program.get(at)?, *program.get(at + 1)?]);
        }
        // Beyond `numberOfHMetrics` every glyph repeats the last advance, which is how a
        // monospaced tail is stored.
        advances.push(last);
    }
    Some(advances)
}

/// A `TrueType` program's units per em, from `head`.
///
/// The descendant is passed only so a failure can name the font; a `TrueType` program without a
/// `head` table is malformed and the fixture is not, so this panics rather than guessing 1000.
fn units_per_em(document: &Document, descendant: &Dictionary, program: &[u8]) -> u16 {
    let _ = (document, descendant);
    let Some((head, _)) = sfnt_table(program, *b"head") else {
        panic!("the embedded TrueType program has no head table");
    };
    let (Some(&high), Some(&low)) = (program.get(head + 18), program.get(head + 19)) else {
        panic!("the head table is truncated");
    };
    u16::from_be_bytes([high, low])
}

/// Finds one table in an sfnt's directory, returning its offset and length.
fn sfnt_table(program: &[u8], tag: [u8; 4]) -> Option<(usize, usize)> {
    let count = usize::from(u16::from_be_bytes([*program.get(4)?, *program.get(5)?]));
    for index in 0..count {
        let record = program.get(12 + index * 16..12 + index * 16 + 16)?;
        if record[..4] != tag {
            continue;
        }
        let offset = u32::from_be_bytes([record[8], record[9], record[10], record[11]]);
        let length = u32::from_be_bytes([record[12], record[13], record[14], record[15]]);
        return Some((usize::try_from(offset).ok()?, usize::try_from(length).ok()?));
    }
    None
}

/// A `doc/corpora/pdfbox` document, or `None` when that submodule is not checked out.
///
/// Same rule as [`corpus_document`]: absent is a skip, present-but-wrong is a panic.
fn pdfbox_document(name: &str) -> Option<Document> {
    let path: PathBuf = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../doc/corpora/pdfbox/pdfbox/src/test/resources/input")
        .join(name);
    let bytes = std::fs::read(path).ok()?;
    Some(Document::open(bytes).unwrap_or_else(|e| panic!("{name} does not open: {e}")))
}

/// Page one's readback.
fn page_one_text(document: &Document, name: &str) -> String {
    let page = pdf_model::Pages::new(document)
        .get(0)
        .unwrap_or_else(|| panic!("{name} has no page one"));
    pdf_model::interpret(document, &page).text
}

/// §9.10.2's third method does not apply to `Identity-H`, so its permission has to.
///
/// The clause names the exception itself — the third method is for a composite font using one
/// of Table 116's predefined `CMap`s "(except Identity -H and Identity -V )" or a descendant in
/// one of five Adobe collections — so an `Identity-H` font whose descendant is `Adobe-Identity`
/// has only the first method available. When its `/ToUnicode` answers for some of its codes and
/// not others, every method has failed for the rest and
/// `pdf_font::LoadedFont::text_from_program` is the choice the clause permits: the `CMap` gives
/// a CID, §9.7.4.2's `/CIDToGIDMap` gives the glyph, and the program names it.
///
/// `PDFBOX-5838-0024320-reduced.pdf` is the sharpest witness in the tree. Its `/ToUnicode` maps
/// 8 codes of the 15 the page shows, so the page read back `H Reeach Pec` — a string with no
/// missing-glyph report attached to it, because nothing was missing except the *names*.
#[test]
fn an_identity_h_fonts_partial_to_unicode_is_completed_by_the_program() {
    let Some(document) = pdfbox_document("PDFBOX-5838-0024320-reduced.pdf") else {
        println!("skipped: doc/corpora/pdfbox is not checked out");
        return;
    };
    let text = page_one_text(&document, "PDFBOX-5838-0024320-reduced.pdf");
    assert_eq!(text.trim(), "Honors Research Project");
}

/// The same clause on the pdf.js corpus, where it had been costing two documents in silence.
///
/// `issue16553.pdf` is an Okular signature appearance in an `Identity-H` Noto subset; it sat on
/// `text_extraction.rs`'s named list for 357 sessions as "partial for reasons nobody has
/// diagnosed further".
#[test]
fn an_identity_h_signature_appearance_reads_back_its_name() {
    let Some(document) = corpus_document("issue16553.pdf") else {
        return;
    };
    let text = page_one_text(&document, "issue16553.pdf");
    assert!(
        text.contains("ONDŘEJ MACHULDA"),
        "the signer's name should be read back, not just drawn: {text:?}"
    );
}

/// And the permission stops where the file stops saying things, which is a **choice**.
///
/// `PDFBOX-4322-Empty-ToUnicode-reduced.pdf` shows `<004a0075007300740069006e>` in an
/// `Identity-H` Calibri subset whose `/ToUnicode` is a copy of the `Identity-H` CID `CMap`:
/// `/CMapType 1`, one `begincidrange`, and not one of the `bfchar` or `bfrange` operators
/// §9.10.3 requires. The embedded program carries neither a `cmap` table nor a `post` one, so
/// it names nothing either, and every route the standard describes has failed.
///
/// `PDFBox` reads each two-byte code as a Unicode value and returns `Justin`, which is right here
/// only because this producer numbered its CIDs by code point. This tree declines: §9.6.5's
/// encodings are one byte per code, so `text_from_the_code`'s argument covers a byte and covers
/// nothing wider. Held by a test because a silence nobody pinned is a silence somebody fills.
#[test]
fn a_two_byte_code_is_not_read_as_a_character_when_nothing_names_it() {
    let Some(document) = pdfbox_document("PDFBOX-4322-Empty-ToUnicode-reduced.pdf") else {
        println!("skipped: doc/corpora/pdfbox is not checked out");
        return;
    };
    let text = page_one_text(&document, "PDFBOX-4322-Empty-ToUnicode-reduced.pdf");
    assert_eq!(text.trim(), "");
}

/// A `/ToUnicode` `CMap` that states only its differences from another one (§9.10.3).
///
/// The entry is `/UseCMap`, which "may be used if the `CMap` is based on another `ToUnicode`
/// `CMap`."
/// The sentence naming it is rewritten by Errata Collection 3's Issue #462 (`/State` `Review`
/// `Completed`), which strikes the clause's pointer at Table 118 and inserts a table of the
/// `/ToUnicode` stream's own entries; this comment quoted the struck half as a blockquote until
/// the five-hundred-and-ninety-first session. The entry is the same under both readings, which is
/// what this test holds.
///
/// `issue5010.pdf` is that shape and states the relationship the *other* way §9.7.5.4 a) allows
/// it to be written — `/Adobe-Korea1-UCS2 usecmap` inside the file, with no `/UseCMap` in the
/// stream dictionary, which that clause requires to be there as well. Its `/ToUnicode` states
/// five mappings of its own, all for codes `<46FB>`–`<4704>`, and the page shows none of them:
/// every one of its twelve codes is answered by `Adobe-Korea1-UCS2` alone.
///
/// **Discriminating**: §9.10.2's other routes cannot reach these characters. The descendant's
/// `/CIDSystemInfo` names the registry `Unidocs`, so the third method's `registry-ordering-UCS2`
/// name is `Unidocs-Korea1-UCS2` and nobody publishes one; the font is composite, so the second
/// method does not apply; and the program is a CID-keyed CFF with no glyph names to fall back
/// on. Before the base was read this page read back the empty string.
///
/// The expected value is derived rather than copied: the codes are CIDs (`Identity-H`), and
/// `data/cmaps/Adobe-Korea1-UCS2` — Adobe's own published file, which §9.10.2 step d) names as
/// the source — maps CIDs 2635, 3104, 2093, 160, 3352, 2635, 3340, 2976, 3286, 1732, 2633 and
/// 3104 to exactly these characters.
#[test]
fn a_to_unicode_cmap_built_on_a_published_one_reads_back_through_it() {
    let Some(document) = corpus_document("issue5010.pdf") else {
        return;
    };
    let text = page_one_text(&document, "issue5010.pdf");
    assert_eq!(text.trim(), "인터뷰●홍인혜카피라이터");
}
