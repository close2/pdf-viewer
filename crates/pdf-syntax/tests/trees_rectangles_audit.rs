//! ISO 32000-2 §7.9.6: name tree key semantics.

use pdf_syntax::{
    object::{Dictionary, Name, Object},
    tree::{TreeKey, lookup},
};

fn node(entries: &[(&str, Object)]) -> Dictionary {
    let mut dict = Dictionary::new();
    for (key, value) in entries {
        dict.insert(Name::new(key.as_bytes().to_vec()), value.clone());
    }
    dict
}

fn direct(object: &Object) -> Object {
    object.clone()
}

/// §7.9.6 compares keys as bytes, allowing encodings while requiring bytewise identity.
/// The keys include a prefix pair and two different encodings of the same displayed text;
/// case folding, Unicode normalization, or dropping a byte would return the wrong value.
#[test]
fn name_tree_lookup_uses_exact_bytes_and_distinguishes_prefixes() {
    let root = node(&[(
        "Names",
        Object::Array(vec![
            Object::String(b"A".to_vec().into()),
            Object::Integer(3),
            Object::String(b"a".to_vec().into()),
            Object::Integer(1),
            Object::String(b"a\0".to_vec().into()),
            Object::Integer(2),
            // UTF-8 "é" and the byte sequence for "e" plus combining acute.
            Object::String(vec![0x65, 0xcc, 0x81].into()),
            Object::Integer(5),
            Object::String(vec![0xc3, 0xa9].into()),
            Object::Integer(4),
        ]),
    )]);

    for (key, value) in [
        (&b"a"[..], 1),
        (&b"a\0"[..], 2),
        (&b"A"[..], 3),
        (&[0xc3, 0xa9][..], 4),
        (&[0x65, 0xcc, 0x81][..], 5),
    ] {
        assert_eq!(
            lookup(&root, &TreeKey::Name(key), &direct),
            Some(Object::Integer(value)),
            "key bytes {key:?} must select their own value"
        );
    }
    assert_eq!(
        lookup(&root, &TreeKey::Name(&[0xc3, 0x89]), &direct),
        None,
        "a distinct byte sequence must not match by case or text equivalence"
    );
}
