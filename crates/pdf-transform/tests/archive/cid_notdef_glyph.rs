//! A CID a `CIDFontType2` font's `/CIDToGIDMap` sends to its program's `.notdef` glyph is given a
//! glyph of its own whose outline and advance are `.notdef`'s, one for each width, and the map is
//! restated as a stream while the content stream stays the producer's bytes (`doc/adr/2556`,
//! `doc/questions/A694` item 3).
//!
//! ISO 19005-2 section 6.2.11.8 and ISO 19005-4 section 6.2.10.9 forbid a text-showing operator
//! to reference `.notdef`; ISO 32000-2 Table 115 states the map stream's layout, and §9.7.4.3 keys
//! a composite font's widths by CID, which ISO 19005-2 section 6.2.11.5 holds the program's
//! advance to.

use pdf_font::LoadedFont;
use pdf_syntax::object::ObjectId;
use pdf_syntax::{Document, Limits};

use super::{
    Authorisations, Because, Conforming, Decision, LIBERATION_SANS, Level, Rewrite, Target,
    Verdict, conversion, convert, decision, holds, stream,
};

const NOTDEF_SHOWN: &str = "fonts/no-notdef-glyph-shown";

/// What the descendant's `/CIDToGIDMap` states in a fixture.
#[derive(Clone, Copy)]
enum Map {
    /// A stream of 37 entries sending CID 36 to glyph 36 (`A` in the program) and every other
    /// CID to glyph 0; the page shows CIDs 1, 2 and 36.
    Stream,
    /// `/Identity`; the page shows CIDs 0 and 36.
    Identity,
}

/// The CIDs on glyph 0 a fixture's page shows, then the one that reaches `A`.
fn shown(map: Map) -> [u16; 3] {
    match map {
        Map::Stream => [1, 2, 36],
        Map::Identity => [0, 0, 36],
    }
}

/// A page showing, in one `Identity-H` font over a `CIDFontType2` whose program is Liberation
/// Sans, the CIDs [`shown`] names, the ones on glyph 0 at `widths` by `/W`.
fn a_page_showing_notdef(map: Map, widths: (u16, u16)) -> Vec<u8> {
    a_page_showing_notdef_under(map, widths, false)
}

/// [`a_page_showing_notdef`]'s page, its descendant written inside `/DescendantFonts` where
/// `direct`, which leaves the copy no object to write its new entries into.
fn a_page_showing_notdef_under(map: Map, widths: (u16, u16), direct: bool) -> Vec<u8> {
    let [first, second, letter] = shown(map);
    let (codes, w, map_entry) = match map {
        Map::Stream => (
            format!("<{first:04X}{second:04X}{letter:04X}>"),
            format!("{first} [{} {}] 36 [667]", widths.0, widths.1),
            "/CIDToGIDMap 10 0 R",
        ),
        Map::Identity => (
            format!("<{first:04X}{letter:04X}>"),
            format!("{first} [{}] 36 [667]", widths.0),
            "/CIDToGIDMap /Identity",
        ),
    };
    let entries: Vec<u8> = (0..=36_u16)
        .flat_map(|cid| if cid == 36 { 36_u16 } else { 0 }.to_be_bytes())
        .collect();
    let mut binary_objects = vec![stream(
        &format!("/Length {}", LIBERATION_SANS.len()),
        LIBERATION_SANS,
    )];
    if matches!(map, Map::Stream) {
        binary_objects.push(stream(&format!("/Length {}", entries.len()), &entries));
    }
    let descendant = format!(
        "<< /Type /Font /Subtype /CIDFontType2 /BaseFont /LiberationSans /CIDSystemInfo << \
         /Registry (Adobe) /Ordering (Identity) /Supplement 0 >> /FontDescriptor 8 0 R /DW 1000 \
         /W [{w}] {map_entry} >>"
    );
    Conforming {
        resources: "/Font << /F1 6 0 R >> /ColorSpace << /CS0 [/CalGray << /WhitePoint \
                    [0.9505 1.0 1.089] >>] >>"
            .to_owned(),
        contents: Some((
            String::new(),
            format!("/CS0 cs 0 sc BT /F1 12 Tf 10 100 Td {codes} Tj ET").into_bytes(),
        )),
        objects: vec![
            format!(
                "<< /Type /Font /Subtype /Type0 /BaseFont /LiberationSans /Encoding /Identity-H \
                 /DescendantFonts [{}] >>",
                if direct { &descendant } else { "7 0 R" }
            ),
            if direct {
                "null".to_owned()
            } else {
                descendant
            },
            "<< /Type /FontDescriptor /FontName /LiberationSans /Flags 4 /FontBBox [-543 -303 \
             1300 980] /ItalicAngle 0 /Ascent 905 /Descent -212 /CapHeight 716 /StemV 80 \
             /FontFile2 9 0 R >>"
                .to_owned(),
        ],
        binary_objects,
        ..Conforming::part_two()
    }
    .build()
}

/// The glyph each CID [`shown`] names reaches in the font object 6 of `bytes`.
fn glyphs_of(bytes: &[u8], map: Map) -> [Option<u16>; 3] {
    let held = Document::open_with_limits(bytes.to_vec(), Limits::DEFAULT).expect("it opens");
    let dict = held
        .get(ObjectId::new(6, 0))
        .as_dict()
        .cloned()
        .expect("the font object");
    let font = LoadedFont::load(&held, &dict, "").expect("the font loads");
    shown(map).map(|cid| {
        let code = *font.decode(&cid.to_be_bytes()).first().expect("one code");
        font.glyph_index(code)
    })
}

