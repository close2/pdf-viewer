//! A scanned page with an invisible OCR text layer, in the two shapes one producer wrote.
//!
//! `OmniPage` CSDK writes scanned pages as an image with the recognised text behind it in
//! rendering mode 3 — Table 104's "Neither fill nor stroke text (invisible)" — through an
//! `Identity-H` `CIDFontType2`. Version 20 embedded no font program and stated `/CIDToGIDMap
//! /Identity`; version 22 embeds a **hollow** program — `head`, `hmtx` and a well-formed
//! `loca` present, every `glyf` entry empty — with a `/CIDToGIDMap` *remapping stream*
//! (§9.7.4.1 Table 115). The text extracts identically from both; what the pair separates is
//! every consumer that could be tempted to derive a character's *vertical extent* from the
//! glyph outlines, which the hollow program states as zero for every glyph. A reader that
//! does so loses its selection overlay and its hit band on every CSDK 22 scan (old `PDFium`
//! did, from `FPDFText_GetCharBox`); one that answers from Table 120's `/Ascent` and
//! `/Descent` — which both shapes state — does not. ADR 0350 is the finding this pair pins.
//!
//! Hand-built per trap 8: no corpus document carries the hollow-program-plus-stream shape,
//! and the two fixtures differ only in the rule under test — same text, same layout, same
//! descriptor, same image.

use std::fmt::Write as _;

use crate::assemble_pdf;

/// The fixture page's size in points, stated where tests can measure against it.
pub const OCR_PAGE: (f32, f32) = (300.0, 200.0);

/// The font size the invisible layer is shown at.
pub const OCR_FONT_SIZE: f32 = 12.0;

/// The descriptor's `/Ascent`, in glyph-space units (Table 120).
pub const OCR_ASCENT: f32 = 718.0;

/// The descriptor's `/Descent`, in glyph-space units (Table 120).
pub const OCR_DESCENT: f32 = -207.0;

/// The baseline the two words sit on, in user space.
pub const OCR_BASELINE: f32 = 150.0;

/// The left edge of the first word, in user space.
pub const OCR_TEXT_X: f32 = 40.0;

/// Where the second word's line origin is placed, in user space.
///
/// Past the first word's 39.6 pt end by more than the inferred-word-break threshold (0.6 of
/// the space advance, which `/DW 1000` makes 7.2 pt at this size), so the readback carries
/// the word break a scanned page's layout implies.
pub const OCR_SECOND_X: f32 = 92.0;

/// The first word, CIDs 1 to 6 in the show string.
pub const OCR_FIRST_WORD: &str = "Hollow";

/// The second word, CIDs 7 to 10.
pub const OCR_SECOND_WORD: &str = "scan";

/// How many CIDs the layer shows: every character of both words, one CID each.
pub const OCR_CID_COUNT: u16 = 10;

/// Glyphs in the hollow program: the `.notdef` at index 0 plus one per CID.
const GLYPH_COUNT: u16 = OCR_CID_COUNT + 1;

/// The remapping the `/CIDToGIDMap` stream states: CID `c` reaches glyph `11 − c`.
///
/// Deliberately not the identity for any CID it covers (`11 − c = c` has no integer solution
/// here), so a reader that ignored the stream and took the CID as the glyph index would land
/// on a different glyph for every code — which
/// [`ocr_advance_for_gid`]'s distinct advances turn into a measurable disagreement with `/W`.
#[must_use]
pub fn ocr_gid_for_cid(cid: u16) -> u16 {
    GLYPH_COUNT.saturating_sub(cid)
}

/// The hollow program's own `hmtx` advance for a glyph, in glyph-space units.
///
/// Distinct per glyph so that the document's `/W` (keyed by CID) and the program's `hmtx`
/// (keyed by glyph index) agree only through the remapping stream.
#[must_use]
pub fn ocr_advance_for_gid(gid: u16) -> u16 {
    400_u16.saturating_add(gid.saturating_mul(20))
}

/// Which font the OCR layer's `CIDFontType2` carries, the pair's one difference.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OcrFont {
    /// CSDK 22's shape: an embedded `/FontFile2` whose every `glyf` entry is empty, with a
    /// `/CIDToGIDMap` remapping stream.
    HollowEmbedded,
    /// CSDK 20's shape: no font program and `/CIDToGIDMap /Identity`, so a reader draws it
    /// through a substitute reached by what the codes mean (§9.7.4.2).
    NotEmbedded,
}

