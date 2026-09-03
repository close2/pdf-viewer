# 0816 — The exclusion redrawn, and what `writer-side` now means

Session 886. Status: **accepted**. The sixth decision record of RFC 0002's implementation, on
the long-lived branch `round-867`, and the first that changes `CLAUDE.md`.

## Context

RFC 0002 §13's first question — "[r]atify §11.1's redrawn exclusion?" — had blocked the
serializer and all four writing verbs since session 867. `doc/todo/57` §2 said so in one
sentence: "[n]othing here starts before that sentence."

The owner said it on 2026-09-03, verbatim:

> RFC 002 and 003 are approved.

So this round is the one allowed to amend `CLAUDE.md`, and no round before it was. What the
amendment costs and what it buys were both argued in RFC 0002 §11 and are not re-argued here;
what this record holds is **what was actually written, where, and what the ratification does to
the conformance ledger** — which is §11.2 and is the half nobody had costed in rows.

## Decision

### 1. The exclusion's text, as RFC §11.1 drafts it

The entry in `CLAUDE.md`'s closed exclusion list was three lines and is now five paragraphs.
**One sentence was replaced and four paragraphs added; two paragraphs were kept unchanged.**

Replaced — the entry's first paragraph, in full:

> - **Authoring a document from nothing** — we do not *create* PDFs, and no clause whose
>   requirements fall on a generator is in scope: linearisation, object-stream packing,
>   optimisation, and the rest of what a producer owes.

by RFC §11.1's three paragraphs, verbatim: *Authoring content from nothing* (no layout engine,
no text-setting, no chart drawing, no "HTML to PDF"); *Assembling documents from existing
documents is in scope*, with the serializer named and "every content stream in their output is
a producer's, carried byte for byte or recompressed without reinterpretation"; and the third,
which bounds what comes into scope — "[g]enerator obligations come into scope only where the
serializer actually emits the construct: §7.5.4/§7.5.5/§7.5.7/§7.5.8 on the way out, §14.4,
§7.6 encryption on the way out. Annex F stays excluded until linearisation is separately
ratified."

**A fourth paragraph was added that RFC §11.1 prints outside its blockquote**, and adding it is
this round's judgment rather than the RFC's instruction: the *boundary line*, "does the
operation invent marks?", with rotate on one side and a watermark stamp on the other. It is in
because an exclusion the file cannot *apply* is an exclusion that decays — the entry without it
says what is in scope and gives nobody a test for the next case — and because the owner's own
sub-question with teeth (RFC §13 question 1) was exactly whether that fence is where it should
be. Ratifying §11.1 and leaving its fence out of the file would have ratified half of it.

Kept unchanged: the `pdf_syntax::Document` immutability paragraph, whole. Kept with one
sentence edited: the amendment-history paragraph, whose first sentence read

> **This exclusion was "we do not create files" and was amended by argument rather than by
> attrition.**

and now reads

> **This exclusion read "we do not create files", then "we do not *create* PDFs", and has been
> amended twice — both times by argument rather than by attrition** (the second on 2026-09-03,
> when the owner ratified RFC 0002 §11.1 with "RFC 002 and 003 are approved").

The rest of that paragraph — the incremental update, the producer's bytes staying byte for
byte, ADR 0100 — is untouched. **The edit was necessary rather than decorative**: a paragraph
saying the exclusion "was amended" once, sitting under an entry that had just been amended
again, is the shape of drift `CLAUDE.md`'s own rules exist to prevent, and the file's history
of its own wording is the thing a later session reads to know whether a claim is current.

### 2. `writer-side` is redefined by its boundary, not by its subject

`grep -n 'writer-side' doc/conformance/ledger.toml` was the worklist RFC §11.2 named, and it
prints its own size. Its header definition was

>   writer-side   addresses a PDF generator; this program writes only §7.5.6's updates

and the second half of that is now false. It is now

>   writer-side   addresses a generator; this program's writers emit structure, never content

