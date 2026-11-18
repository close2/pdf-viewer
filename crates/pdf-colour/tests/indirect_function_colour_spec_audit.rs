//! §7.3.10 indirect numeric values in function and colour-space dictionaries.
//!
//! Both fixtures are complete ordinary PDFs with exact classic-xref offsets. Function
//! expected values come from two literal 8-bit samples and the stated affine mappings;
//! `CalGray`'s expected neutral is sRGB encoding of linear 0.25.

#![expect(
    clippy::expect_used,
    clippy::float_cmp,
    reason = "small exact-xref fixtures; parsed literals and identical computations must be exactly equal"
)]

use pdf_colour::{colour::ColourSpace, function::Function};
use pdf_syntax::{Dictionary, Document, Object, ObjectId};

/// Complete one-page PDF with object 4 as harmless content, plus an optional stream object.
fn pdf_with_stream(
    stream_id: u32,
    stream_dictionary: &str,
    stream_bytes: &[u8],
    extra_objects: &[(u32, Vec<u8>)],
) -> Document {
    let mut objects: Vec<(u32, Vec<u8>, Option<Vec<u8>>)> = vec![
        (1, b"<< /Type /Catalog /Pages 2 0 R >>".to_vec(), None),
        (
            2,
            b"<< /Type /Pages /Kids [3 0 R] /Count 1 >>".to_vec(),
            None,
        ),
        (
            3,
            b"<< /Type /Page /Parent 2 0 R /MediaBox [0 0 10 10] /Contents 4 0 R >>".to_vec(),
            None,
        ),
        (4, b"<< /Length 3 >>".to_vec(), Some(b"q Q".to_vec())),
        (
            stream_id,
            format!("<< {stream_dictionary} /Length {} >>", stream_bytes.len()).into_bytes(),
            Some(stream_bytes.to_vec()),
        ),
    ];
    objects.extend(
        extra_objects
            .iter()
            .map(|(number, body)| (*number, body.clone(), None)),
    );
    objects.sort_by_key(|(number, _, _)| *number);

    let size = objects
        .last()
        .map_or(5usize, |(number, _, _)| *number as usize + 1);
    let mut offsets = vec![None; size];
    let mut bytes = b"%PDF-1.7\n".to_vec();
    for (number, body, stream) in objects {
        offsets[number as usize] = Some(bytes.len());
        bytes.extend_from_slice(format!("{number} 0 obj\n").as_bytes());
        bytes.extend_from_slice(&body);
        if let Some(data) = stream {
            bytes.extend_from_slice(b"\nstream\n");
            bytes.extend_from_slice(&data);
            bytes.extend_from_slice(b"\nendstream");
        }
        bytes.extend_from_slice(b"\nendobj\n");
    }

    let xref = bytes.len();
    bytes.extend_from_slice(format!("xref\n0 {size}\n0000000000 65535 f \n").as_bytes());
    for offset in offsets.iter().skip(1) {
        match offset {
            Some(offset) => bytes.extend_from_slice(format!("{offset:010} 00000 n \n").as_bytes()),
            None => bytes.extend_from_slice(b"0000000000 00000 f \n"),
        }
    }
    bytes.extend_from_slice(
        format!("trailer\n<< /Size {size} /Root 1 0 R >>\nstartxref\n{xref}\n%%EOF\n").as_bytes(),
    );
    let document = Document::open(bytes).expect("complete PDF with exact xref offsets");
    assert!(!document.was_recovered(), "fixture must use its xref table");
    document
}

fn function(document: &Document, number: u32) -> Function {
    Function::parse(
        document,
        &Object::Reference(ObjectId {
            number,
            generation: 0,
        }),
    )
    .expect("valid function parses")
}

