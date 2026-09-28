# 1365 — Spaces before a `stream` keyword's end-of-line marker are skipped, and said

Session 1264. Status: accepted and **built**.
Context: `crates/pdf-syntax/src/parser.rs` (`Parser::parse_stream_data`, `KeywordPadding`,
`past_end_of_line`), `crates/pdf-syntax/src/document.rs` (`Document::padded_stream_keywords`,
`with_stated_length`, `stated_extent`), `crates/pdf-syntax/tests/stream_keyword_padding.rs`.
Found by: ADR 1349's fax census, whose six `4113564.pdf` streams it named for `pdf-syntax`.

## 1. What the clause says, and what it does not

ISO 32000-2 §7.3.8.1:

> The keyword stream that follows the stream dictionary shall be followed by an end-ofline marker
> consisting of either a CARRIAGE RETURN and a LINE FEED or just a LINE FEED, and not by a CARRIAGE
> RETURN alone. The sequence of bytes that make up a stream lie between the end-of-line marker
> following the stream keyword and the endstream keyword

A space is not an end-of-line marker, so a file that writes `stream`, a space and a line feed is
malformed. The clause states no recovery; it describes valid files, and what a reader owes a
malformed one is the corpus's question (`CLAUDE.md`'s two denominators). The row stays
`implemented`: a recovery is not a requirement.

## 2. The decision

The parser skips a run of spaces and tabs between the keyword and a CR or LF, then the marker, and
records how many it skipped. `Document::padded_stream_keywords` reports each such stream by object
number with that count, the way `misfiled_objects` reports a table repair: answerable, not silent.

**Why skipping is right and not only convenient.** The second sentence above puts the data after
the marker. Bytes before the marker are data on no reading of the file, and handing them to a
filter corrupts every stream whose filter does not delimit its own data — `CCITTFaxDecode` is one:
two leading bytes shift every scan line of a Group 4 image.

## 3. The guard, stated as the claim it is (trap 28)

- **Only spaces and tabs, and only where a CR or LF ends them.** `stream`, a space and then data is
  a file with no marker at all; the recovery is not for it, and the space is handed on as before.
- **Never past the first data byte.** The padding is skipped only up to the marker; the marker is
  the one §7.3.8.1 names, CR LF, LF, or the CR alone this parser already accepted.
- **The file's own length wins.** Where a direct `/Length` is confirmed by `endstream` only from the
  keyword's end — the padding counted as data — the stream is read that way and not reported. The
  indirect `/Length` gets the same guard one layer up, in `with_stated_length`, which is the only
  place a reference can be resolved.

The fixtures are the pairs: a conforming LF and CR LF (nothing reported); four paddings under a
direct, an indirect, a wrong and no `/Length` (skipped and reported); a space with no marker (left);
and a `/Length` counting the padding, direct and indirect (the file wins, nothing reported).

## 4. What it measured

- `4113564.pdf`: its twelve streams all pad the keyword with one space. Its six fax images now decode
  to every row `/Rows` states; with the space and line feed prepended, as before, every one fails.
- A textual scan of `corpus-cache` and `doc/corpora` (89 789 files): 33 documents write the shape,
  178 keywords. `pdf-model --test corpus` and `pdf-syntax --test on_disk` are unchanged.

## 5. Left

The report is on `pdf_syntax::Document` only; `viewer-core`'s notes do not say it yet.