#[test]
fn two_cids_on_notdef_at_two_widths_reach_two_glyphs_of_their_own() {
    // 500 and 502 thousandths of an em are 1024 and 1028 of the program's 2048 units: one copy
    // would carry one advance, and the other CID's width would differ from it by more than ISO
    // 19005-2 section 6.2.11.5's thousandth.
    given_glyphs(
        &a_page_showing_notdef(Map::Stream, (500, 502)),
        Map::Stream,
        false,
    );
}

#[test]
fn a_width_no_advance_can_hold_is_refused_and_no_file_is_written() {
    // 40 em is 81 920 of the program's units, past the 16 bits an advance is stored in.
    let source = a_page_showing_notdef(Map::Stream, (500, 40_000));
    let (report, output) = convert(&source, Target::Two(Level::B), Authorisations::default());
    assert!(
        matches!(
            decision(&report, NOTDEF_SHOWN),
            Decision::Refused(Because::TheFence(why)) if why.contains("CIDToGIDMap")
        ),
        "{:?}",
        decision(&report, NOTDEF_SHOWN)
    );
    assert!(output.is_none(), "and no file is written");
}

#[test]
fn two_cids_on_notdef_at_one_width_share_one_glyph_of_their_own() {
    given_glyphs(
        &a_page_showing_notdef(Map::Stream, (500, 500)),
        Map::Stream,
        true,
    );
}

#[test]
fn cid_zero_under_the_identity_is_refused_and_no_file_is_written() {
    // ISO 32000-2 §9.7.4.2: CID 0's glyph is the CIDFont's analogue of `.notdef`, so showing CID 0
    // references it whatever glyph a map stream would send it to.
    let source = a_page_showing_notdef(Map::Identity, (750, 750));
    let (report, output) = convert(&source, Target::Two(Level::B), Authorisations::default());
    let program_row = decision(&report, "fonts/embedded-programs-define-every-glyph-shown");
    assert!(
        matches!(
            program_row,
            Decision::Refused(Because::NotBuiltYet(why)) if why.contains("not CID 0")
        ),
        "{program_row:?}"
    );
    assert!(output.is_none(), "and no file is written");
}

/// Converts `source` at 2b and part 4 and holds each output to the restatement's properties, the
/// two CIDs on one glyph where `shared` and on two otherwise.
fn given_glyphs(source: &[u8], map: Map, shared: bool) {
    let before = glyphs_of(source, map);
    assert_eq!(
        before,
        [Some(0), Some(0), Some(36)],
        "the CIDs reach .notdef and A"
    );
    let failed: Vec<&str> = holds(source, Target::Two(Level::B))
        .failures()
        .map(|failed| failed.id)
        .collect();
    assert!(failed.contains(&NOTDEF_SHOWN), "{failed:?}");
    for target in [
        Target::Two(Level::B),
        Target::Four(pdf_archive::Flavour::Plain),
    ] {
        let (report, output) = convert(source, target, Authorisations::default());
        assert!(
            matches!(
                decision(&report, NOTDEF_SHOWN),
                Decision::Stated {
                    rewrite: Rewrite::NotdefGlyphGiven,
                    ..
                }
            ),
            "{:?}",
            decision(&report, NOTDEF_SHOWN)
        );
        let output = output.unwrap_or_else(|| {
            panic!(
                "{:?}\n{:?}\n{:?}",
                conversion(&report).achieved,
                report.refused,
                report.warnings
            )
        });
        let held = holds(&output, target);
        assert_eq!(held.verdict(), Verdict::Conforms, "{}", held.render());
        let after = glyphs_of(&output, map);
        let given = after[0].expect("the first CID reaches a glyph");
        assert_ne!(given, 0, "no CID reaches .notdef");
        if shared {
            assert_eq!(after[1], Some(given), "the two CIDs share the one copy");
        } else {
            assert!(
                after[1].is_some_and(|other| other != given && other != 0),
                "two widths, two glyphs: {after:?}"
            );
        }
        assert_eq!(after[2], Some(36), "A reaches the glyph it reached");
        let text = String::from_utf8_lossy(&output).into_owned();
        assert!(
            text.contains(match map {
                Map::Stream => "BT /F1 12 Tf 10 100 Td <000100020024> Tj ET",
                Map::Identity => "BT /F1 12 Tf 10 100 Td <00000024> Tj ET",
            }),
            "the content stream is the producer's bytes"
        );
        let packet = String::from_utf8(super::document_packet(&output)).expect("UTF-8");
        assert!(packet.contains("CIDToGIDMap is restated"), "{packet}");
    }
}

#[test]
fn a_descendant_the_copy_does_not_reach_states_the_copy_s_own_reason() {
    // The same page with its descendant written in place, which the copy does not reach: the
    // program row names the remedy's condition, never the placeholder for a construction nobody
    // asked for.
    let source = a_page_showing_notdef_under(Map::Stream, (500, 500), true);
    let (report, output) = convert(&source, Target::Two(Level::B), Authorisations::default());
    let program_row = decision(&report, "fonts/embedded-programs-define-every-glyph-shown");
    assert!(
        matches!(
            program_row,
            Decision::Refused(Because::NotBuiltYet(why)) if why.contains("a font here is another kind")
        ),
        "{program_row:?}"
    );
    assert!(output.is_none(), "and no file is written");
}