/// Builds the scanned-page fixture: an image "scan" with the OCR text layer over it.
///
/// `render_mode` is Table 104's: `3` is the producer's own invisible layer, and `0` is the
/// visible control — the same hollow font with a mark *owed* for every code, which is what
/// separates "nothing was drawn because nothing paints in mode 3" from "nothing was drawn
/// because the program is hollow" in `Interpretation`'s accounting (ADR 0270).
#[must_use]
#[expect(
    clippy::arithmetic_side_effects,
    reason = "every quantity here is bounded by the fixture's own ten-character text"
)]
pub fn scanned_ocr_pdf(font: OcrFont, render_mode: u8) -> Vec<u8> {
    let (width, height) = OCR_PAGE;

    // The stand-in scan: an 8×8 grey checker drawn over the whole page.
    let image_data: Vec<u8> = (0..64_u8)
        .map(|i| if (i / 8 + i % 8) % 2 == 0 { 0xF0 } else { 0xC8 })
        .collect();

    let mut content = format!("q {width} 0 0 {height} 0 0 cm /Im0 Do Q\n");
    let hex = |first: u16, last: u16| {
        let mut string = String::from("<");
        for cid in first..=last {
            let _ = write!(string, "{cid:04X}");
        }
        string.push('>');
        string
    };
    let _ = write!(
        content,
        "BT /F0 {OCR_FONT_SIZE} Tf {render_mode} Tr {OCR_TEXT_X} {OCR_BASELINE} Td {} Tj \
         {} 0 Td {} Tj ET",
        hex(1, 6),
        OCR_SECOND_X - OCR_TEXT_X,
        hex(7, OCR_CID_COUNT),
    );
    let content = content.into_bytes();

    // §9.7.4.3's /W, keyed by CID: each width is the hollow program's own hmtx advance for
    // the glyph the *stream* maps that CID to, so the two statements of one fact agree only
    // through the remapping — the same cross-check `pdf-model`'s composite-font tests run.
    let mut widths = String::from("[1 [");
    for cid in 1..=OCR_CID_COUNT {
        let _ = write!(widths, "{} ", ocr_advance_for_gid(ocr_gid_for_cid(cid)));
    }
    widths.push_str("]]");

    // §9.10.2's first method, which is why extraction is identical across the pair: the
    // producer states what every CID means.
    let text: String = format!("{OCR_FIRST_WORD}{OCR_SECOND_WORD}");
    let mut bfchars = String::new();
    for (index, character) in text.chars().enumerate() {
        let _ = writeln!(bfchars, "<{:04X}> <{:04X}>", index + 1, character as u32);
    }
    let to_unicode = format!(
        "/CIDInit /ProcSet findresource begin\n12 dict begin\nbegincmap\n\
         /CMapName /Adobe-Identity-UCS def\n/CMapType 2 def\n\
         1 begincodespacerange\n<0000> <FFFF>\nendcodespacerange\n\
         {} beginbfchar\n{bfchars}endbfchar\nendcmap\n\
         CMapName currentdict /CMap defineresource pop\nend\nend\n",
        text.chars().count()
    )
    .into_bytes();

    // Table 115: "the glyph index for a particular CID value c shall be a 2-byte value
    // stored in bytes 2 × c and 2 × c + 1, where the first byte shall be the high-order
    // byte" — CID 0 maps to glyph 0, every shown CID to its remapped glyph.
    let mut cid_to_gid = Vec::with_capacity(usize::from(GLYPH_COUNT) * 2);
    cid_to_gid.extend_from_slice(&0_u16.to_be_bytes());
    for cid in 1..=OCR_CID_COUNT {
        cid_to_gid.extend_from_slice(&ocr_gid_for_cid(cid).to_be_bytes());
    }

    let stream = |dict: String, data: Vec<u8>| {
        let mut object = format!("<< {dict} /Length {} >>\nstream\n", data.len()).into_bytes();
        object.extend_from_slice(&data);
        object.extend_from_slice(b"\nendstream");
        object
    };

    let descendant = format!(
        "<< /Type /Font /Subtype /CIDFontType2 /BaseFont /AAAAAA+OcrHollow \
         /CIDSystemInfo << /Registry (Adobe) /Ordering (Identity) /Supplement 0 >> \
         /FontDescriptor 9 0 R /DW 1000 /W {widths} /CIDToGIDMap {} >>",
        match font {
            OcrFont::HollowEmbedded => "10 0 R",
            OcrFont::NotEmbedded => "/Identity",
        }
    );

    let descriptor = format!(
        "<< /Type /FontDescriptor /FontName /AAAAAA+OcrHollow /Flags 4 \
         /FontBBox [0 {OCR_DESCENT} 600 {OCR_ASCENT}] /ItalicAngle 0 \
         /Ascent {OCR_ASCENT} /Descent {OCR_DESCENT} /CapHeight 700 /StemV 80{} >>",
        match font {
            OcrFont::HollowEmbedded => " /FontFile2 11 0 R",
            OcrFont::NotEmbedded => "",
        }
    );

    let mut objects: Vec<Vec<u8>> = vec![
        b"<< /Type /Catalog /Pages 2 0 R >>".to_vec(),
        b"<< /Type /Pages /Kids [3 0 R] /Count 1 >>".to_vec(),
        format!(
            "<< /Type /Page /Parent 2 0 R /MediaBox [0 0 {width} {height}] /Contents 4 0 R \
             /Resources << /XObject << /Im0 5 0 R >> /Font << /F0 6 0 R >> >> >>"
        )
        .into_bytes(),
        stream(String::new(), content),
        stream(
            "/Type /XObject /Subtype /Image /Width 8 /Height 8 /ColorSpace /DeviceGray \
             /BitsPerComponent 8"
                .to_owned(),
            image_data,
        ),
        b"<< /Type /Font /Subtype /Type0 /BaseFont /AAAAAA+OcrHollow /Encoding /Identity-H \
          /DescendantFonts [7 0 R] /ToUnicode 8 0 R >>"
            .to_vec(),
        descendant.into_bytes(),
        stream(String::new(), to_unicode),
        descriptor.into_bytes(),
    ];
    if font == OcrFont::HollowEmbedded {
        let program = hollow_truetype();
        objects.push(stream(String::new(), cid_to_gid));
        objects.push(stream(format!("/Length1 {}", program.len()), program));
    }

    assemble_pdf(&objects)
}

