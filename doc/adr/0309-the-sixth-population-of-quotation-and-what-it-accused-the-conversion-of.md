# ADR 0309 — The sixth population of quotation, and what it accused the conversion of

Status: accepted, session 474.

## Context

`CLAUDE.md` principle 5 states the rule without an exception:

> **Quotation marks mean verbatim.** A load-bearing normative sentence goes in as a rustdoc
> blockquote, exact, under its clause number, so that the conformance checker can verify it against
> `doc/md/`. Anything less than verbatim is prose *without* quotation marks: paraphrase is fine and
> often clearer, paraphrase that claims to be a quote is not.

`tools/conformance`'s gate reads **one** population — rustdoc blockquotes under `crates/`, `tools/`
and `fuzz/`. Five more have been counted since: quotation marks in rustdoc prose, in ordinary `//`
comments, in `ledger.toml` notes, and the same spans asked whether an erratum struck them (ADRs
0249, 0254, 0255). `doc/todo/48` named a sixth and did not take it, saying why:

> **A sixth population is named and not counted**: every quotation of the standard in `doc/*.md`,
> in `doc/todo/`, in `doc/HANDOVER.md` and in the 255 ADRs. Nothing reads any of it. The reason to
> expect something there is the only reason any of these five was swept — each of the first sweeps
> found something.

It is the largest of the six and the oldest: hundreds of rounds wrote it, each quoting the standard
from memory or from a conversion, and not one word of it had ever been compared with anything.

## Decision

**`conformance::prose`, and a binary rather than a gate.** `cargo run --release -p conformance
--bin quotations` reads every Markdown document this project wrote under `doc/` — everything except
`doc/md/`, which is the standard itself, the third-party checkouts, and the errata file
`spec-errata` generates — and judges each quotation against all fourteen specifications.

Not a gate, for ADR 0249's reason one population over: a `> ` under a clause number is unambiguous
and a pair of `"` in a paragraph means nothing at all. These documents quote `CLAUDE.md`, the
project owner, another renderer's output, a report this program prints, a test's name, and their
own retired wording — the last of those *by design*, since `doc/todo/01`'s fourth sweep is "this row
said *X*" and every correction this project has ever recorded quotes the sentence it retired.

**The discriminator is ADR 0249's and it is what makes the output readable**: report only a
quotation matching a specification for at least five words *and* at least half its length, and then
diverging. Four coarsenings on top of `quote::normalise`, each of which can only hide a finding and
never invent one — spaces out (ADR 0253's repair for the conversion's broken words), case folded,
square brackets dropped so that `[t]he` is the exact quotation it is meant to be, and dash shapes
folded together. Two more were added by the first run and each removed a whole class of noise:

- **The two quotation marks fold together.** A quotation delimited by `"` cannot contain a `"`, and
  every cross-reference in the standard carries one — `(see 9.8, "Font descriptors")` — so a writer
  substitutes an apostrophe. The substitution is forced by the shape of the mark rather than chosen.
- **Mathematical Alphanumeric Symbols fold to the letters they style.** ISO sets every variable in a
  formula in mathematical italic, so Table 191's inversion is `𝑓 (𝑥) = 1 – 𝑥` in U+1D4xx and the
  document quoting it types `f(x) = 1 - x`. Same letters, another font — which is what `normalise`
  already folds a curly quotation mark for.

**And the report prints the standard's continuation from the point of divergence**, read back
through an index from the folded text to the spaced text. That is the addition to the method: it
turns a finding from "this is wrong" into the correction, written out, and it is what let thirteen
of them be fixed in one sitting.

## What the first run found

**2671 quotations in 398 documents, before this file joined them. 1270 verbatim in a specification,
12 suspects, 13 corrections** — thirteen rather than twelve because one of them lives in four files
and one was found by the errata question instead. `cargo run --release -p conformance --bin
quotations` prints the current figures, and they are nine higher than the twelve because this file
and session 474's record each quote the wrong sentences in order to name them — which is the
fourth-sweep shape below, arriving in the round that built the sweep.

Three of them are the shape round 466 found in the ledger, and they are the ones that matter: **a
sentence quoted as the standard's that ISO 32000-2 does not contain.**

- **`/TR2` "shall be used in preference to `TR`"** (ADR 0204 and `content.rs`). Table 57's own
  sentence is "[i]f both TR and TR2 are present in the same graphics state parameter dictionary, TR2
  shall take precedence." The invented one is §8.11.2.2's, about `/VE`, three hundred pages away.
- **"the first and last vertex shall be implicitly connected"** (ADR 0192 and `appearance.rs`), the
  reason a polygon's `/LE` is refused. §12.5.6.9 says the opposite sentence about the other subtype:
  a polyline is a polygon "except that the first and last vertex are not implicitly connected". The
  *inference* was right and the quotation was manufactured from it.
- **§7.5.5's `/Size` as "one greater than the highest object number used in the file"** (ADR 0130),
  which is half of Table 15's sentence and half of Table 17's. Table 15 says "1 greater than the
  highest object number **defined** in the PDF file"; "used in this section or in any section for
  which this shall be an update" is the cross-reference *stream*'s, two subclauses away.