/// Nondefault Domain, reversed Encode, Decode distinct from Range, and clipping make each
/// field observable. The raw samples are [0,255]. At input 3, the normalized sample is 0.5
/// and Decode [-1,1] gives 0; falling back to Range would give 0.25 instead.
#[test]
fn sampled_function_indirect_arrays_and_numeric_members_keep_the_direct_result() {
    let raw_samples = [0, 255];
    let direct = pdf_with_stream(
        5,
        "/FunctionType 0 /Domain [2 4] /Range [-0.25 0.75] /Size [2] \
         /BitsPerSample 8 /Encode [1 0] /Decode [-1 1]",
        &raw_samples,
        &[],
    );

    let indirect = pdf_with_stream(
        5,
        "/FunctionType 0 /Domain 6 0 R /Range 7 0 R /Size [2] \
         /BitsPerSample 8 /Encode 8 0 R /Decode 9 0 R",
        &raw_samples,
        &[
            (6, b"[10 0 R 11 0 R]".to_vec()),
            (7, b"[12 0 R 13 0 R]".to_vec()),
            (8, b"[14 0 R 15 0 R]".to_vec()),
            (9, b"[16 0 R 17 0 R]".to_vec()),
            (10, b"2".to_vec()),
            (11, b"4".to_vec()),
            (12, b"-0.25".to_vec()),
            (13, b"0.75".to_vec()),
            (14, b"1".to_vec()),
            (15, b"0".to_vec()),
            (16, b"-1".to_vec()),
            (17, b"1".to_vec()),
        ],
    );

    let direct_function = function(&direct, 5);
    let indirect_function = function(&indirect, 5);
    for (input, expected) in [
        (1.0, 0.75),
        (2.0, 0.75),
        (2.5, 0.5),
        (3.0, 0.0),
        (3.5, -0.25),
        (4.0, -0.25),
        (5.0, -0.25),
    ] {
        assert_eq!(
            direct_function.eval(&[input]),
            [expected],
            "direct input {input}"
        );
        assert_eq!(
            indirect_function.eval(&[input]),
            [expected],
            "indirect input {input}"
        );
    }
}

/// Table 62's `WhitePoint` is a three-number array and Gamma is a number. This gives the
/// same `CalGray` dictionary fields when indirect, and its literal gamma-2 neutral is encoded
/// from linear light 0.25: 1.055 × 0.25^(1/2.4) − 0.055 ≈ 0.53709873 per IEC sRGB.
#[test]
fn calgray_whitepoint_array_members_and_gamma_resolve_indirectly() {
    let direct = pdf_with_stream(
        5,
        "",
        &[],
        &[(
            6,
            b"[/CalGray << /WhitePoint [0.9642 1 0.8249] /Gamma 2 >>]".to_vec(),
        )],
    );
    let indirect = pdf_with_stream(
        5,
        "",
        &[],
        &[
            (6, b"[/CalGray 7 0 R]".to_vec()),
            (7, b"<< /WhitePoint 8 0 R /Gamma 12 0 R >>".to_vec()),
            (8, b"[9 0 R 10 0 R 11 0 R]".to_vec()),
            (9, b"0.9642".to_vec()),
            (10, b"1".to_vec()),
            (11, b"0.8249".to_vec()),
            (12, b"2".to_vec()),
        ],
    );
    let parse = |document: &Document| {
        let object = document.get(ObjectId {
            number: 6,
            generation: 0,
        });
        ColourSpace::parse(document, &object, &Dictionary::new()).expect("CalGray space")
    };
    let direct_space = parse(&direct);
    let indirect_space = parse(&indirect);
    assert!(matches!(
        &direct_space,
        ColourSpace::CalGray { white, gamma: 2.0, .. }
            if *white == [0.9642, 1.0, 0.8249]
    ));
    assert!(matches!(
        &indirect_space,
        ColourSpace::CalGray { white, gamma: 2.0, .. }
            if *white == [0.9642, 1.0, 0.8249]
    ));

    // The adaptation matrices use rounded constants. Allow less than one quarter of an
    // 8-bit channel step here; exact parsed parameters and exact direct/indirect RGB equality
    // below isolate reference handling from colour-conversion precision.
    let color = indirect_space.to_rgb(&[0.5]);
    for component in [color.r, color.g, color.b] {
        assert!((component - 0.537_098_7).abs() < 1.0 / 1024.0, "{color:?}");
    }
    assert_eq!([color.r, color.g, color.b], {
        let direct_color = direct_space.to_rgb(&[0.5]);
        [direct_color.r, direct_color.g, direct_color.b]
    });
}
