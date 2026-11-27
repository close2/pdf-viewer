//! Marked-content sequences the removal reached: their alternate text, and the identifiers the
//! structure tree reaches them by (ISO 32000-2 §14.6, §14.7.5.2, §14.9.3–§14.9.5, ADR 3725).
//!
//! §14.9.4 lets a sequence's own property list state `/ActualText`, the value that "shall be
//! used as a replacement, not a description, for the content", and §14.9.3 and §14.9.5 let it
//! state `/Alt` and `/E` beside it. All three are written by the producer to stand for what the
//! sequence shows. Where the removal took any of what a sequence shows, those entries stand for
//! content the file no longer holds, and carrying them would carry the removed words in the
//! content stream itself — the "traces of the specified content" §12.5.6.23 requires removed. So
//! an inline property list loses the three, and the sequence's `/MCID` is recorded so that the
//! structure element it belongs to loses its own (`super::traces`).
//!
//! A property list named from `/Properties` is a resource other content may use; restating it
//! for this one sequence is a construction this verb does not make, so a sequence the removal
//! reached whose named list states any of the three refuses the page by name, as every other
//! removal that cannot be proven is refused.

use pdf_syntax::lexer::{Lexer, Token};
use pdf_syntax::object::{Object, ObjectId};

use super::{Operand, Walk, collect_array, skip_dictionary};

/// The property-list entries that describe a sequence's content (§14.9.3, §14.9.4, §14.9.5).
pub(super) const DESCRIBING: [&[u8]; 3] = [b"ActualText", b"Alt", b"E"];

/// How much the walk had removed, counted every way it removes.
#[derive(Clone, Copy, PartialEq, Eq)]
pub(super) struct Tally {
    codes: usize,
    removed: usize,
    others: usize,
}

/// One open `BDC` or `BMC`: what it states and what the walk had removed when it opened.
pub(super) struct Open {
    /// The top-level entries of an inline property list that describe the content, as byte
    /// ranges of the stream to delete.
    describing: Vec<(usize, usize)>,
    /// The `/MCID` its property list states.
    mcid: Option<i64>,
    /// Whether a named property list states a describing entry.
    named_describes: bool,
    before: Tally,
}

/// One marked-content sequence with an identifier that the removal reached (§14.7.5.2).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) struct Reached {
    /// The form `XObject` whose stream holds it, or `None` for the page's own content.
    pub(super) stream: Option<ObjectId>,
    /// Its `/MCID`.
    pub(super) mcid: i64,
    /// Whether every glyph it showed was removed, rather than part of what it showed.
    pub(super) whole: bool,
    /// Where `stream` is a form this placement drew from a copy — its own, for a form drawn more
    /// than once (ADR 1363), or the page's, for a form the page does not own (ADR 1196) — the
    /// page's form edit that copy is: the sequence is in the copy's stream, not the form's, and
    /// the structure tree reaches it by the copy's number (ADRs 3811, 3849).
    pub(super) copy: Option<usize>,
}

/// The top-level entries of the inline dictionary that starts at `start`: each key, its byte
/// range, and its value where that is an integer.
fn entries(content: &[u8], start: usize) -> Vec<(Vec<u8>, usize, usize, Option<i64>)> {
    let mut lexer = Lexer::at(content, start);
    if lexer.next_token() != Some(Token::DictOpen) {
        return Vec::new();
    }
    let mut out = Vec::new();
    loop {
        lexer.skip_whitespace();
        let key_start = lexer.position();
        let Some(Token::Name(key)) = lexer.next_token() else {
            break;
        };
        lexer.skip_whitespace();
        let integer = match lexer.next_token() {
            Some(Token::Integer(value)) => Some(value),
            Some(Token::ArrayOpen) => {
                collect_array(&mut lexer);
                None
            }
            Some(Token::DictOpen) => {
                skip_dictionary(&mut lexer);
                None
            }
            Some(_) => None,
            None => break,
        };
        out.push((key, key_start, lexer.position(), integer));
    }
    out
}

impl Walk<'_> {
    fn tally(&self) -> Tally {
        Tally {
            codes: self.code_index,
            removed: self.removed,
            others: self
                .paths_cut
                .saturating_add(self.clears.len())
                .saturating_add(self.inline_cleared)
                .saturating_add(self.form_edits.len())
                .saturating_add(self.located.len()),
        }
    }

    /// `BDC` or `BMC` opens a sequence; `operands` are the operator's.
    pub(super) fn open_sequence(&mut self, content: &[u8], operands: &[(Operand, usize)]) {
        let mut open = Open {
            describing: Vec::new(),
            mcid: None,
            named_describes: false,
            before: self.tally(),
        };
        match operands.get(1) {
            Some((Operand::Other, start)) => {
                for (key, from, to, integer) in entries(content, *start) {
                    if key == b"MCID" {
                        open.mcid = integer;
                    }
                    if DESCRIBING.contains(&key.as_slice()) {
                        open.describing.push((from, to));
                    }
                }
            }
            Some((Operand::Name(name), _)) => {
                let properties = self.document.get_key(&self.resources, "Properties");
                let list = properties
                    .as_dict()
                    .map(|table| self.document.get_key(table, &String::from_utf8_lossy(name)));
                if let Some(list) = list.as_ref().and_then(Object::as_dict) {
                    open.mcid = self.document.get_key(list, "MCID").as_integer();
                    open.named_describes = list.iter().any(|(key, value)| {
                        DESCRIBING.contains(&key.as_bytes()) && *value != Object::Null
                    });
                }
            }
            _ => {}
        }
        self.sequences.push(open);
    }

    /// `EMC` closes the innermost sequence this stream opened; one closing nothing this stream
    /// opened is left to the interpreter, which reads it.
    pub(super) fn close_sequence(&mut self) -> Result<(), String> {
        if self.sequences.len() <= self.sequence_floor {
            return Ok(());
        }
        let Some(open) = self.sequences.pop() else {
            return Ok(());
        };
        let now = self.tally();
        if now.removed == open.before.removed && now.others == open.before.others {
            return Ok(());
        }
        if open.named_describes {
            return Err(
                "§14.9: a marked-content sequence the removal reached states its alternate or \
                 actual text in a named property list, which this verb does not restate for one \
                 sequence; the page is refused rather than written with that text"
                    .to_owned(),
            );
        }
        if !open.describing.is_empty() {
            self.described = self.described.saturating_add(1);
        }
        for (from, to) in open.describing {
            self.edits.push((from, to, Vec::new()));
        }
        if let Some(mcid) = open.mcid {
            let codes = now.codes.saturating_sub(open.before.codes);
            let removed = now.removed.saturating_sub(open.before.removed);
            self.reached.push(Reached {
                stream: self.forms_open.last().copied(),
                mcid,
                whole: codes > 0 && removed == codes,
                copy: None,
            });
        }
        Ok(())
    }

    /// Closes every sequence the stream left open at its end, as §14.6's nesting would have.
    pub(super) fn close_every_sequence(&mut self) -> Result<(), String> {
        while self.sequences.len() > self.sequence_floor {
            self.close_sequence()?;
        }
        Ok(())
    }
}
