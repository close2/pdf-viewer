//! Fuzzes ISO 32000-2 Annex O's fragment identifier reader.
//!
//! A fragment arrives with the *request* rather than with the file, which makes it the first
//! untrusted input in this tree that no document can carry — and exactly as untrusted as one, because
//! a URI is written by whoever sent the link. `CLAUDE.md` asks for fuzzing from the first parser
//! commit, and this parser was the first one committed since that list was last added to.
//!
//! Three properties are under test.
//!
//! **Reading terminates and never panics**, over any string — including the
//! `arithmetic_side_effects` this workspace lints for, because the fuzz profile keeps overflow
//! checks on and the reader does hexadecimal arithmetic on percent-escapes.
//!
//! **Nothing is dropped in silence.** §O.2's parameters are separated by one character, so the
//! number of parameters read plus the number named as unread can never exceed the number of
//! separators plus one. A reader that lost one would fail this.
//!
//! **Every argument came out of the fragment.** Percent-decoding only ever shortens — `%41` is
//! three bytes in and one out — and nothing else lengthens anything, so no byte string a
//! parameter carries can be longer than the text it was read from.

#![no_main]

use libfuzzer_sys::fuzz_target;
use pdf_model::fragment::{Fragment, Parameter};

fuzz_target!(|data: &[u8]| {
    // A fragment is the text after `#` in a URI, so a host holding bytes has already decided what
    // they mean. Anything that is not a string was never a fragment.
    let Ok(text) = std::str::from_utf8(data) else {
        return;
    };
    let read = Fragment::parse(text);

    let separators = text.matches('&').count();
    assert!(
        read.parameters.len() + read.unread.len() <= separators + 1,
        "{} parameters and {} unread out of {} separators",
        read.parameters.len(),
        read.unread.len(),
        separators
    );

    for parameter in &read.parameters {
        let length = match parameter {
            Parameter::NamedDestination(bytes)
            | Parameter::StructureElement(bytes)
            | Parameter::Comment(bytes)
            | Parameter::EmbeddedFile(bytes)
            | Parameter::Fdf(bytes) => bytes.len(),
            Parameter::Search(words) => words.iter().map(Vec::len).sum(),
            _ => 0,
        };
        assert!(
            length <= text.len(),
            "{length} bytes of argument out of {} of fragment",
            text.len()
        );
        // Every parameter names itself, and the four this program cannot carry out say why. A
        // host reports from these two and from nothing of its own.
        assert!(!parameter.name().is_empty());
        assert!(parameter.unhonoured().is_none_or(|why| !why.is_empty()));
    }

    for unread in &read.unread {
        assert!(
            unread.name.len() <= text.len(),
            "a parameter name longer than the fragment that stated it"
        );
    }

    // Reading is a function of the text: the same string twice is the same answer, which is what
    // makes a fragment safe to re-apply when §7.6.4.1's prompt reopens the document.
    assert!(
        Fragment::parse(text) == read,
        "the same fragment read twice gave two different answers"
    );
});
