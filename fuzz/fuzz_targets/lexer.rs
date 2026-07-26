//! Fuzzes the lexer.
//!
//! The property under test is that tokenising *any* byte sequence terminates, makes
//! progress, and never panics. The lexer is the first thing untrusted bytes reach, so a
//! hang here is a denial of service on opening a document.

#![no_main]

use libfuzzer_sys::fuzz_target;
use pdf_syntax::Lexer;

fuzz_target!(|data: &[u8]| {
    let mut lexer = Lexer::new(data);
    let mut last = lexer.position();
    let mut tokens = 0usize;

    while lexer.next_token().is_some() {
        let now = lexer.position();
        // Every token must consume at least one byte. Without this the loop below would
        // spin forever on input the lexer cannot classify, and the bound would hide it.
        assert!(now > last, "the lexer failed to advance at byte {last}");
        last = now;

        tokens += 1;
        // One token per byte is the theoretical maximum.
        assert!(tokens <= data.len(), "more tokens than bytes");
    }
});
