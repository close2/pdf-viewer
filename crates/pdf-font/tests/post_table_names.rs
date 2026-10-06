//! A `TrueType` program's `post` names are read once per face, however many it holds.
//!
//! ISO 32000-2 §9.6.5.4 sends a simple font's glyph name that no list maps to the font program's
//! `post` table. The OpenType specification's `post` table, in its version 2.0 section, stores
//! those names as a run of Pascal strings with no offsets, so a reader that asks for one glyph's
//! name steps over every name before it. A search of every glyph for one name, made once per
//! code, then costs the square of the glyph count per code. The
//! `page` fuzz target found it: `slow-unit-0c5ebcab3dec74b1571560be89ddeb8ab3b64b92`,
//! `slow-unit-2b9f6ddf11729a032f3e5d6f2f9fc7a2c9a50524` and
//! `slow-unit-e788c7ce4632e38b0c3999bccaf37829c596a759` share one 5125-glyph table and spent
//! 14.3 G instructions each in it (ADR 1584).
//!
//! The fixtures are generated rather than copied from those units, at the unit's size and at the
//! format's largest, 65 535 glyphs: 256 codes whose names are not in the Adobe Glyph List, half
//! named by the table and half named by nothing, so that half the lookups search the whole table
//! and find nothing.
//!
//! **The bound is a clock, and it is not a bound on the machine's load.** The answers are checked
//! name by name, which holds the reading; the clock holds the shape. At 65 535 glyphs the search
//! the clock guards against steps over about 5 × 10¹¹ strings for this fixture, which is hours,
//! and the table built once steps over 65 535; the ceiling sits two orders of magnitude above the
//! second and several below the first.

#![expect(
    clippy::expect_used,
    clippy::panic,
    clippy::arithmetic_side_effects,
    reason = "test code: a malformed fixture should fail loudly, and the sums are offsets into a \
              fixture of three tables and glyph numbers below the count they are built from"
)]

use std::fmt::Write as _;
use std::time::{Duration, Instant};

use pdf_font::LoadedFont;
use pdf_syntax::{Document, ObjectId};

/// How long one load and 256 lookups may take on a loaded machine: about a hundred times what
/// they take, and a seventh of what the 5125-glyph fixture took searched per name (ADR 1584).
const CEILING: Duration = Duration::from_secs(1);

/// The 258 names a version 2.0 index below 258 refers to instead of a string of its own.
const MACINTOSH_NAMES: u16 = 258;

/// The name the fixture's table gives glyph `glyph`, which no glyph list holds.
fn listed(glyph: u16) -> String {
    format!("g{glyph}.post")
}

/// A name of the same shape that the fixture's table gives no glyph.
fn unlisted(code: u8) -> String {
    format!("absent{code}.post")
}

/// How many glyphs the fixture's table names with a string of its own.
///
/// Every glyph after `.notdef`, as far as a 16-bit index reaches: 65 535 less the 258 standard
/// names is the last string an index can name, so a table of the format's largest leaves its last
/// 256 glyphs `.notdef`.
fn named(glyphs: u16) -> u16 {
    (glyphs - 1).min(u16::MAX - MACINTOSH_NAMES + 1)
}

/// The glyph code `code` is drawn with: the last 128 the table names, so each search runs the
/// table's length.
fn glyph_of(glyphs: u16, code: u8) -> u16 {
    named(glyphs) - 127 + u16::from(code)
}

/// A version 2.0 `post` table naming each of [`named`]'s glyphs with a string of its own.
fn post(glyphs: u16) -> Vec<u8> {
    let mut table = vec![0_u8; 32];
    table.splice(0..4, 0x0002_0000_u32.to_be_bytes());
    table.extend_from_slice(&glyphs.to_be_bytes());
    table.extend_from_slice(&0_u16.to_be_bytes());
    for glyph in 1..glyphs {
        let index = if glyph <= named(glyphs) {
            glyph - 1 + MACINTOSH_NAMES
        } else {
            0
        };
        table.extend_from_slice(&index.to_be_bytes());
    }
    for glyph in 1..=named(glyphs) {
        let name = listed(glyph);
        table.push(u8::try_from(name.len()).expect("a short name"));
        table.extend_from_slice(name.as_bytes());
    }
    table
}

/// A `head` stating 1000 units per em, the one field a load reads before the names.
fn head() -> Vec<u8> {
    let mut table = vec![0_u8; 54];
    table.splice(0..4, 0x0001_0000_u32.to_be_bytes());
    table.splice(12..16, 0x5F0F_3CF5_u32.to_be_bytes());
    table.splice(18..20, 1000_u16.to_be_bytes());
    table
}

/// A version 0.5 `maxp`, which states the glyph count and nothing else.
fn maxp(glyphs: u16) -> Vec<u8> {
    let mut table = 0x0000_5000_u32.to_be_bytes().to_vec();
    table.extend_from_slice(&glyphs.to_be_bytes());
    table
}

/// A `cmap` with one empty (3, 1) subtable, so that no name reaches a glyph by its character.
fn cmap() -> Vec<u8> {
    let mut table = Vec::new();
    for field in [0_u16, 1, 3, 1] {
        table.extend_from_slice(&field.to_be_bytes());
    }
    table.extend_from_slice(&12_u32.to_be_bytes());
    for field in [6_u16, 10, 0, 0, 0] {
        table.extend_from_slice(&field.to_be_bytes());
    }
    table
}

