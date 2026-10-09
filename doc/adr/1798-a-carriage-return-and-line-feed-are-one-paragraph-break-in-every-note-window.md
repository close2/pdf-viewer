# 1798 — A carriage return and line feed are one paragraph break in every note window

Session 1481. Status: **accepted** and **built**. Context: ISO 32000-2 §12.5.6.2, §3.23, §7.2.3,
§7.3.4.2; ADRs 1642, 1739, 1770. Code: `crates/viewer-host/src/popup.rs` (`paragraphs`, `thread`,
`escape_into`), `crates/viewer-ui/src/chrome.rs` (`draw_thread`'s replies, `draw_plain`,
`plain_carets`), `crates/viewer-ui/src/bin/quorra/typing.rs` (`caret_boundary`, `before`, `after`).
Tests: `viewer-host` `popup::tests::a_carriage_return_and_line_feed_together_are_one_paragraph_break`;
`viewer-ui` `panel::a_carriage_return_and_line_feed_are_one_paragraph_break_in_a_note_s_window` and
`typing::tests::a_carriage_return_and_line_feed_are_one_step_for_the_caret`.

## 1. What the standard says, and what it leaves

§12.5.6.2 names the separator of a markup annotation's text, in a sentence addressed to the writer:
"When separating text into paragraphs, a CARRIAGE RETURN (0Dh) shall be used and not, for example, a
LINE FEED character (0Ah)." The ledger's row already reads it for a reader: a carriage return ends a
paragraph, and a line feed is accepted as one too, because enforcing the writer's rule would show a
producer's break as no break.

What it does not say is what a carriage return followed by a line feed is. Split on each byte, the
pair was two breaks with an empty paragraph between them, in `quorra`'s window, its replies, its
caret places, and the thread string `viewer_host::popup::thread` hands Qt. **The pair is one break,
and that is a choice**, on two pieces of evidence about the file's syntax rather than about a text
string's display: §3.23 defines an end-of-line marker as a "one or two character sequence", and
§7.2.3 says the pair "shall be treated as one EOL marker". §7.3.4.2 says the same of a literal
string's unescaped line end, which reaches `/Contents` as one line feed. So a pair that survives into
a text string was written escaped or in hexadecimal, and the producer meant one line end.

A line feed followed by a carriage return is two breaks. Only §3.23's order is one marker.

## 2. One split, in one place

`viewer_host::popup::paragraphs` is the split, and it yields each paragraph's starting byte, so that
`plain_carets` no longer assumes a separator one byte long. `quorra` draws through it. `thread` and
Qt's `escape_into` use it as well. `escape_into` used to drop a carriage return and turn a line feed
into a `br`. That ran a conforming note's paragraphs together in Qt's rich thread, because the
clause's own separator is the carriage return. GTK hands the raw text to Pango, which already treats
CR, LF and CR LF as paragraph separators, and Qt's editor already counts the pair as one block
separator (`editor_position`). So the three windows now agree.

The variable-text layout of a field (`pdf_model::variable_text`) already reads the pair as one break,
so `quorra`'s caret now steps over it as one character too: Left, Right, Backspace and Delete cross
the pair whole, and an offset between its halves stands before it. Half a break would leave the
window drawing the same text, and the person would see nothing happen.

## 3. What this does not decide

The writer's half of §12.5.6.2 is this program's whenever it writes a note's `/Contents`. That
happens with `Edit::SetNoteText`, written by §7.5.6's incremental update (ADR 1721). `quorra`'s
Enter inserts a line feed there, and the toolkits' editors hand back line feeds. Writing a carriage
return at the edit is a separate change to all three windows and to the editors' offset arithmetic,
and is left to a later round.
