//! Consumer regression for §9.9.1's instruction to ignore embedded-program `PaintType`.
//! Embedded program paint choices must not override PDF text rendering mode.

#![expect(
    clippy::arithmetic_side_effects,
    clippy::expect_used,
    clippy::indexing_slicing,
    clippy::panic,
    reason = "test code builds fixed, complete PDF and font-program fixtures"
)]

use std::io::Write as _;

use pdf_render::Command;
use pdf_syntax::Document;

// Small name-keyed CFF generated and independently parsed with FontTools; its Top DICT explicitly has PaintType 2
// and its A charstring draws a closed rectangle with a 500-unit advance.
const TYPE1C_PAINTTYPE_2_HEX: &str = "01000401000101010b5061696e7450726f62650001010114f81b02f81b03f818048d0c05c80f8be012cb11000101010c5061696e742050726f62650000000022000201010410f8880ef8888b16f7c0f888fbc0060e";

// Small CID-keyed CFF generated and independently parsed with FontTools; ROS is Adobe-Identity, CID 1 names A,
// FDArray/FDSelect select its private dictionary, and PaintType 2 is explicit in the Top DICT.
const CID_CFF_PAINTTYPE_2_HEX: &str = "01000401000101010b5061696e7450726f62650001010122f81bf81c8b0c1ef81d02f81d03f818048d0c058d0c22f00ff30c25f7140c24f61100040101060e192341646f62654964656e746974795061696e742050726f62655061696e7450726f62650000000001000000000201010410f8880ef8888b16f7c0f888fbc0060e0001010115f81e0c261e1c3f8b8b1e1c3f8b8b0c078bf72d12";

#[derive(Clone, Copy, Debug)]
enum Format {
    Type1,
    Type1C,
    CidFontType0C,
}

fn unhex(hex: &str) -> Vec<u8> {
    assert_eq!(hex.len() % 2, 0);
    hex.as_bytes()
        .chunks_exact(2)
        .map(|pair| {
            let digit = |byte: u8| -> u8 {
                match byte {
                    b'0'..=b'9' => byte.saturating_sub(b'0'),
                    b'a'..=b'f' => byte.saturating_sub(b'a').saturating_add(10),
                    _ => panic!("non-hex digit in fixed font program"),
                }
            };
            digit(pair[0])
                .saturating_mul(16)
                .saturating_add(digit(pair[1]))
        })
        .collect()
}

fn type1_number(value: i32) -> Vec<u8> {
    match value {
        -107..=107 => vec![u8::try_from(value.saturating_add(139)).expect("small number")],
        108..=1131 => {
            let shifted = value.saturating_sub(108);
            vec![
                u8::try_from((shifted / 256).saturating_add(247)).expect("positive number"),
                u8::try_from(shifted % 256).expect("positive number"),
            ]
        }
        -1131..=-108 => {
            let shifted = value.saturating_add(108).saturating_neg();
            vec![
                u8::try_from((shifted / 256).saturating_add(251)).expect("negative number"),
                u8::try_from(shifted % 256).expect("negative number"),
            ]
        }
        _ => {
            let mut encoded = vec![255];
            encoded.extend_from_slice(&value.to_be_bytes());
            encoded
        }
    }
}

fn encrypt(bytes: &[u8], mut key: u16) -> Vec<u8> {
    bytes
        .iter()
        .map(|byte| {
            let cipher = *byte ^ u8::try_from(key >> 8).expect("shifted key fits a byte");
            key = u16::from(cipher)
                .wrapping_add(key)
                .wrapping_mul(52_845)
                .wrapping_add(22_719);
            cipher
        })
        .collect()
}

