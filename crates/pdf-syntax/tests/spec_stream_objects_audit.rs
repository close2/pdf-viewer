//! §7.3.8's external-stream boundary and §7.4.1's corrected Example 3.

#![expect(
    clippy::arithmetic_side_effects,
    reason = "test fixtures are complete, in-memory PDFs with small checked offsets"
)]

use std::fmt::Write as _;

use pdf_syntax::filter::Pumped;
use pdf_syntax::{Document, ObjectId, StreamRefusal, StreamSource};

/// A complete one-page PDF with `content` as object 4 and an optional font object 5.
fn one_page(content_dict: &str, data: &[u8]) -> Vec<u8> {
    let mut bytes = b"%PDF-1.7\n".to_vec();
    let mut offsets = Vec::new();
    let bodies: [&[u8]; 4] = [
        b"<< /Type /Catalog /Pages 2 0 R >>",
        b"<< /Type /Pages /Count 1 /Kids [3 0 R] >>",
        b"<< /Type /Page /Parent 2 0 R /MediaBox [0 0 612 792] /Resources << /Font << /F1 5 0 R >> >> /Contents 4 0 R >>",
        b"",
    ];
    for (index, body) in bodies[..3].iter().enumerate() {
        offsets.push(bytes.len());
        let _ = writeln!(StringWriter(&mut bytes), "{} 0 obj", index + 1);
        bytes.extend_from_slice(body);
        bytes.extend_from_slice(b"\nendobj\n");
    }
    offsets.push(bytes.len());
    let _ = write!(
        StringWriter(&mut bytes),
        "4 0 obj\n<< {content_dict} /Length {} >>\nstream\n",
        data.len()
    );
    bytes.extend_from_slice(data);
    bytes.extend_from_slice(b"\nendstream\nendobj\n");
    offsets.push(bytes.len());
    bytes.extend_from_slice(
        b"5 0 obj\n<< /Type /Font /Subtype /Type1 /BaseFont /Helvetica >>\nendobj\n",
    );

    let xref = bytes.len();
    bytes.extend_from_slice(b"xref\n0 6\n0000000000 65535 f \n");
    for offset in offsets {
        let _ = writeln!(StringWriter(&mut bytes), "{offset:010} 00000 n ");
    }
    let _ = write!(
        StringWriter(&mut bytes),
        "trailer\n<< /Size 6 /Root 1 0 R >>\nstartxref\n{xref}\n%%EOF\n"
    );
    bytes
}

/// Appends fixture text to a byte vector.
struct StringWriter<'a>(&'a mut Vec<u8>);

impl std::fmt::Write for StringWriter<'_> {
    fn write_str(&mut self, text: &str) -> std::fmt::Result {
        self.0.extend_from_slice(text.as_bytes());
        Ok(())
    }
}

/// §7.3.8.1: `/F` moves the bytes elsewhere, so the embedded sequence is not stream data.
#[test]
fn an_external_stream_is_refused_while_the_same_embedded_filter_decodes_without_f() {
    let content = b"0 0 m 10 10 l S";
    let mut encoded: Vec<u8> = content
        .iter()
        .flat_map(|byte| format!("{byte:02X}").into_bytes())
        .collect();
    encoded.push(b'>');
    // Positive control first: identical encoded bytes and ordinary Filter decode as the
    // content the page's producer wrote.
    let control = Document::open(one_page("/Filter /ASCIIHexDecode", &encoded))
        .expect("the control PDF is structurally valid");
    assert!(!control.was_recovered(), "the control xref is exact");
    let stream = control
        .get(ObjectId::new(4, 0))
        .as_stream()
        .expect("the control page contents is a stream")
        .clone();
    match control.stream_source(&stream).expect("the control decodes") {
        StreamSource::Whole(decoded) => assert_eq!(&*decoded.data, content),
        StreamSource::Pumped { mut pump, .. } => {
            let mut bytes = [0; 64];
            assert_eq!(pump.pump(&mut bytes), Pumped::Ended(content.len()));
            assert_eq!(&bytes[..content.len()], content);
        }
        StreamSource::Refused { .. } => panic!("the valid control must decode"),
    }

    let external = Document::open(one_page(
        "/Filter /ASCIIHexDecode /F (outside.dat) /FFilter /ASCIIHexDecode",
        &encoded,
    ))
    .expect("the external-stream PDF is structurally valid");
    assert!(!external.was_recovered(), "the fixture xref is exact");
    let stream = external
        .get(ObjectId::new(4, 0))
        .as_stream()
        .expect("the page contents is a stream")
        .clone();
    assert_eq!(
        external.stream_source(&stream).err(),
        Some(StreamRefusal::External),
        "embedded bytes are ignored when /F states the external data source"
    );
    assert_eq!(
        external.decoded_stream_data(&stream),
        None,
        "the reader must not pass embedded bytes off as external stream data"
    );
    assert_eq!(
        external.nested_content_source(&stream).err(),
        Some(StreamRefusal::External),
        "the nested-content route also refuses external bytes"
    );
}

