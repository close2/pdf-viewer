//! What the operator loop says about what it could not run: the report's entry points and the
//! translation of a nested stream's damage into the words a form, a pattern or a glyph can use.

use crate::content::report::{DamagedStream, KeywordNote, Unsupported};

use crate::content::Interpreter;

impl Interpreter<'_> {
    /// Records one thing this page could not be drawn as.
    ///
    /// The counter beside the map is what attributes a refusal to the marked-content sequence
    /// that enclosed it — see [`Interpreter::notes_raised`] for why it is not the map's length.
    pub(in crate::content) fn note(&mut self, item: Unsupported) {
        self.notes_raised = self.notes_raised.saturating_add(1);
        self.unsupported.insert(item);
    }

    /// [`Interpreter::note`] for one of the three sentences a content stream's keyword produces,
    /// which builds the sentence only the first time the page meets that keyword under it.
    ///
    /// A repeat is counted all the same, since a marked-content sequence asks whether anything
    /// was raised inside it and a repeat was. [`crate::content::report::Notes`] says what this saves and
    /// why.
    pub(super) fn note_keyword(
        &mut self,
        sentence: KeywordNote,
        keyword: &[u8],
        item: impl FnOnce(&[u8]) -> Unsupported,
    ) {
        self.notes_raised = self.notes_raised.saturating_add(1);
        if self.unsupported.keyword_is_new(sentence, keyword) {
            self.unsupported.insert(item(keyword));
        }
    }

    /// The report of a keyword dispatched where an operator belongs that this interpreter does
    /// not run: one Table 50 does not list, or one of Table 90's that belongs inside an inline
    /// image.
    pub(super) fn note_unknown_operator(&mut self, keyword: &[u8]) {
        self.note_keyword(KeywordNote::Unknown, keyword, |keyword| {
            Unsupported::Operator {
                operator: String::from_utf8_lossy(keyword).into_owned(),
            }
        });
    }

    /// Says what reading one of §7.8.2's other four content streams through a window found.
    ///
    /// [`ContentIssue`] is Table 31's noun and every one of its indexed variants is about a
    /// part of a *page's* `/Contents`, so what the window raises here is translated rather
    /// than passed on: a form reached through `/XObject` has no part index, and putting a
    /// zero there would say something about `/Contents` that is not true. ADR 0359 made the
    /// same distinction for the damage report and this is the rest of it.
    ///
    /// [`ContentIssue`]: crate::page::ContentIssue
    pub(super) fn note_nested(&mut self, issue: crate::page::ContentIssue, what: &str) {
        match issue {
            // ADR 0343's sentence, in the same words the whole-decode route uses — see
            // [`Interpreter::content_stream`].
            crate::page::ContentIssue::Damaged { damage, kept, .. } => {
                self.note(Unsupported::DamagedContentStream {
                    stream: DamagedStream {
                        detail: what.to_owned(),
                        damage,
                        kept,
                    },
                });
            }
            // Neither of these is indexed by a part, so both carry across as themselves: a
            // token no buffer of `CEILING` bytes can hold, and a stream that reached
            // `max_stream_len`. The second is a bound of ours and the first is this reader's
            // buffer, and both are refusals rather than damage — ADR 0365's distinction.
            issue @ crate::page::ContentIssue::TokenTooLong { .. } => {
                self.note(Unsupported::Content { issue });
            }
            crate::page::ContentIssue::TooLarge { limit, .. } => {
                self.note(Unsupported::Content {
                    issue: crate::page::ContentIssue::TooLarge { part: None, limit },
                });
            }
            // A stream the window could not decode at all reaches the caller's own sentence
            // instead, because `nested_content_source` only windows a stream whose decode has
            // already produced bytes. It is written rather than left out because a silence
            // here would be exactly the failure trap 5 is about.
            crate::page::ContentIssue::Undecodable { .. }
            | crate::page::ContentIssue::NotAStream { .. }
            | crate::page::ContentIssue::Unreachable { .. } => {
                self.note(Unsupported::Operator {
                    operator: format!("undecodable {what}"),
                });
            }
        }
    }
}