/// Builds the hollow `TrueType` program: real metrics, not one glyph outline.
///
/// `head`, `hhea`, `hmtx`, `maxp`, a well-formed all-zero short `loca` — every entry's start
/// equal to the next, which is the table's own statement that every glyph is empty — and a
/// zero-length `glyf`. The advances are [`ocr_advance_for_gid`]'s; the em square is 1000.
#[expect(
    clippy::arithmetic_side_effects,
    reason = "six fixed tables over eleven glyphs: every offset and length is bounded by \
              construction"
)]
fn hollow_truetype() -> Vec<u8> {
    let mut head = Vec::new();
    push_u32(&mut head, 0x0001_0000); // version 1.0
    push_u32(&mut head, 0x0001_0000); // fontRevision
    push_u32(&mut head, 0); // checkSumAdjustment: nothing here validates it
    push_u32(&mut head, 0x5F0F_3CF5); // magicNumber
    push_u16(&mut head, 0); // flags
    push_u16(&mut head, 1000); // unitsPerEm — §9.2.4's thousandths
    head.extend_from_slice(&[0; 16]); // created, modified
    for _ in 0..4 {
        push_u16(&mut head, 0); // xMin, yMin, xMax, yMax: no glyph has a box
    }
    push_u16(&mut head, 0); // macStyle
    push_u16(&mut head, 8); // lowestRecPPEM
    push_u16(&mut head, 2); // fontDirectionHint
    push_u16(&mut head, 0); // indexToLocFormat: short
    push_u16(&mut head, 0); // glyphDataFormat

    let mut hhea = Vec::new();
    push_u32(&mut hhea, 0x0001_0000);
    push_i16(&mut hhea, 718); // ascender: agrees with the descriptor, as a real subset's does
    push_i16(&mut hhea, -207); // descender
    push_i16(&mut hhea, 0); // lineGap
    push_u16(&mut hhea, ocr_advance_for_gid(GLYPH_COUNT - 1)); // advanceWidthMax
    for _ in 0..3 {
        push_i16(&mut hhea, 0); // minLeftSideBearing, minRightSideBearing, xMaxExtent
    }
    push_i16(&mut hhea, 1); // caretSlopeRise
    push_i16(&mut hhea, 0); // caretSlopeRun
    push_i16(&mut hhea, 0); // caretOffset
    for _ in 0..4 {
        push_i16(&mut hhea, 0); // reserved
    }
    push_i16(&mut hhea, 0); // metricDataFormat
    push_u16(&mut hhea, GLYPH_COUNT); // numberOfHMetrics

    let mut hmtx = Vec::new();
    for gid in 0..GLYPH_COUNT {
        push_u16(&mut hmtx, ocr_advance_for_gid(gid));
        push_i16(&mut hmtx, 0); // left side bearing
    }

    let mut maxp = Vec::new();
    push_u32(&mut maxp, 0x0001_0000);
    push_u16(&mut maxp, GLYPH_COUNT);
    for _ in 0..13 {
        push_u16(&mut maxp, 0); // maxPoints … maxComponentDepth: no glyph has any
    }

    // Short format: offsets divided by two, one per glyph plus the end sentinel, all zero.
    let loca = vec![0_u8; (usize::from(GLYPH_COUNT) + 1) * 2];
    let glyf = Vec::new();

    // Alphabetical, which is the directory's required order.
    let tables: [([u8; 4], Vec<u8>); 6] = [
        (*b"glyf", glyf),
        (*b"head", head),
        (*b"hhea", hhea),
        (*b"hmtx", hmtx),
        (*b"loca", loca),
        (*b"maxp", maxp),
    ];

    let count = tables.len();
    let mut largest_power = 1_usize;
    while largest_power * 2 <= count {
        largest_power *= 2;
    }
    let mut out = Vec::new();
    push_u32(&mut out, 0x0001_0000); // sfntVersion: TrueType outlines
    push_u16(&mut out, truncate_u16(count));
    push_u16(&mut out, truncate_u16(largest_power * 16));
    push_u16(&mut out, truncate_u16(largest_power.ilog2() as usize));
    push_u16(&mut out, truncate_u16(count * 16 - largest_power * 16));

    let mut offset = 12 + count * 16;
    for (tag, data) in &tables {
        out.extend_from_slice(tag);
        push_u32(&mut out, table_checksum(data));
        push_u32(&mut out, truncate_u32(offset));
        push_u32(&mut out, truncate_u32(data.len()));
        offset += data.len().next_multiple_of(4);
    }
    for (_, data) in &tables {
        out.extend_from_slice(data);
        out.resize(out.len().next_multiple_of(4), 0);
    }
    out
}

