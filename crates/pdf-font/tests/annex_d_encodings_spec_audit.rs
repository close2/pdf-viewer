//! Audit 2260 of ISO 32000-2 Annex D against `pdf-font`: which names are predefined encodings.
//!
//! Each test pairs a control that passes on main with the case the audit found.

#![expect(
    clippy::expect_used,
    clippy::panic,
    clippy::arithmetic_side_effects,
    reason = "test fixtures are deliberately small; malformed fixtures should fail loudly"
)]
#![expect(
    clippy::doc_markdown,
    reason = "the specification's sentences are quoted verbatim, encoding names unbackticked as it \
              prints them"
)]

use pdf_font::{Code, LoadedFont};
use pdf_syntax::{Document, ObjectId};

/// A Type 1 charstring number (Adobe Type 1 Font Format, section 6.2), for the small values used.
fn number(value: i32) -> Vec<u8> {
    match value {
        -107..=107 => vec![u8::try_from(value + 139).expect("a small number")],
        108..=1131 => {
            let shifted = value - 108;
            vec![
                u8::try_from(shifted / 256 + 247).expect("a positive number"),
                u8::try_from(shifted % 256).expect("a byte"),
            ]
        }
        -1131..=-108 => {
            let shifted = -(value + 108);
            vec![
                u8::try_from(shifted / 256 + 251).expect("a negative number"),
                u8::try_from(shifted % 256).expect("a byte"),
            ]
        }
        _ => panic!("the fixture uses no larger number"),
    }
}

/// The Type 1 encryption (Adobe Type 1 Font Format, section 7.1) with the key given.
fn encrypt(bytes: &[u8], mut key: u16) -> Vec<u8> {
    bytes
        .iter()
        .map(|byte| {
            let cipher = *byte ^ u8::try_from(key >> 8).expect("a shifted key fits a byte");
            key = u16::from(cipher)
                .wrapping_add(key)
                .wrapping_mul(52_845)
                .wrapping_add(22_719);
            cipher
        })
        .collect()
}

/// A Type 1 program with two glyphs, `.notdef` and `A`, whose built-in encoding puts `A` at code
/// 66 — where `StandardEncoding` has `B` — and nothing else. Returns the program and the lengths
/// of its clear and encrypted parts.
fn program() -> (Vec<u8>, usize, usize) {
    let mut a = number(0);
    a.extend(number(500));
    a.push(13); // hsbw
    a.extend(number(0));
    a.extend(number(0));
    a.push(21); // rmoveto
    for (x, y) in [(300, 0), (0, 500), (-300, 0)] {
        a.extend(number(x));
        a.extend(number(y));
        a.push(5); // rlineto
    }
    a.extend([9, 14]); // closepath endchar
    let charstring = |body: &[u8]| {
        let mut plain = vec![0; 4];
        plain.extend_from_slice(body);
        encrypt(&plain, 4330)
    };
    let notdef = charstring(&[139, 139, 13, 14]);
    let a = charstring(&a);
    let mut private = b"dup /Private 8 dict dup begin\n\
        /RD{string currentfile exch readstring pop}executeonly def\n\
        /ND{noaccess def}executeonly def\n\
        /NP{noaccess put}executeonly def\n\
        /lenIV 4 def\n\
        /Subrs 0 array def\n\
        put\n\
        dup /CharStrings 2 dict dup begin\n"
        .to_vec();
    private.extend_from_slice(format!("/.notdef {} RD ", notdef.len()).as_bytes());
    private.extend_from_slice(&notdef);
    private.extend_from_slice(format!(" ND\n/A {} RD ", a.len()).as_bytes());
    private.extend_from_slice(&a);
    private.extend_from_slice(
        b" ND\nend put\nend\ndup /FontName get exch definefont pop\nmark currentfile closefile\n",
    );
    let mut plain = vec![0xff; 4];
    plain.extend_from_slice(&private);
    let encrypted = encrypt(&plain, 55_665);
    let clear = b"%!PS-AdobeFont-1.0: BuiltInProbe 1.0\n\
        11 dict dup begin\n\
        /FontName /BuiltInProbe def\n\
        /FontType 1 def\n\
        /FontMatrix [0.001 0 0 0.001 0 0] readonly def\n\
        /FontBBox [0 0 300 500] readonly def\n\
        /PaintType 0 def\n\
        /Encoding 256 array\n\
        0 1 255 {1 index exch /.notdef put} for\n\
        dup 66 /A put\n\
        readonly def\n\
        end\n\
        currentfile eexec\n";
    let mut program = clear.to_vec();
    program.extend_from_slice(&encrypted);
    (program, clear.len(), encrypted.len())
}