/// An sfnt holding the tables given, each on a four-byte boundary.
fn sfnt(tables: &[([u8; 4], Vec<u8>)]) -> Vec<u8> {
    let mut out = 0x0001_0000_u32.to_be_bytes().to_vec();
    let count = u16::try_from(tables.len()).expect("a handful of tables");
    for field in [count, 0, 0, 0] {
        out.extend_from_slice(&field.to_be_bytes());
    }
    let directory = 12 + 16 * tables.len();
    let mut body = Vec::new();
    for (tag, data) in tables {
        out.extend_from_slice(tag);
        out.extend_from_slice(&0_u32.to_be_bytes());
        let offset = u32::try_from(directory + body.len()).expect("a small file");
        out.extend_from_slice(&offset.to_be_bytes());
        let length = u32::try_from(data.len()).expect("a small table");
        out.extend_from_slice(&length.to_be_bytes());
        body.extend_from_slice(data);
        body.resize(body.len().next_multiple_of(4), 0);
    }
    out.extend_from_slice(&body);
    out
}

/// A one-font document: a `TrueType` simple font whose `/Differences` give each code a name.
fn document(glyphs: u16) -> (Document, pdf_syntax::Dictionary) {
    let mut differences = String::from("0");
    for code in 0..=u8::MAX {
        let name = if code < 128 {
            listed(glyph_of(glyphs, code))
        } else {
            unlisted(code)
        };
        let _ = write!(differences, " /{name}");
    }
    let program = sfnt(&[
        (*b"cmap", cmap()),
        (*b"head", head()),
        (*b"maxp", maxp(glyphs)),
        (*b"post", post(glyphs)),
    ]);

    let objects: [Vec<u8>; 2] = [
        format!(
            "1 0 obj\n<< /Type /Font /Subtype /TrueType /BaseFont /Fixture \
             /Encoding << /Differences [{differences}] >> \
             /FontDescriptor << /Flags 32 /FontFile2 2 0 R >> >>\nendobj\n"
        )
        .into_bytes(),
        [
            format!("2 0 obj\n<< /Length {} >>\nstream\n", program.len()).into_bytes(),
            program,
            b"\nendstream\nendobj\n".to_vec(),
        ]
        .concat(),
    ];
    let mut out = b"%PDF-1.7\n".to_vec();
    let mut offsets = Vec::new();
    for object in &objects {
        offsets.push(out.len());
        out.extend_from_slice(object);
    }
    let xref_at = out.len();
    let mut xref = String::from("xref\n0 3\n0000000000 65535 f \n");
    for offset in offsets {
        let _ = writeln!(xref, "{offset:010} 00000 n ");
    }
    let _ = write!(
        xref,
        "trailer\n<< /Size 3 /Root 1 0 R >>\nstartxref\n{xref_at}\n%%EOF\n"
    );
    out.extend_from_slice(xref.as_bytes());

    let document = Document::open(out).expect("the fixture is a valid PDF");
    let dict = document
        .get(ObjectId {
            number: 1,
            generation: 0,
        })
        .as_dict()
        .expect("object 1 is the font dictionary")
        .clone();
    (document, dict)
}

/// Loads the font, asks every code for its glyph, and returns how long that took.
fn load_and_ask(glyphs: u16) -> Duration {
    let (document, dict) = document(glyphs);
    let started = Instant::now();
    let font = LoadedFont::load(&document, &dict, "Fixture").expect("the fixture loads");
    for code in 0..128_u8 {
        let decoded = font.decode(&[code]);
        let [code_read] = decoded.as_slice() else {
            panic!("one byte is one code in a simple font");
        };
        assert_eq!(
            font.glyph_index(*code_read),
            Some(glyph_of(glyphs, code)),
            "code {code} is named {} and the post table names that glyph",
            listed(glyph_of(glyphs, code))
        );
    }
    for code in 128..=u8::MAX {
        let decoded = font.decode(&[code]);
        let [code_read] = decoded.as_slice() else {
            panic!("one byte is one code in a simple font");
        };
        // A name the table does not hold reaches no glyph by §9.6.5.4's routes; whatever the
        // processor's own tiers answer below them, it is not a glyph the table named.
        assert!(
            font.glyph_index(*code_read)
                .is_none_or(|glyph| glyph < glyph_of(glyphs, 0)),
            "code {code}'s name is in no table"
        );
    }
    started.elapsed()
}

/// The fuzzed units' own size: 5125 glyphs.
#[test]
fn a_five_thousand_glyph_post_table_is_searched_once() {
    let elapsed = load_and_ask(5125);
    assert!(elapsed < CEILING, "{elapsed:?} against {CEILING:?}");
}

/// The format's largest, where the search per name would cost hours.
#[test]
fn the_largest_post_table_the_format_allows_is_searched_once() {
    let elapsed = load_and_ask(u16::MAX);
    assert!(elapsed < CEILING, "{elapsed:?} against {CEILING:?}");
}
