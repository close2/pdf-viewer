//! Fuzzes a composite font's `CMap` parser and its decoding algorithm.
//!
//! ISO 32000-2 §9.7.5.3's `CMap` file is a stream in the document, so its bytes are as
//! untrusted as any others, and §9.7.6.2's decoder runs over the bytes of every string a
//! content stream shows. Two properties are under test.
//!
//! **Parsing terminates and never panics**, over any byte sequence, including one whose
//! `begincidrange` sections state four-billion-code ranges — which is why the parser keeps
//! ranges as ranges rather than expanding them.
//!
//! **Decoding always makes progress**, which is the one that could hang a page rather than
//! merely draw it wrong. §9.7.6.2 extracts "a sequence of one or more bytes" per code and
//! §9.7.6.3 decides the length when none of the codespace ranges match; a `CMap` that led
//! either to consume zero bytes would loop forever on the first string shown.

#![no_main]

use libfuzzer_sys::fuzz_target;
use pdf_font::cmap::CMap;

fuzz_target!(|data: &[u8]| {
    let map = CMap::parse(data, None);

    // Decode the same bytes as a shown string: any input is a plausible one, and it exercises
    // whatever codespace ranges the first half of the input happened to define.
    let mut rest = data;
    let mut codes = 0usize;
    while !rest.is_empty() {
        let code = map.next_code(rest);
        let taken = usize::from(code.length());
        // The clause's own words are "one or more bytes", and a zero here is a hang.
        assert!(
            taken >= 1,
            "a code consumed no bytes at {} left",
            rest.len()
        );
        assert!(
            taken <= 4,
            "§9.7.6.2 bounds a code at four bytes, got {taken}"
        );

        // The lookups must be total over anything `next_code` can produce.
        let _ = map.cid(code);
        let _ = map.notdef_cid(code);

        rest = rest.get(taken.min(rest.len())..).unwrap_or_default();
        codes += 1;
        assert!(codes <= data.len(), "more codes than bytes");
    }

    // A `CMap` built on another must not change either property.
    let over = CMap::parse(data, Some(&map));
    let _ = over.wmode();
    if !data.is_empty() {
        assert!(over.next_code(data).length() >= 1);
    }
});