Two are a word changed inside an otherwise exact sentence, and both had spread:

- **`/Root` as "[t]he catalog dictionary for the PDF document"** — Table 15's word is **file** — in
  four places: ADR 0097, `pdf-syntax/src/document.rs`, `pdf-model/tests/corpus.rs` and §7.5.5's
  ledger row. None of the four is a blockquote, which is exactly why the gate never saw it.
- **Table 31's NOTE 2 with a word inserted**: "the **document** catalog dictionary" for "the catalog
  dictionary" (ADR 0080).

Four are an elision that was not marked, which `doc/todo/01`'s eleventh sweep also found twice in
the ledger: ADR 0009 dropping "(screen or print)" out of the middle of a definition, ADR 0030 and
ADR 0071 running a cross-reference out of a sentence, ADR 0092 joining two sentences of §7.9.4 that
have two more between them. One is a **full stop closing a sentence the standard continues**
(`doc/todo/13`), and one is a bracketed substitution inside the quotation marks rather than outside
them (ADR 0089).

**And one is not a quotation at all.** ADR 0122 attributes `/H` to **Table 192** and `/H` is Table
191's; Table 192 is the appearance characteristics dictionary one row below it. That is the
`§9.3.6 Table 106` shape the thirteenth session found in the code, surviving in a document because
`tools/conformance`'s table check reads Rust sources and the ledger, and no document at all.

## The twelve that remain, and why the conversion is the first suspect

**None is unexplained**, which is the other half of a usable sweep:

- **Two accuse `doc/md/` and the PDF acquits the document.** Table 29's `/OpenAction` row ends "the
  document shall be opened to the top of the first page at the default magnification factor" and the
  conversion **truncated the row** before it; Table 179's `OpenArrow` description is intact in the
  PDF and the conversion **shifted the table's columns** through the middle of it. Both were checked
  with `pdftotext -layout` before anything was edited, and `doc/todo/48`'s warning is now evidence
  rather than advice: a silence in `doc/md/` is not a silence in the standard.
- **Eight are corrections quoting the wording they retired** — ADR 0249, `doc/history.md` and
  `doc/todo/01` reporting the very defects the ledger's own sweep found. Reporting them is right and
  suppressing them would be wrong.
- **One quotes an erratum's replacement text**, which `doc/md/` cannot hold by construction
  (`doc/errata-read.md`).
- **One is a coincidence**: this project's own sentence "a content stream does not record" shares its
  first five words with ISO 14289-2's "a content stream does not indicate…".

## The errata question over the same population

ADR 0254's instrument was pointed at the new population too — `spec_errata::document_landings`,
`Quoted::Document`, no new argument needed because the erratum supplies the other side of the
comparison. **42 landings in the clause they cite**, and they are almost all correct writing by
construction: 32 in `doc/errata-read.md`, whose subject *is* the struck text, and five in ADR 0092
on §8.9.5.4 — the one clause `doc/todo/48` records this tree as knowingly implementing a retired
version of. Two were worth an edit: ADR 0145 quoted §7.11.4.1's "map name strings to file
specifications" after Issue #481 struck the sentence, and Table 33 still says it, so the citation
moved to the copy the standard still contains; ADR 0092's §7.9.4 GMT sentence now says which erratum
retires it.

## Discrimination, verified rather than assumed

A checker nobody has seen fail is a checker nobody has tested. One word was changed in a quotation
of §7.7.3.3 — `clipped` to `cropped` — planted in a document and swept:

```text
doc/todo/00-ambiguous-bucket.md:960: quoted
    quoted:  the region to which the contents of the page shall be cropped
    matched 11 of 12 words, then diverged
    standard: the region to which the contents of the page shall be clipped when output in a
              production environment (see 14.11.2, "Page boundaries"). …
```

The plant was removed; the same case lives on as a unit test, along with one for each of the four
foldings, so that a coarsening added later cannot quietly swallow a finding.

## Consequences

- **The sixth population is counted, and `doc/todo/48` item 3a closes.** What is left of that item
  is §8.9.5.4, §14.8.6.3's enclosure requirement, the single-quoted spans, and the two steps the
  file has carried since it was written.
- **A gate is still possible and still priced the same.** It needs a syntax saying which quotations
  are the standard's, which these documents have no more of than the ledger does — and now the cost
  is 1401 spans rather than 417.
- **A misquotation is a spreading defect.** Two of the thirteen were in four files and two files
  respectively, and in both cases the copies were in `crates/` where a gate exists and could not see
  them, because neither was a blockquote. The three populations `spec-errata` reads and the one
  `conformance` gates are the same sentences; **only the gate's own is checked against current
  text**, and this sweep is the first thing that has ever asked the others.
- **The instrument is cheap to re-run**: four seconds over 398 documents and 27 MB of
  specifications, with no state. It belongs in `doc/todo/02` §4 with the other sweeps.