/// The sfnt table checksum: the sum of the table read as big-endian words, zero-padded.
fn table_checksum(data: &[u8]) -> u32 {
    let mut sum = 0_u32;
    for chunk in data.chunks(4) {
        let mut word = [0_u8; 4];
        if let Some(slot) = word.get_mut(..chunk.len()) {
            slot.copy_from_slice(chunk);
        }
        sum = sum.wrapping_add(u32::from_be_bytes(word));
    }
    sum
}

/// Narrows a fixture-bounded count into the field width the sfnt states for it.
#[expect(
    clippy::cast_possible_truncation,
    reason = "every caller passes a quantity bounded by six tables of a few dozen bytes"
)]
fn truncate_u16(value: usize) -> u16 {
    value as u16
}

/// As [`truncate_u16`], for the directory's 32-bit offsets and lengths.
#[expect(
    clippy::cast_possible_truncation,
    reason = "every caller passes a quantity bounded by six tables of a few dozen bytes"
)]
fn truncate_u32(value: usize) -> u32 {
    value as u32
}

fn push_u16(out: &mut Vec<u8>, value: u16) {
    out.extend_from_slice(&value.to_be_bytes());
}

fn push_i16(out: &mut Vec<u8>, value: i16) {
    out.extend_from_slice(&value.to_be_bytes());
}

fn push_u32(out: &mut Vec<u8>, value: u32) {
    out.extend_from_slice(&value.to_be_bytes());
}