/// The embedded font with `encoding` written into its font dictionary, loaded as `F1`.
fn loaded(encoding: &str) -> LoadedFont {
    let (program, clear, encrypted) = program();
    let mut out = b"%PDF-1.7\n".to_vec();
    let font_at = out.len();
    out.extend_from_slice(
        format!(
            "1 0 obj\n<< /Type /Font /Subtype /Type1 /BaseFont /BuiltInProbe {encoding} \
             /FirstChar 66 /LastChar 66 /Widths [500] /FontDescriptor << /Type /FontDescriptor \
             /FontName /BuiltInProbe /Flags 32 /FontBBox [0 0 300 500] /ItalicAngle 0 \
             /Ascent 500 /Descent 0 /CapHeight 500 /StemV 80 /FontFile 2 0 R >> >>\nendobj\n"
        )
        .as_bytes(),
    );
    let stream_at = out.len();
    out.extend_from_slice(
        format!(
            "2 0 obj\n<< /Length {} /Length1 {clear} /Length2 {encrypted} /Length3 0 >>\nstream\n",
            program.len()
        )
        .as_bytes(),
    );
    out.extend_from_slice(&program);
    out.extend_from_slice(b"\nendstream\nendobj\n");
    let xref_at = out.len();
    out.extend_from_slice(
        format!(
            "xref\n0 3\n0000000000 65535 f \n{font_at:010} 00000 n \n{stream_at:010} 00000 n \n\
             trailer\n<< /Size 3 /Root 1 0 R >>\nstartxref\n{xref_at}\n%%EOF\n"
        )
        .as_bytes(),
    );
    let document = Document::open(out).expect("the fixture is a valid PDF");
    let dict = document
        .get(ObjectId {
            number: 1,
            generation: 0,
        })
        .as_dict()
        .expect("object 1 is the font dictionary")
        .clone();
    LoadedFont::load(&document, &dict, "F1").expect("the fixture font loads")
}

/// Annex D, Table D.1, of `StandardEncoding`: "PDF processors shall not have a predefined
/// encoding named StandardEncoding. However, it is necessary to describe this encoding, since a
/// font's built-in encoding can be used as the base encoding from which differences may be
/// specified in an encoding dictionary." Table 109 lists the names an `/Encoding` may hold, and
/// this crate treats a name outside them as no `/Encoding` at all, the program's built-in
/// encoding standing (`glyph_names::base_encoding`). `StandardEncoding` is such a name, so code
/// 66 of a program whose built-in encoding puts `A` there reaches `A`, as it does with no entry.
#[test]
fn standard_encoding_is_no_predefined_encoding_spec_audit() {
    let code = Code::single_byte(66);
    let built_in = loaded("").glyph_index(code);
    assert!(
        built_in.is_some_and(|glyph| glyph != 0),
        "control: the program's built-in encoding reaches A at code 66, got {built_in:?}"
    );
    assert_eq!(
        loaded("/Encoding /NotAnEncoding").glyph_index(code),
        built_in,
        "control: a name Table 109 does not list leaves the built-in encoding standing"
    );
    assert_eq!(
        loaded("/Encoding /StandardEncoding").glyph_index(code),
        built_in,
        "Table D.1: a processor shall not have a predefined encoding named StandardEncoding, so \
         the name leaves the program's built-in encoding standing, which puts A at code 66"
    );
}