/// The corrected PDF 2.0 §7.4.1 Example 3 bytes (Issue #527 adds `~>` and changes Length 447 to 449).
const EXAMPLE_3: &str = r#"GhQ\j_/=ii'LfceJ%/-bPR=PCD-LeMX!Z7pmG\.BSJGfG1MH^`^(F$rh*;h>h/i!XMIl:?"
o%Tk8W%<tN0"eRE==1.4N>=Cc@SiF+*s\3`snB>Y$n=^$I[!5nkk"\Wp1;WBoI"]#[bZ=Ou#c
QqtW?@3^VI[CftgH<HWngT*U:T12>8lm%+Q=gI^i0+\=:@fc@rHeGG!&rPQYfZ>YdpV`eM._@ Z]46KR;P?sT-tkq-
Xn4&]=EE2[h<FPBAePZJ9inNNF4R*)W-?\NH1C-GG&?=IC_>b%&++C2jo
*B+a7eU&`hee<0<F%-Y]De8<t.o"7p,rtNedLjK.J`kSL@c;BDEq+;F1Y?O)>keNVh+-(1*S=
CQ?fQ_I]jWNc*Qe2XR^R#;3:<$\m;6d>@C3egX,+W+VfZ]X4*3r5*tiejhQe:!'jq$^jfSIj%
13FL"#;

/// The actual corrected example passes through a complete page's stream reader.
#[test]
fn corrected_example_3_counts_and_preserves_both_ascii85_eod_bytes() {
    let payload = EXAMPLE_3.trim().as_bytes();
    let mut encoded = payload.to_vec();
    encoded.extend_from_slice(b"~>");
    assert_eq!(
        encoded.len(),
        449,
        "Issue #527's corrected Length includes ~>"
    );

    // Positive control first: the example's page-content operators are readable without a
    // filter, in a complete PDF with the font the stream names.
    let plain = b"BT /F1 12 Tf 72 712 Td (control) Tj ET";
    let control = Document::open(one_page("", plain)).expect("the control PDF is valid");
    assert!(!control.was_recovered(), "the control xref is exact");
    let control_stream = control
        .get(ObjectId::new(4, 0))
        .as_stream()
        .expect("control content stream")
        .clone();
    assert_eq!(&*control_stream.data, plain);

    let document = Document::open(one_page("/Filter [/ASCII85Decode /FlateDecode]", &encoded))
        .expect("the corrected Example 3 PDF is valid");
    assert!(!document.was_recovered(), "the example xref is exact");
    let stream = document
        .get(ObjectId::new(4, 0))
        .as_stream()
        .expect("the example content stream")
        .clone();
    assert_eq!(
        stream.data.as_ref(),
        encoded.as_slice(),
        "Issue #319: EOD is part of the encoded extent"
    );
    let decoded = document
        .decoded_stream_data(&stream)
        .expect("ASCII85 then Flate are both applied");
    assert!(decoded.starts_with(b"2 J\rBT\r/F1 12 Tf"));
    assert!(
        decoded
            .windows(37)
            .any(|window| window == b"Unfiltered streams can be read easily")
    );
}
