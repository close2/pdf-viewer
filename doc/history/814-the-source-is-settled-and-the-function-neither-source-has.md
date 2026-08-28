# 814 — the source is settled, and the function neither source has

**Finding:** the project owner decided where the ECMAScript host object model is to be read from —
Adobe's *JavaScript for Acrobat API Reference*, with `pdf-association/pdf-issues` secondary, and ISO
21757-1:2020 not to be bought — and reading both sources against the todo's proposed first step
found that **the step cannot pass its own test**: the `AF*` form-format functions it needs are in
neither source, and the PDF Association's working group has formally declined to specify them
(issue #100, closed *no fix*). Two smaller findings came with it: **`ISO 21757-1:2020 §9` passes the
citation gate in silence as a citation of ISO 32000-2 §9**, because the year defeats the
foreign-document guard; and **`pdf-association/pdf-issues` is not a new source to this tree at all**
— it is the upstream of `tools/spec-errata`, whose annotations have carried its issue numbers since
ADR 0252.

Date: 2026-08-28. Branch `round-814`, from `main` at `7619c4ab`.
ADR: **none.** A round that records the owner's decision rather than making one should not take an
ADR number, and this one did not: the argument in the file is the owner's, the limit written under
it is `CLAUDE.md` principle 5 applied unchanged, and nothing was decided that a later round would
need the reasoning for. `doc/todo/56` §3 holds all of it.
Files: `doc/todo/56-a-script-engine-that-is-memory-safe.md`, this file. No `Cargo.toml`, no ledger
status, no source, nothing under `doc/rfc/`, no line of `CLAUDE.md`.

## What was asked

The owner, verbatim:

> "we won't obtain ISO 21757-1:2020 (it costs something). The adobe javascript reference is good
> enough. ISO 21757-1:2020 directly standardized Adobe's existing Acrobat JavaScript object model
> into an ISO standard. The core objects you need in a viewer—such as Doc, Field, event, util,
> color, and app—are documented in depth in Adobe's JavaScript for Acrobat API Reference. There is
> https://github.com/pdf-association/pdf-issues should be treated as secondary reference. Please
> update the todo."

Round 809's §9 step 4 said acquiring ISO 21757-1 was "not optional" and that principle 5 could not
be satisfied without it. That step is answered and rewritten; the exclusion itself is untouched, and
§8's four counter-claims stand exactly as 809 wrote them.

## The principle-5 question, which is the part worth the words

`CLAUDE.md` principle 5 demotes poppler, mupdf, pdf.js and Acrobat to *evidence about our reading*.
What puts them there is their **position**: each is written downstream of ISO 32000-2, so its output
is another party's reading of the same text, and agreement is confirmation rather than definition.
Adobe's reference is not in that position — ISO 21757-1 was made **from** it, so the vendor document
is the standard's ancestor rather than a competing reading of it, and the direction of derivation
runs Adobe → ISO where principle 5's inference runs the other way. That is the owner's argument and
it holds.

**It was checked rather than accepted**, because a claim about a document is checkable. ISO
21757-1's contents page, from the free preview, is Adobe's heading for heading: 10.5 `app`, 10.8
`color`, 10.13 `Doc`, 10.16 `event` with "Event type/name combinations", "Document Event
Processing", "Form Event Processing" and "event properties" beneath it, 10.17 `Field` with "Field
versus widget attributes", 10.37 `util`. The ancestry is visible in the table of contents.

**And the limit is written just as plainly, in three parts.** The derivation is *visible* and
nowhere *stated* — neither the Foreword nor the Introduction of the preview says the document
derives from an Adobe publication, so "Adobe is the ancestor" is this project's inference from a
structural match. Where the two diverge this project cannot tell, because there is no copy of ISO
21757-1 here and the preview stops before the first object type. And **they demonstrably do
diverge**: the secondary reference records `console` present in Adobe and absent from ISO (#744),
`ScreenAnnot` documented by Adobe and missing from ISO (#99), and `XMLData` referred to by ISO and
defined nowhere in it (#535). Three measured divergences settle it — the documents are not
interchangeable, so **every API detail taken from Adobe is a documented choice in principle 5's own
sense, never a derivation from ISO 32000-2.** What *is* derived from ISO 32000-2 is the dispatch:
where a script hangs, when it fires, in what order.

## The citation finding, measured rather than reasoned about

`§` in this tree means a clause of ISO 32000-2, and `tools/conformance` enforces it — so adding two
non-`§` sources has an instrument cost, and it was priced by feeding lines to
`conformance::citation::scan` and asking the clause index what each landed on (trap 13):

| written as | recorded as | verdict |
|---|---|---|
| `ISO 21757-1 §9` | a foreign citation | **caught**, with a message teaching "ISO 21757-1 section N" |
| `ISO 21757-1:2020 §9` | ISO 32000-2 §9, which exists | **silent pass, wrong standard** |
| `the JavaScript for Acrobat API Reference §12.5` | ISO 32000-2 §12.5, which exists | **silent pass, wrong standard** |
| `ISO 21757-1 Table 113` in a comment | ISO 32000-2's Table 113, "Additional entries in Mac OS Roman encoding not in MacRomanEncoding" | **silently the wrong table** |

The guard is `citation::another_document`, which recognises another document by an acronym followed
by a **plain number** — `RFC 3986 §5.2`, `ISO 15076-1 §6`. `21757-1:2020` is not a plain number, so
the year defeats it. **The spelling that fails is the natural one**, and round 809's file writes it
two dozen times.

No instrument change was made. This is a documents round, the hole has no occurrence in the tree
today, and closing it belongs to the round that writes the first such citation — which is why the
shapes to write are now rules in `doc/todo/56` §3 rather than a note. The one place the instruments
are already right is the blockquote: a rustdoc blockquote is compared against `doc/md/` and an
unattributed one is reported, so Adobe's words cannot wear the standard's clothes by accident.

## The secondary reference was already here, under another name

`tools/spec-errata` reads the PDF Association's errata as annotations on the standard's own PDFs,
and every annotation carries a GitHub issue number. The `#181`, `#293`, `#236` and `#374` filling
`doc/errata-read.md`'s tables **are** `pdf-association/pdf-issues` issue numbers. So the owner's
"secondary reference" names the upstream of an instrument this tree has had since ADR 0252.

The boundary that follows is clean and is now written in the todo: **for ISO 32000-2 the repository
must not become a second errata feed**, because `spec-errata`'s population is the annotated PDFs and
the repository's markdown is the same resolutions in another container — two instruments over one
population is how a project comes to believe whichever agrees with it, and `doc/errata-read.md`
already records a collection disagreeing with itself. **For ISO 21757-1 it is the only channel this
project can have**, since `spec-errata` reads `doc/*.pdf` and there will never be an annotated ISO
21757-1 among them. Its issue *threads* are not duplicated by anything here either: they carry the
argument, which an annotation does not.

That is where the round's best single fact came from. **Issue #185, closed and ISO-approved, pins
the language version, and neither ISO document states it**: as published, ISO 21757-1's normative
reference for the language is dated to a standard that itself refers to ECMA-262 undatedly, so the
published text fixes no ECMAScript version at all. The erratum replaces it with ECMA-262's eleventh
edition — ECMAScript 2020. Every engine-conformance claim in §2.1 should be read against that.

## The re-pricing, which is the finding

809's smallest first step was `/AA /F` and `/AA /K` on one field, in a third worker, with the object
model limited to `event`, `this`, `util` and a read-only `Field`, and it named its own criterion:
does a currency field read `$1,234.50`.

**The members it needs are now a list of seventeen, and every one of them is documented** — eleven
`event` properties, four `Field` properties, four `util` methods — which is a *smaller* surface than
809 guessed, since `color` and `app` are not needed at all and `Doc` only for `getField`, which the
step excludes. That much is good news and is in the todo as a table a round can tick off.

**But the criterion is unreachable, and the reason is not in either source.** A field's `/AA /F` in
a real document is a call to `AFNumber_Format`. Those functions are in **neither** Adobe's reference
(measured: zero occurrences of `AF[A-Za-z]*_[A-Za-z]+` across the whole reference and the developer
guide) **nor** ISO 21757-1 (its contents run `Annotation` … `util` with no such clause). Acrobat
supplies them from its own shipped library, and the step excludes document-level scripts, so nothing
can supply them. **The step as designed evaluates the call and raises a reference error on every
document it is aimed at.** Adobe's only published description of the family is an argument table in
a *different* book — the Interapplication Communication guide — which gives parameter menus and no
algorithm, documents one argument for two of its four values, mislabels another, and omits half the
family.

Issue #100 is the record of that being a decision rather than an oversight: the working group closed
"no fix" on Adobe's statement that these are undocumented private methods it has chosen not to
standardise. **It is `CLAUDE.md`'s two denominators pulling apart in one page** — the specification
denominator says the functions do not exist, the world denominator says a large share of form
documents call them.

**The adjustment recommended in the todo is to change what the step is judged on**, not to grow it:
run the trigger, evaluate the script under the worker's budgets, apply an assignment through the
action log beside the document, and refuse everything outside the seventeen members **by name, out
loud** — trap 5, which is what this tree does with every other refusal. On the documents §7 aims at
that reports `AFNumber_Format` refused rather than a formatted field, and the step is then judged on
four things the sources fully determine instead of on a formatting result nothing specifies. The
`AF*` layer becomes the second step, ahead of the calculation chain, with its own argument — because
implementing it would be the largest block of documented-choice code this tree has ever taken, and
it should not arrive attached to a step whose purpose is to prove a worker and a protocol.

## What else the reading turned up, and what was deliberately not done with it

- **Adobe's reference has no edition to cite.** The PDF is retired and 301s to a landing page; the
  live document is an HTML pair with no version number and no version segment in its URL, content
  frozen at Acrobat XI, and Adobe's own canonical short link redirects to a 404. So the todo carries
  a pinning rule: cite the commit of `adobe/dc-acrobat-sdk-docs`, which is public and **MIT**, the
  licence covering "this software and associated documentation files" — better terms for quoting
  than the retired PDF's.
- **Five things an implementation needs that neither source states**: the trigger firing order
  (published only as a PNG state diagram), what happens when a trigger script throws, `util.printd`'s
  locale, `Field.value`'s string-to-number conversion, and `Calculate` re-entrancy. Adobe also
  contradicts itself about `event.rc` on Validate in consecutive sentences. These are recorded
  because they are places the standard **would not have helped either**, so the owner's decision
  costs nothing there.
- **A stale claim was found and left alone, on purpose.** `doc/errata-read.md`'s *Owed* section says
  the populations nothing reads at all include "a quotation in a Markdown file under `doc/`" — and
  `cargo run --release -p conformance --bin quotations` reads exactly that population, printing its
  count of documents on every run. The sentence has decayed. It is one clause of another file's
  record and correcting it is not this round's subject; it is noted here so the next round touching
  `doc/errata-read.md` has it.
