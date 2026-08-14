//! §7.10.2's sample array, and the bytes a stream has to hold for it.
//!
//! > The stream data shall be long enough to contain the entire sample array, as indicated by
//! > Size , Range , and BitsPerSample ; see 7.3.8.2, "Stream extent".
//!
//! A Type 0 function's stream is one of the objects §7.3.8.2 means by "many objects from whose
//! attributes a length can be inferred", and the arithmetic is stated rather than implied. Until
//! the five-hundred-and-twenty-first session the sample reader answered **0** for every sample
//! past the end of the data, so a stream holding half its samples produced a function whose
//! second half was a value nobody wrote — decoded through `/Decode` and interpolated into the
//! samples beside it. A tint transform or a shading built on one of those does not draw part of
//! what the producer asked for; it draws something else, which is the substitutive half of trap
//! 5's test.
//!
//! **The corpus cannot reach this rule**, which is why it is pinned by hand: `doc/todo/03` §9's
//! census over 65 944 crawled documents, the 974 of the pdf.js corpus and the 167 of
//! `format-corpus` finds **not one** sampled function whose stream falls short. So the fixtures
//! come in pairs differing in exactly one thing — the number of bytes after `stream` — which is
//! the shape `cross_references.rs` uses for the same reason (trap 8).

#![expect(
    clippy::expect_used,
    reason = "test code: a malformed fixture should fail loudly"
)]

use pdf_model::function::Function;
use pdf_syntax::{Document, Object, ObjectId};

/// The four samples the exact fixture carries: the ends of `/Range` and two steps between.
const SAMPLES: [u8; 4] = [0x00, 0x55, 0xAA, 0xFF];

/// A one-page document whose object 6 is a Type 0 function carrying `samples`.
///
/// Everything else is held constant: `/Size [4]`, `/BitsPerSample 8` and a one-component
/// `/Range`, so §7.10.2's arithmetic asks for exactly four bytes and the fixtures differ only in
/// how many arrive.
fn document_with(samples: &[u8]) -> Document {
    let mut objects: Vec<Vec<u8>> = vec![
        b"<< /Type /Catalog /Pages 2 0 R >>".to_vec(),
        b"<< /Type /Pages /Kids [3 0 R] /Count 1 >>".to_vec(),
        b"<< /Type /Page /Parent 2 0 R /MediaBox [0 0 100 50] \
          /Resources << /Shading << /Sh0 5 0 R >> >> /Contents 4 0 R >>"
            .to_vec(),
        b"<< /Length 8 >>\nstream\n/Sh0 sh\nendstream".to_vec(),
        b"<< /ShadingType 2 /ColorSpace /DeviceGray /Coords [0 0 100 0] /Function 6 0 R >>"
            .to_vec(),
    ];

    let mut function = format!(
        "<< /FunctionType 0 /Domain [0 1] /Range [0 1] /Size [4] /BitsPerSample 8 /Length {} >>\n\
         stream\n",
        samples.len()
    )
    .into_bytes();
    function.extend_from_slice(samples);
    function.extend_from_slice(b"\nendstream");
    objects.push(function);

    let mut out = b"%PDF-1.7\n".to_vec();
    let mut offsets = Vec::new();
    for (index, object) in objects.iter().enumerate() {
        offsets.push(out.len());
        out.extend_from_slice(format!("{} 0 obj\n", index.saturating_add(1)).as_bytes());
        out.extend_from_slice(object);
        out.extend_from_slice(b"\nendobj\n");
    }
    let xref_at = out.len();
    let size = offsets.len().saturating_add(1);
    out.extend_from_slice(format!("xref\n0 {size}\n").as_bytes());
    out.extend_from_slice(b"0000000000 65535 f \n");
    for offset in &offsets {
        out.extend_from_slice(format!("{offset:010} 00000 n \n").as_bytes());
    }
    out.extend_from_slice(
        format!("trailer\n<< /Size {size} /Root 1 0 R >>\nstartxref\n{xref_at}\n%%EOF\n")
            .as_bytes(),
    );
    Document::open(out).expect("the fixture opens")
}

/// Object 6, the function under test.
fn function() -> Object {
    Object::Reference(ObjectId {
        number: 6,
        generation: 0,
    })
}

/// The whole array present is the function the file describes, read sample for sample.
#[test]
fn a_sampled_function_whose_stream_holds_its_array_is_built() {
    let document = document_with(&SAMPLES);
    let built = Function::parse(&document, &function()).expect("four samples of four are enough");

    // `/Size [4]` with the default `/Encode [0 3]` puts the samples at x = 0, ⅓, ⅔ and 1, and
    // `/Decode` defaults to `/Range`, so each byte is its own fraction of 255.
    for (index, sample) in SAMPLES.iter().enumerate() {
        #[expect(
            clippy::cast_precision_loss,
            reason = "four grid positions, each exactly representable"
        )]
        let at = index as f32 / 3.0;
        let expected = f32::from(*sample) / 255.0;
        let value = built.eval(&[at]);
        assert!(
            value
                .first()
                .is_some_and(|out| (out - expected).abs() < 1e-3),
            "sample {index} should read back as {expected} and gave {value:?}"
        );
    }
}

/// One byte short is a function the file does not carry, and it is refused rather than filled in.
///
/// The pair's only difference is the byte count, so the refusal cannot be about anything else.
#[test]
fn a_sampled_function_whose_stream_is_short_is_refused_rather_than_filled_with_zeroes() {
    let document = document_with(&SAMPLES[..3]);
    let error =
        Function::parse(&document, &function()).expect_err("three bytes cannot hold four samples");
    let said = format!("{error}");
    assert!(
        said.contains("needs 4 bytes") && said.contains("holds 3"),
        "the refusal should name §7.10.2's arithmetic and both numbers, and said {said}"
    );
}

/// And the page that uses it says so, rather than drawing a gradient of invented samples.
#[test]
fn a_page_whose_shading_needs_that_function_reports_it() {
    let document = document_with(&SAMPLES[..3]);
    let pages = pdf_model::Pages::new(&document);
    let page = pages.get(0).expect("the fixture has a page");
    let interpretation = pdf_model::interpret(&document, &page);
    assert!(
        !interpretation.is_complete(),
        "a shading whose function could not be built is a mark the page does not make"
    );
}
