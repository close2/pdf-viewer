//! Fuzzes ISO 32000-2 §14.3.2's metadata reader — the tree's first XML over untrusted bytes.
//!
//! A metadata stream is a stream in the document, so its decoded bytes are as hostile as any
//! others, and XML is the format with the two most famous denial-of-service shapes in it. This
//! target exists because `doc/adr/0186` said it was owed with the dependency: taking a parser and
//! not fuzzing it is taking the parser's reputation instead of its behaviour.
//!
//! Three properties are under test.
//!
//! **Parsing terminates and never panics**, over any byte sequence — which includes the
//! `arithmetic_side_effects` this workspace lints for, because the fuzz profile keeps overflow
//! checks on.
//!
//! **Every budget holds.** `pdf_model::xmp` states four, and the point of a bound nothing checks
//! is nothing. Whatever comes back must be inside the two that are observable from outside: at
//! most `MAX_PROPERTIES` properties, and no value longer than the packet that stated it.
//!
//! **A successful parse is idempotent.** Reading the same bytes twice must give the same
//! properties: the reader carries a namespace stack and a frame stack across the whole packet,
//! and a state machine that leaked state between elements is exactly the defect a single pass
//! cannot see.

#![no_main]

use libfuzzer_sys::fuzz_target;
use pdf_model::xmp::{Value, Xmp};

/// `pdf_model::xmp::MAX_PROPERTIES`, which is private to that module — restated here so that the
/// target checks the bound rather than trusting it.
const MAX_PROPERTIES: usize = 4096;

fuzz_target!(|data: &[u8]| {
    let Ok(xmp) = Xmp::parse(data) else {
        // A refusal is a result: every one of them is a named `XmpError` rather than a panic,
        // which is the property the `Err` arm is here to have exercised.
        return;
    };

    assert!(
        xmp.properties().len() <= MAX_PROPERTIES,
        "{} properties, past the stated bound",
        xmp.properties().len()
    );

    for (name, value) in xmp.properties() {
        // Unescaping only ever shortens: the five predefined entities and a numeric reference
        // all cost more bytes written than the character they name, and an unknown reference is
        // copied verbatim. So no value can be longer than the packet it came out of.
        let length = match value {
            Value::Text(text) => text.len(),
            Value::Alt(items) => items.iter().map(|(_, text)| text.len()).sum(),
            Value::Seq(items) | Value::Bag(items) => items.iter().map(String::len).sum(),
            Value::Structure => 0,
        };
        assert!(
            length <= data.len(),
            "a value of {length} bytes came out of a packet of {}",
            data.len()
        );
        // A property's namespace is one an `xmlns` in scope bound, or empty. It is *not*
        // checked for being a URI: the target's first run asserted that and the first seeded
        // packet refuted it in seconds, because a mutated `xmlns:dc="…/1   \n   .1/"` is a
        // perfectly well-formed attribute whose value has white space in it. XML says nothing
        // about what a namespace name looks like, and a reader that decided otherwise would be
        // refusing files over a rule nobody wrote.
        assert!(
            name.namespace.len().saturating_add(name.local.len()) <= data.len(),
            "a name longer than the packet that stated it"
        );
    }

    // The accessors clause 12 and clause 14 reach for must be total over anything that parses.
    let _ = (
        xmp.title(),
        xmp.authors(),
        xmp.description(),
        xmp.producer(),
        xmp.keywords(),
        xmp.creator_tool(),
        xmp.created(),
        xmp.modified(),
    );

    let again = Xmp::parse(data).expect("the same bytes parsed once already");
    assert!(
        again == xmp,
        "reading the same packet twice gave two different answers"
    );
});