fn type1_program_with_painttype_2() -> (Vec<u8>, usize, usize) {
    // 0 500 hsbw; draw an explicitly closed rectangular outline for A.
    let mut charstring = type1_number(0);
    charstring.extend(type1_number(500));
    charstring.push(13); // hsbw
    for (x, y, op) in [
        (0, 0, 21),   // rmoveto
        (300, 0, 5),  // rlineto
        (0, 500, 5),  // rlineto
        (-300, 0, 5), // rlineto
        (0, 0, 9),    // closepath
        (0, 0, 14),   // endchar
    ] {
        if op == 21 || op == 5 {
            charstring.extend(type1_number(x));
            charstring.extend(type1_number(y));
            charstring.push(op);
        } else {
            charstring.push(op);
        }
    }

    let encrypt_charstring = |body: &[u8]| {
        let mut plain = vec![0; 4];
        plain.extend_from_slice(body);
        encrypt(&plain, 4330)
    };
    let notdef = encrypt_charstring(&[139, 139, 13, 14]);
    let a = encrypt_charstring(&charstring);

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
    private.extend_from_slice(b" ND\n/A ");
    private.extend_from_slice(format!("{} RD ", a.len()).as_bytes());
    private.extend_from_slice(&a);
    private.extend_from_slice(
        b" ND\nend put\nend\ndup /FontName get exch definefont pop\nmark currentfile closefile\n",
    );

    let mut eexec_plain = vec![0xff; 4];
    eexec_plain.extend_from_slice(&private);
    let encrypted = encrypt(&eexec_plain, 55_665);
    let clear = b"%!PS-AdobeFont-1.0: Type1PaintProbe 1.0\n\
        11 dict dup begin\n\
        /FontName /Type1PaintProbe def\n\
        /FontType 1 def\n\
        /FontMatrix [0.001 0 0 0.001 0 0] readonly def\n\
        /FontBBox [0 0 300 500] readonly def\n\
        /PaintType 2 def\n\
        /StrokeWidth 40 def\n\
        /Encoding 256 array\n\
        0 1 255 {1 index exch /.notdef put} for\n\
        dup 65 /A put\n\
        readonly def\n\
        end\n\
        currentfile eexec\n";
    let mut program = clear.to_vec();
    program.extend_from_slice(&encrypted);
    (program, clear.len(), encrypted.len())
}

fn program_bytes(format: Format) -> (Vec<u8>, Option<(usize, usize)>) {
    match format {
        Format::Type1 => {
            let (program, clear, encrypted) = type1_program_with_painttype_2();
            (program, Some((clear, encrypted)))
        }
        Format::Type1C => (unhex(TYPE1C_PAINTTYPE_2_HEX), None),
        Format::CidFontType0C => (unhex(CID_CFF_PAINTTYPE_2_HEX), None),
    }
}

fn pdf(format: Format, program: &[u8], lengths: Option<(usize, usize)>, mode: u8) -> Vec<u8> {
    let code = if matches!(format, Format::CidFontType0C) {
        "<0001>"
    } else {
        "(A)"
    };
    let content = format!("BT /F1 48 Tf 1 0 0 RG 0 0 1 rg 40 40 Td {mode} Tr {code} Tj ET");
    let font = match format {
        Format::Type1 => b"<< /Type /Font /Subtype /Type1 /BaseFont /Type1PaintProbe /Encoding /WinAnsiEncoding /FirstChar 65 /LastChar 65 /Widths [500] /FontDescriptor 6 0 R >>".to_vec(),
        Format::Type1C => b"<< /Type /Font /Subtype /Type1 /BaseFont /PaintProbe /Encoding /WinAnsiEncoding /FirstChar 65 /LastChar 65 /Widths [500] /FontDescriptor 6 0 R >>".to_vec(),
        Format::CidFontType0C => b"<< /Type /Font /Subtype /Type0 /BaseFont /PaintProbe /Encoding /Identity-H /DescendantFonts [6 0 R] >>".to_vec(),
    };
    let font_child = match format {
        Format::Type1 => b"<< /Type /FontDescriptor /FontName /Type1PaintProbe /Flags 32 /FontBBox [0 0 300 500] /ItalicAngle 0 /Ascent 500 /Descent 0 /CapHeight 500 /StemV 80 /FontFile 7 0 R >>".to_vec(),
        Format::Type1C => b"<< /Type /FontDescriptor /FontName /PaintProbe /Flags 32 /FontBBox [0 0 300 500] /ItalicAngle 0 /Ascent 500 /Descent 0 /CapHeight 500 /StemV 80 /FontFile3 7 0 R >>".to_vec(),
        Format::CidFontType0C => b"<< /Type /Font /Subtype /CIDFontType0 /BaseFont /PaintProbe /CIDSystemInfo << /Registry (Adobe) /Ordering (Identity) /Supplement 0 >> /FontDescriptor 7 0 R /DW 500 /W [1 [500]] >>".to_vec(),
    };
    let descriptor = match format {
        Format::Type1 | Format::Type1C => None,
        Format::CidFontType0C => Some(b"<< /Type /FontDescriptor /FontName /PaintProbe /Flags 32 /FontBBox [0 0 300 500] /ItalicAngle 0 /Ascent 500 /Descent 0 /CapHeight 500 /StemV 80 /FontFile3 8 0 R >>".to_vec()),
    };
    let stream_id = if matches!(format, Format::CidFontType0C) {
        8
    } else {
        7
    };
    let stream_dict = match (format, lengths) {
        (Format::Type1, Some((clear, encrypted))) => format!(
            "<< /Length {} /Length1 {clear} /Length2 {encrypted} /Length3 0 >>\nstream\n",
            program.len()
        )
        .into_bytes(),
        (Format::Type1C, None) => {
            format!("<< /Length {} /Subtype /Type1C >>\nstream\n", program.len()).into_bytes()
        }
        (Format::CidFontType0C, None) => format!(
            "<< /Length {} /Subtype /CIDFontType0C >>\nstream\n",
            program.len()
        )
        .into_bytes(),
        _ => panic!("font format and stream lengths disagree"),
    };
    let mut font_stream = stream_dict;
    font_stream.extend_from_slice(program);
    font_stream.extend_from_slice(b"\nendstream");

    let mut objects: Vec<Vec<u8>> = vec![
        b"<< /Type /Catalog /Pages 2 0 R >>".to_vec(),
        b"<< /Type /Pages /Kids [3 0 R] /Count 1 >>".to_vec(),
        b"<< /Type /Page /Parent 2 0 R /MediaBox [0 0 120 120] /Resources << /Font << /F1 5 0 R >> >> /Contents 4 0 R >>".to_vec(),
        format!("<< /Length {} >>\nstream\n{content}\nendstream", content.len()).into_bytes(),
        font,
        font_child,
    ];
    if let Some(descriptor) = descriptor {
        objects.push(descriptor);
    }
    assert_eq!(
        objects.len() + 1,
        stream_id,
        "stream object number is sequential"
    );
    objects.push(font_stream);

    let mut out = b"%PDF-1.7\n".to_vec();
    let mut offsets = Vec::new();
    for (index, object) in objects.iter().enumerate() {
        offsets.push(out.len());
        out.extend_from_slice(format!("{} 0 obj\n", index + 1).as_bytes());
        out.extend_from_slice(object);
        out.extend_from_slice(b"\nendobj\n");
    }
    let xref_at = out.len();
    out.extend_from_slice(
        format!("xref\n0 {}\n0000000000 65535 f \n", objects.len() + 1).as_bytes(),
    );
    for offset in offsets {
        out.extend_from_slice(format!("{offset:010} 00000 n \n").as_bytes());
    }
    let _ = write!(
        out,
        "trailer\n<< /Size {} /Root 1 0 R >>\nstartxref\n{xref_at}\n%%EOF\n",
        objects.len() + 1
    );
    out
}

