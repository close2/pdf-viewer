//! Complete valid objects distinguish configured resource limits from truncation errors.
//! These limits are reader policy, not fixed numeric requirements of ISO 32000-2.

use pdf_syntax::object::Object;
use pdf_syntax::parser::Limits;
use pdf_syntax::{Parser, SyntaxError};

fn limits() -> Limits {
    Limits {
        max_depth: 4,
        max_array_len: 2,
        max_dict_len: 2,
        max_string_len: 2,
        ..Limits::DEFAULT
    }
}

fn rejects_for(source: &[u8], expected_limit: &str) {
    let result = Parser::with_limits(source, limits()).parse_object();
    assert!(
        matches!(&result, Err(SyntaxError::LimitExceeded { limit, .. }) if *limit == expected_limit),
        "complete object must hit {expected_limit}, not an unrelated syntax error: {result:?}"
    );
}

#[test]
fn complete_array_at_capacity_succeeds_and_one_more_item_names_the_limit() {
    let value = Parser::with_limits(b"[1 true]", limits()).parse_object();
    assert_eq!(
        value.expect("two-item array is within the configured limit"),
        Object::Array(vec![Object::Integer(1), Object::Boolean(true)])
    );
    rejects_for(b"[1 true null]", "max_array_len");
}

#[test]
fn complete_dictionary_at_capacity_preserves_both_entries() {
    let value = Parser::with_limits(b"<< /A 1 /B 2 >>", limits())
        .parse_object()
        .expect("two-entry dictionary is within the configured limit");
    let Object::Dictionary(dictionary) = value else {
        panic!("dictionary fixture must yield a dictionary");
    };
    assert_eq!(dictionary.len(), 2);
    assert_eq!(dictionary.get("A"), Some(&Object::Integer(1)));
    assert_eq!(dictionary.get("B"), Some(&Object::Integer(2)));
    rejects_for(b"<< /A 1 /B 2 /C 3 >>", "max_dict_len");
}

#[test]
fn closed_nested_arrays_distinguish_depth_limit_from_unexpected_end() {
    let value = Parser::with_limits(b"[[[[0]]]]", limits())
        .parse_object()
        .expect("four closed nested arrays are within the depth limit");
    let mut expected = Object::Integer(0);
    for _ in 0..4 {
        expected = Object::Array(vec![expected]);
    }
    assert_eq!(value, expected);
    rejects_for(b"[[[[[0]]]]]", "max_depth");
}

#[test]
fn string_capacity_counts_decoded_bytes_for_literal_and_hex_spellings() {
    for source in [&br"(\101B)"[..], b"<4 1 42>"] {
        let value = Parser::with_limits(source, limits())
            .parse_object()
            .expect("two decoded bytes fit despite longer source spelling");
        assert_eq!(value, Object::String(b"AB".to_vec().into()));
    }
    rejects_for(br"(\101BC)", "max_string_len");
    rejects_for(b"<4 1 42 43>", "max_string_len");
}
