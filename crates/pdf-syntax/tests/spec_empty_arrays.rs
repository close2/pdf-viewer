//! ISO 32000-2 §7.3.6 permits arrays with zero elements, including nested arrays.

use pdf_syntax::{Object, Parser};

#[test]
fn empty_and_nested_empty_arrays_are_valid() {
    let empty = Parser::new(b"[]")
        .parse_object()
        .expect("an array may have zero elements");
    assert_eq!(empty, Object::Array(Vec::new()));

    let nested = Parser::new(b"[[]]")
        .parse_object()
        .expect("arrays may contain arrays to any depth");
    assert_eq!(nested, Object::Array(vec![Object::Array(Vec::new())]));
}
