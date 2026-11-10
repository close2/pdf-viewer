//! Discriminating tests for the §7.3 examples and the numeric writer spelling.

use std::fmt::Write as _;

use pdf_syntax::object::{Name, Object, ObjectId};
use pdf_syntax::{Document, Parser, write};

#[expect(
    clippy::expect_used,
    reason = "this helper must surface an invalid specification example rather than pass silently"
)]
fn parsed(source: &[u8]) -> Object {
    Parser::new(source)
        .parse_object()
        .expect("the clause example is a PDF object")
}

#[test]
fn numeric_examples_are_read_as_the_stated_integer_and_real_values() {
    for (source, expected) in [
        (&b"123"[..], Object::Integer(123)),
        (b"43445", Object::Integer(43445)),
        (b"+17", Object::Integer(17)),
        (b"-98", Object::Integer(-98)),
        (b"0", Object::Integer(0)),
        (b"34.5", Object::Real(34.5)),
        (b"-3.62", Object::Real(-3.62)),
        (b"+123.6", Object::Real(123.6)),
        (b"4.", Object::Real(4.0)),
        (b"-.002", Object::Real(-0.002)),
    ] {
        assert_eq!(
            parsed(source),
            expected,
            "{}",
            String::from_utf8_lossy(source)
        );
    }
}

#[test]
#[expect(
    clippy::approx_constant,
    reason = "3.14 is the exact real value printed in §7.3.6 EXAMPLE"
)]
fn heterogeneous_array_example_preserves_order_and_types() {
    assert_eq!(
        parsed(b"[549 3.14 false (Ralph) /SomeName]"),
        Object::Array(vec![
            Object::Integer(549),
            Object::Real(3.14),
            Object::Boolean(false),
            Object::String(b"Ralph".to_vec().into()),
            Object::Name(Name::new(b"SomeName".to_vec())),
        ]),
    );
}

#[test]
fn nested_dictionary_example_preserves_its_entries() {
    let parsed = parsed(b"<</Type /Example /Subtype /DictionaryExample /Version 0.01 /IntegerItem 12 /StringItem (a string) /Subdictionary << /Item1 0.4 /Item2 true /LastItem (not !) /VeryLastItem (OK) >> >>");
    let Object::Dictionary(outer) = parsed else {
        panic!("the example is a dictionary");
    };
    assert_eq!(
        outer
            .get("Type")
            .and_then(Object::as_name)
            .and_then(Name::as_str),
        Some("Example")
    );
    assert_eq!(
        outer
            .get("Subtype")
            .and_then(Object::as_name)
            .and_then(Name::as_str),
        Some("DictionaryExample")
    );
    assert_eq!(outer.get("Version"), Some(&Object::Real(0.01)));
    assert_eq!(outer.get("IntegerItem"), Some(&Object::Integer(12)));
    assert_eq!(
        outer.get("StringItem"),
        Some(&Object::String(b"a string".to_vec().into()))
    );
    let Object::Dictionary(inner) = outer.get("Subdictionary").expect("the nested dictionary")
    else {
        panic!("Subdictionary is a dictionary");
    };
    assert_eq!(inner.get("Item1"), Some(&Object::Real(0.4)));
    assert_eq!(inner.get("Item2"), Some(&Object::Boolean(true)));
    assert_eq!(
        inner.get("LastItem"),
        Some(&Object::String(b"not !".to_vec().into()))
    );
    assert_eq!(
        inner.get("VeryLastItem"),
        Some(&Object::String(b"OK".to_vec().into()))
    );
}

#[test]
fn indirect_object_definition_example_is_parsed_with_its_identifier() {
    let (id, value) = Parser::new(b"12 0 obj (Brillig) endobj")
        .parse_indirect_object()
        .expect("the indirect-object example is valid");
    assert_eq!(id, ObjectId::new(12, 0));
    assert_eq!(value, Object::String(b"Brillig".to_vec().into()));
}

#[test]
fn real_writer_uses_fixed_decimal_and_round_trips_extreme_values() {
    for value in [f64::MAX, 6.02e23, 1e-21, -1e-21] {
        let mut bytes = Vec::new();
        write::object(&Object::Real(value), &mut bytes);
        assert!(
            !bytes.iter().any(|byte| matches!(byte, b'e' | b'E' | b'#')),
            "a PDF real must not use exponent or radix notation: {}",
            String::from_utf8_lossy(&bytes)
        );
        let parsed = Parser::new(&bytes)
            .parse_object()
            .expect("writer emitted a PDF number");
        assert_eq!(
            parsed,
            Object::Real(value),
            "the written decimal preserves the represented value"
        );
    }
}

#[expect(
    clippy::expect_used,
    reason = "the valid PDF fixture must fail loudly if its page tree cannot be opened"
)]
fn valid_one_page_document() -> Document {
    let mut pdf = String::from("%PDF-1.7\n");
    let mut offsets = Vec::new();
    for body in [
        "<< /Type /Catalog /Pages 2 0 R /Probe 4 1 R >>",
        "<< /Type /Pages /Kids [3 0 R] /Count 1 >>",
        "<< /Type /Page /Parent 2 0 R /MediaBox [0 0 10 10] /Resources <<>> >>",
        "42",
    ] {
        offsets.push(pdf.len());
        let number = offsets.len();
        let _ = writeln!(pdf, "{number} 0 obj\n{body}\nendobj");
    }
    let xref = pdf.len();
    pdf.push_str("xref\n0 5\n0000000000 65535 f \n");
    for offset in offsets {
        let _ = writeln!(pdf, "{offset:010} 00000 n ");
    }
    let _ = write!(
        pdf,
        "trailer\n<< /Size 5 /Root 1 0 R >>\nstartxref\n{xref}\n%%EOF\n"
    );

    let document = Document::open(pdf.into_bytes()).expect("the xref and page objects are valid");
    assert!(!document.was_recovered());
    let catalog = document
        .catalog()
        .expect("the catalog and page tree are defined");
    assert_eq!(
        catalog
            .get("Type")
            .and_then(Object::as_name)
            .and_then(Name::as_str),
        Some("Catalog")
    );
    document
}

#[test]
fn a_cold_wrong_generation_reference_is_undefined() {
    let document = valid_one_page_document();
    // This is the first lookup of object 4: the reference names (4, 1), while the file
    // defines (4, 0). Capture both results before asserting so the valid-generation control
    // still runs even when the cold wrong-generation route is defective.
    let undefined = document.get(ObjectId::new(4, 1));
    let defined = document.get(ObjectId::new(4, 0));
    assert_eq!(
        defined,
        Object::Integer(42),
        "the existing generation-0 object is the positive control"
    );
    assert_eq!(
        undefined,
        Object::Null,
        "generation 1 is undefined although generation 0 exists"
    );
}

#[test]
fn a_warm_wrong_generation_reference_is_undefined() {
    let document = valid_one_page_document();
    let defined = document.get(ObjectId::new(4, 0));
    let catalog = document.catalog().expect("the catalog is defined");
    let reference = catalog.get("Probe").expect("the fixture states 4 1 R");
    assert_eq!(reference, &Object::Reference(ObjectId::new(4, 1)));
    let undefined = document.resolve(reference);
    assert_eq!(
        defined,
        Object::Integer(42),
        "the existing generation-0 object is the positive control"
    );
    assert_eq!(
        undefined,
        Object::Null,
        "the generation-0 cache entry must not satisfy a generation-1 lookup"
    );
}
