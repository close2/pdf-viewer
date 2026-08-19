# 596 — The sentence that was about the next document

Annex O's `ef` row has two sentences and this tree carried out one. The `shall` — "the PDF processor
shall open the embedded file contained within the EmbeddedFiles name tree identified by name" —
landed in the four-hundred-and-seventy-fifth as `Event::Extracted` (ADR 0310); the one before it,
"[a]ny remaining parameters after this parameter apply to the selected embedded file", was the only
thing keeping §O.2.1's ledger row `partial` and the last unbuilt sentence in the annex. This round
took it, and Annex O now has four `implemented` rows and nothing reported.

Date: 2026-08-19.
ADR: [0431](../adr/0431-the-sentence-that-was-about-the-next-document.md).

## What `tools/state.sh annex-o` said

Before and after, unchanged and deliberately so: `carried out:` names all eleven parameters and
`reported:` is empty. That command reads `Parameter::unhonoured`, which is a claim about the eleven
*parameters* — and the sentence this round owed was never a parameter's refusal. **The instrument
that could see this one was the ledger row's note**, which said in words what the program did not do
and would have gone on saying it however green the gates ran. Worth remembering the next time a
count is asked to stand in for a reading.

## The three pieces, on boundaries that already existed

- **`pdf_model::fragment::Fragment::parse` stops at `ef`.** The remainder goes whole and undecoded
  into a new `after_embedded_file` field — a substring of the URI, not a re-spelling, because a
  fragment this reader had split and printed again would be this program's sentence rather than the
  one somebody wrote. Parameters belonging to another document no longer land in `parameters` at
  all, which is where the mistake was.
- **`Event::Extracted` carries it.** A variant changed shape; no message was added. A host *has*
  the fragment — it supplied it — so a new `Event` would have failed `doc/ui-boundary.md`'s test.
  What a host has not got is §O.2's grammar, and six consumers each splitting a fragment is six
  chances to disagree about where a parameter ends.
- **A host composes `Command::Open`.** `viewer-ui` opens the embedded bytes into its window with the
  remainder as their fragment. `viewer-confined` carries the field over the pipe; the other four
  consumers needed no line, because every one of them already matched `Extracted` with a `..`.

## What the window did

```
doc/pdf.js/test/pdfs/issue17056.pdf: 1 page(s)
note: this document carries an embedded file: destination-doc.pdf, 10305 bytes
note: and the fragment continues `page=3` after it, which applies to that embedded file rather
      than to this document and travels with it
opening the embedded file "destination-doc.pdf" at `page=3` (§O.2.1)
destination-doc.pdf: 30 page(s)
```

and the title bar read **destination-doc.pdf — 3 — page 3 of 30**. A gate could not have seen this:
a fragment identifier arrives with the request and no corpus document contains one.

## Two policy questions where there was one

`viewer_host::may_open_extracted` joins `may_write_extracted`. Showing the file in this reader is
the row's own `shall` and is answered `Ok` for both provenances; writing it into somebody's
directory is what the row's caution is about and is still declined for a URI. Both are functions so
that `doc/todo/38`'s *ask* and *warn* levels change one place, which is what `CLAUDE.md` §3 asks of
a restriction.

## What was not needed

No counter on the `ef` chain — the remainder is strictly shorter at every open, so a document
embedding itself runs out of fragment. No URI parser in `pdf-model`, which is the boundary
`doc/todo/39` drew: what the crate learned is the *end* of a fragment, which is Annex O's own
grammar.

## What is left, and what it is blocked on

Nothing in Annex O. `doc/todo/39` is closed and kept as the reading beside `tools/state.sh annex-o`.
Two limits stay named as *not* refusals of this annex: XFDF wants an XML parser (a dependency,
§12.7.6.4's row), and where a host looks for `fdf`'s file is a statement about these three hosts.
The four restriction levels are `doc/todo/38`'s.
