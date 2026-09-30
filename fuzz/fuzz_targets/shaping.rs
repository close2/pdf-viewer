//! Fuzzes `pdf_font::shaping`: UAX #9's bidirectional order and the Unicode Standard's cursive
//! joining over a text this program lays out itself.
//!
//! ISO 32000-2 §12.7.4.3 has the processor "construct an appearance stream dynamically at
//! rendering time" from a field's value, and §7.9.2.2.1 makes that value Unicode text: the
//! document states the string, so every character of it is as untrusted as any other byte of the
//! file. The same functions order an outline title and a tab's file name (ADRs 1413, 1414, 1417).
//!
//! The first byte states the paragraph direction (none, left to right, right to left) and the rest
//! is the text, read lossily so that every input is one. Beyond never panicking — overflow checks
//! stay on in this profile — five properties are under test:
//!
//! - **Joining loses and invents no character.** Every stored character is shaped exactly once,
//!   as an item's `source` or as the alef a lam-alef ligature took in (`joined_with`), and a
//!   ligature's second letter follows its first.
//! - **Rule L2's order is a permutation**, over any levels at all — including the levels above
//!   UAX #9's maximum depth that `visual_order` is documented to read as right to left.
//! - **Rule L1 keeps a level per byte**: `line_levels` over a range answers one level per byte of
//!   it, for every range on character boundaries.
//! - **A label's glyphs cover its text**: each glyph's `stored` bytes are a non-empty range on
//!   character boundaries inside the text, and there are as many glyphs as joining produced.
//! - **A caret lands on a glyph edge**: `Label::boundary` answers at most the glyph count, asked
//!   at up to 257 bytes spread across the text and one past it.

#![no_main]
#![expect(
    clippy::expect_used,
    reason = "a fuzz target states its properties by failing: `expect` and `panic!` are how a violated one reaches libFuzzer, and each message here names the property rather than the call"
)]

use libfuzzer_sys::fuzz_target;
use pdf_font::shaping::{Label, Paragraphs, displayed_characters, positions, shape, visual_order};

fuzz_target!(|data: &[u8]| {
    let Some((&head, rest)) = data.split_first() else {
        return;
    };
    let direction = match head % 3 {
        0 => None,
        1 => Some(false),
        _ => Some(true),
    };
    let text = String::from_utf8_lossy(rest);
    let text = text.as_ref();
    let letters: Vec<char> = text.chars().collect();

    // Joining.
    assert_eq!(
        positions(&letters).len(),
        letters.len(),
        "one joining position per character"
    );
    let shaped = shape(&letters);
    let mut seen = vec![0u8; letters.len()];
    for item in &shaped {
        let slot = seen
            .get_mut(item.source)
            .expect("an item's source is a character of the text");
        *slot = slot.saturating_add(1);
        if let Some(alef) = item.joined_with {
            assert!(alef > item.source, "a ligature's alef follows its lam");
            let slot = seen
                .get_mut(alef)
                .expect("a ligature's second letter is a character of the text");
            *slot = slot.saturating_add(1);
        }
    }
    assert!(
        seen.iter().all(|count| *count == 1),
        "joining shaped every character exactly once"
    );

    // Rule L2 over arbitrary levels: the raw bytes, one level each.
    let order = visual_order(rest);
    let mut placed = vec![false; rest.len()];
    for index in &order {
        let slot = placed
            .get_mut(*index)
            .expect("rule L2 orders the items it was given");
        assert!(!*slot, "rule L2 places each item once");
        *slot = true;
    }
    assert_eq!(order.len(), rest.len(), "rule L2 places every item");

    // Resolution and rule L1.
    let paragraphs = Paragraphs::with_direction(text, direction);
    if let Some(paragraphs) = &paragraphs {
        let boundaries: Vec<usize> = text
            .char_indices()
            .map(|(at, _)| at)
            .chain(std::iter::once(text.len()))
            .collect();
        // A handful of ranges rather than every pair, which would be quadratic in the input.
        for (&start, &end) in boundaries.iter().zip(boundaries.iter().rev()).take(8) {
            let (start, end) = (start.min(end), start.max(end));
            assert_eq!(
                paragraphs.line_levels(start..end).len(),
                end.saturating_sub(start),
                "rule L1 answers one level per byte of the line"
            );
        }
        for at in 0..=text.len() {
            let _ = paragraphs.level(at);
            let _ = paragraphs.paragraph_level(at);
            let _ = paragraphs.removed_by_x9(at);
        }
    }
    let _ = displayed_characters(text, paragraphs.as_ref());

    // A label.
    let label = Label::new(text);
    assert_eq!(
        label.glyphs().len(),
        shaped.len(),
        "a label draws one glyph per shaped item"
    );
    for glyph in label.glyphs() {
        assert!(
            glyph.stored.start < glyph.stored.end && glyph.stored.end <= text.len(),
            "a glyph's stored bytes are a non-empty range of the text: {:?} of {}",
            glyph.stored,
            text.len()
        );
        assert!(
            text.is_char_boundary(glyph.stored.start) && text.is_char_boundary(glyph.stored.end),
            "a glyph's stored bytes start and end on character boundaries"
        );
    }
    // `boundary` is linear in the label, so the carets asked are capped rather than every byte.
    let step = (text.len() / 256).max(1);
    for at in (0..=text.len()).step_by(step) {
        assert!(
            label.boundary(at) <= label.glyphs().len(),
            "a caret stands at a glyph edge"
        );
    }
});
