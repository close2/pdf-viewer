# ADR 0207 — The number the parser was standing on

Status: accepted, 2026-08-06 (session 361).

## Context

ADR 0206 gave the conformance ledger the standard's normative annexes. Annex I came out `partial`
for a sentence nobody had read:

> If a PDF processor opens a PDF file with a version number newer than the version that it supports
> or it identifies document requirements (12.11, "Document requirements") that it is not prepared to
> process, it should warn the user that it is unlikely to be able to read the document successfully
> and that the user may not be able to change or save the document.

The second half was met — `requirements::unmet` names what §12.11 asks for and this program cannot
promise, and `viewer_core::notes` says it before a page is drawn. **The first half was not, and the
reason is the small kind that lasts**: `xref::read` searches the first kilobyte for `%PDF-` because
§7.5.2 makes every byte offset relative to it, and then reads no digits. The number was under the
cursor for three hundred and sixty sessions.

## Decision

### One version, from the two places that state it

`pdf_syntax::Version` is a pair of numbers, not the name the file writes. Table 29's rule is *later
than*, and `"1.10"` beside `"1.7"` is exactly the string comparison that gets *later* backwards.

`Document::version()` reads both and ranks them the way §7.7.2 does — "[i]f the header specifies a
later version, or if this entry is absent, the document shall conform to the version specified in
the header" — so the catalog's entry counts only when it is later. A `/Version` written as a *number*
is not the entry at all: the table says "[t]he value of this entry shall be a name object, not a
number", and a reader that accepted both would be reading an entry the standard does not define.

The header is parsed as strictly as §7.5.2 states it — "'%PDF1. n ' or '%PDF2. n ' … where ' n ' is
a single digit number between 0 (30h) and 9 (39h)" — because being liberal here invents a number the
file never said. It is read from the same window `xref::read` measures offsets from, so NOTE 1's
"arbitrary bytes preceding the %PDF-" cost the version nothing.

**Nothing is read at open.** `version()` walks the first kilobyte when somebody asks, which is once
per document in `notes::about`; principle 2's rule is that nothing joins the launch path without a
reason, and a number no page needs is not one.

### The warning, and what it is worth

`notes::about` gains Annex I's sentence, beside the four claims about the file it already makes.

**No corpus document reaches it.** The 974 headers are 354 at 1.7, 235 at 1.4, 212 at 1.6, 104 at
1.5, 41 at 1.3, 9 at 2.0, 7 at 1.2, 5 at 1.1 and 5 at 1.0 — measured before the code was written,
which is `doc/todo/13`'s rule and the reason this took an afternoon rather than a week. So the note
exists for a file that has not been written yet, and the test builds one: a `%PDF-2.1` document
warns and the same document at `%PDF-2.0` says nothing.

That is the honest shape of a **coverage** obligation. A corpus cannot rank a requirement no
document exercises, and this one will be exercised by every file written after the next revision of
the standard — at which point a viewer that says nothing is a viewer that looks broken rather than
old.

## Consequences

- **§I.2 goes from `partial`-with-nothing-read to `partial` for a writer's sentence only.** What
  remains is "a PDF file's version should never be changed to an older version", which §7.5.6's
  incremental update meets by leaving the header alone and writing no `/Version` of its own.
- **`pdf_syntax::Version` is public and ordered**, which is what a host asking "what is this file"
  will want. Nothing answers `Query` with it yet, and ADR 0178's rule says a model entry with no
  consumer goes stale — so this one has its consumer in the same commit, and it is the note.
- **Two rounds, one thread.** ADR 0206 found the annex because the ledger could not spell its
  number; this round paid the cheapest thing the annex asked for. The expensive one — Annex O's
  fragment identifiers — is `doc/todo/39` and waits for a host with a URI.
- **1065 tests** over the ten crates that touch PDF bytes, six of them new here: five in
  `version.rs` and one in `viewer-core`'s headless host, which is the only one that can see the
  note a person would read.