fn commands(format: Format, mode: u8) -> Vec<&'static str> {
    let (program, lengths) = program_bytes(format);
    let document =
        Document::open(pdf(format, &program, lengths, mode)).expect("complete PDF opens");
    assert!(
        !document.was_recovered(),
        "the fixture has a complete xref table"
    );
    let page = pdf_model::Pages::new(&document).get(0).expect("one page");
    let interpretation = pdf_model::interpret(&document, &page);
    assert!(
        interpretation.unsupported.is_empty(),
        "fixture has no unsupported operations: {:?}",
        interpretation.unsupported
    );
    assert_eq!(interpretation.glyphs, 1, "the embedded glyph is shown");

    let resource = document.get_key(&page.resources, "Font");
    let fonts = resource.as_dict().expect("page has font resources");
    let font_object = document.resolve(&document.get_key(fonts, "F1"));
    let font_dict = font_object.as_dict().expect("F1 is a font dictionary");
    let font = pdf_font::LoadedFont::load(&document, font_dict, "F1").expect("embedded font loads");
    let bytes = if matches!(format, Format::CidFontType0C) {
        &[0x00, 0x01][..]
    } else {
        b"A"
    };
    let code = *font.decode(bytes).first().expect("one character code");
    assert!(
        font.outline(code)
            .is_some_and(|path| !path.commands().is_empty()),
        "program provides a nonempty A outline"
    );
    assert_eq!(
        font.program_advance(code),
        Some(0.5),
        "the loaded embedded program states a half-em advance"
    );

    interpretation
        .display_list
        .commands()
        .iter()
        .map(|command| match command {
            Command::Fill { .. } => "fill",
            Command::Stroke { .. } => "stroke",
            _ => "other",
        })
        .collect()
}

#[test]
fn embedded_painttype_2_is_ignored_for_type1_type1c_and_cidfonttype0c() {
    // Each program explicitly says PaintType 2 (stroke), but PDF Tr 0 still fills and Tr 1
    // strokes. program_advance plus a nonempty outline ensure the tested program was actually
    // loaded; the ordinary PDF rendering-mode controls are applied to the same glyph path.
    for format in [Format::Type1, Format::Type1C, Format::CidFontType0C] {
        assert_eq!(
            commands(format, 0),
            ["fill"],
            "PDF Tr 0 fills for {format:?}"
        );
        assert_eq!(
            commands(format, 1),
            ["stroke"],
            "PDF Tr 1 strokes for {format:?}"
        );
    }
}