which is `CLAUDE.md`'s own enforceable test in the ledger's vocabulary. The definition lives in
`tools/conformance/src/bin/ledger.rs`'s `PREAMBLE` and in `Status::WriterSide`'s doc comment,
because the generator stamps the header back into `ledger.toml` every time it runs — ADR 0345's
defect, which is why both copies were changed together and why the doc comment now records both
wordings it has retired rather than only the first.

### 3. What actually moved, and what did not — which is the honest half

RFC §11.2 predicted "§7.5.7's and §7.5.8's producer halves are the certain movers". Read
against the rows, **one of those two moved and the other was already `implemented`**, and three
rows the RFC did not name moved instead. The re-derivation, row by row:

| row | before | after | why |
|---|---|---|---|
| §7.5.2 File header | `implemented` | `implemented` | its note said "this tree writes §7.5.6's appends and never a header", which the serializer falsifies; the note now carries the header it writes and the binary-marker comment line |
| §7.5.3 File body | `implemented` | `implemented` | a body of indirect objects is now emitted as well as read |
| §7.5.4 Cross-reference table | `implemented` | `implemented` | a *whole file's* table, not only an update's subsection |
| §7.5.5 File trailer | `partial` | `partial` | a whole file's trailer is written; the `partial` is still the `/Size` reading departure, which is unrelated |
| **§7.5.7 Object streams** | `implemented` | **`partial`** | the serializer generates none, and that is a debt with a name: `optimize`'s `--object-streams=generate` |
| §7.5.8 Cross-reference streams | `implemented` | `implemented` | a whole file's stream is written |
| §7.3.8.2 Stream extent | `implemented` | `implemented` | `/Length` is now *stated* by this program, re-derived from the bytes written |
| §7.3.10 Indirect objects | `implemented` | `implemented` | the clause's null answer is now *written*, and counted |
| §7.7.3.4 Inheritance | `implemented` | `implemented` | the clause is now applied rather than only read: `split` flattens |
| §7.6.2 Application of encryption | `implemented` | `implemented` | a writer that applies none of it, stated as a choice with the warning it carries |
| §7.6.4.2 Standard encryption dictionary | `implemented` | `implemented` | Table 22's bit 11 is consumed for the first time |
| §14.4 File identifiers | `implemented` | `implemented` | a file is now *created* here, so "[w]hen a PDF file is first written, both identifiers shall be set to the same value" applies |
| §7.6.4.4.7–.9, Algorithms 8–10 | `writer-side` | `writer-side` | the serializer emits no `/Encrypt`; the notes now say that is the condition, so the rows move the day one does |
| Annex F (24 rows) | `out-of-scope`, `writer-side` | unchanged | `CLAUDE.md` says so in as many words; the annex row's note now rests on "this program's writers do not emit this construct" rather than on "this program does not write files" |
| §7.6.7, §14.12.2, §14.12.3, E.2, Annex L | `writer-side` | unchanged | none of them is a construct any writer here emits |

**The lesson is worth writing down because it cuts against the RFC that predicted it.** A
status is a claim about *this tree's code*, and a document written a hundred sessions earlier
cannot know which rows a landing will touch — §11.2 guessed from the clause numbers and got the
family right and the members wrong. Moving §7.5.7 and §7.5.8 because an RFC said they would
move would have been the corpus-going-quiet failure with a decision record for cover. What
moved is what the code changed.

## Consequences

- **The serializer is legal**, and ADR 0817 is what it is. `split` follows in ADR 0818.
- **Producer clauses are in the conformance denominator**, bounded by §11.1's "only where the
  serializer actually emits the construct". Two of them are already owed out loud: §7.5.7's
  object-stream generation, and §7.6's encryption on the way out the day a caller wants an
  encrypted derivative.
- **`writer-side` now decays the way every other status does.** Its old definition was a fact
  about the project that stayed true for seven hundred sessions; its new one is a fact about
  what this tree's writers emit, which changes whenever a writer grows. A round that adds an
  emitter owes this vocabulary a re-read, and `Status::WriterSide`'s doc comment says so.
- **Watermarking is now the first feature on the far side of a written line**, rather than
  something nobody had ruled on. Taking it is its own argued amendment.
