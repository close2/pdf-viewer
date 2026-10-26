//! The first half of the loop: one token read into the [`Step`] the loop acts on.

use std::sync::Arc;

use pdf_syntax::Object;

use crate::content::reader::{ContentReader, Operator, Strings, Word, token_to_object};

use crate::content::MAX_OPERANDS;

/// Reads the stream's next token, keeping an operand in `pending` and handing back anything else
/// as the [`Step`] the loop acts on.
///
/// **The token is read inside the closure and nothing borrowed leaves it**, which is what lets
/// the stream be read through a moving window: see [`ContentReader::with_operand`]. An *operand*
/// is dealt with in there and never named again — the closure holds `pending` and the array
/// depth, so the object goes straight into the list the operator will read it from.
pub(super) fn next_step(
    reader: &mut ContentReader<'_>,
    strings: Strings,
    pending: &mut Vec<Object>,
    array_depth: &mut usize,
    empty_string: &Arc<[u8]>,
) -> Step {
    reader.with_operand(strings, |token| match token {
        None => Step::End,
        Some(pdf_syntax::Token::Keyword(word)) if *array_depth == 0 => {
            Operator::of(word).map_or_else(|| Step::Unknown(Word::new(word)), Step::Operator)
        }
        // §7.8.2's grammar puts an operator *after* its operands, and §7.3.6 makes an
        // array "a one-dimensional collection of objects arranged sequentially" — so a
        // keyword between two elements of an array is neither an element (it is not an
        // object) nor an operator (an array is one operand, and an operator cannot be
        // inside one). `operator-in-TJ-array.pdf` writes exactly that:
        // `[(Grandes) 0.0 Tc -250.0 (Clientèles,) 0.0 Tc … ] TJ`, and dispatching those
        // `Tc`s consumed the runs before them — the page drew one word of five.
        //
        // The recovery is to skip the keyword and keep the array, *and say so*: the
        // file is malformed and the standard states no reading for it, so drawing the
        // text without a word would be the silent-fallback failure this project
        // forbids, and refusing the text would lose what the file plainly states.
        Some(pdf_syntax::Token::Keyword(word)) => Step::InsideAnArray(Word::new(word)),
        // An inline dictionary is one operand, assembled by the caller because it
        // needs the reader again. §14.6.2: "[i]f all of the values in a property list
        // dictionary are direct objects, the dictionary may be written inline in the
        // content stream as a direct object" — the form real documents use for
        // §14.9.4's `/ActualText` (ADR 0060).
        Some(pdf_syntax::Token::DictOpen) => Step::Dictionary,
        Some(other) => {
            if matches!(other, pdf_syntax::Token::ArrayOpen) {
                *array_depth = array_depth.saturating_add(1);
            } else if matches!(other, pdf_syntax::Token::ArrayClose) {
                *array_depth = array_depth.saturating_sub(1);
            }
            // Arrays are deliberately left flattened: `TJ` and `d` read their elements
            // as separate operands and have since the beginning.
            if pending.len() < MAX_OPERANDS {
                pending.push(match other {
                    // One shared value, so an empty string costs no allocation —
                    // which is every string `with_operand` stepped over.
                    pdf_syntax::Token::String(bytes) if bytes.is_empty() => {
                        Object::String(Arc::clone(empty_string))
                    }
                    other => token_to_object(other),
                });
                Step::Operand
            } else {
                // An unclosed `[` would otherwise suppress every operator for the
                // rest of the stream, which on a fuzzed file means a blank page. One
                // operand cap's worth of tokens is as far as an array is believed.
                *array_depth = 0;
                Step::TooManyOperands
            }
        }
    })
}

/// What the operator loop must do about a token it has just read.
///
/// The [`ContentReader::with_token`] closure returns one of these, which is what carries a
/// token's meaning *out* of the borrow it was read under. An operand carries nothing: the
/// closure has already pushed it. What is left needs either the reader again, or a report, or
/// the dispatch table — and an operator comes out as the [`Operator`] it spells, a keyword
/// Table 50 does not list as a copy of its bytes (see [`Word`]), rather than borrowed, because
/// the dispatch calls the reader again for §8.9.7's inline images.
#[derive(Debug)]
pub(super) enum Step {
    /// A bare keyword where an operator belongs, one Table 50 lists: the dispatch table's business.
    Operator(Operator),
    /// A bare keyword where an operator belongs that Table 50 does not list, which the report
    /// names.
    Unknown(Word),
    /// An operand, already in the pending list.
    Operand,
    /// A keyword inside an array, which §7.3.6 admits nothing but objects into.
    InsideAnArray(Word),
    /// `<<` — an inline dictionary, which the caller assembles because it needs the reader.
    Dictionary,
    /// One more operand than `MAX_OPERANDS` allows.
    TooManyOperands,
    /// The stream has no more tokens.
    End,
}

/// A keyword in an operator's place, as the loop holds it past the token's borrow.
#[derive(Debug)]
pub(super) enum Keyword {
    /// One Table 50 lists.
    Operator(Operator),
    /// One it does not, kept as the file spelled it for the report.
    Unknown(Word),
}

impl Keyword {
    /// The bytes the file wrote.
    pub(super) fn bytes(&self) -> &[u8] {
        match self {
            Self::Operator(operator) => operator.name(),
            Self::Unknown(word) => word.as_slice(),
        }
    }
}
