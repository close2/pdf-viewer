# ADR 0090 — The second file a form is made of

Status: accepted, 2026-08-01.

## Context

The conformance ledger's whole remaining silence was one file format. Fourteen rows: §12.7.8's
Forms Data Format across twelve subclauses, §12.7.7's named pages, and §12.1's aggregate over
them. Every other clause of the standard's eight technical clauses had reached zero.

FDF looks like the archetype of what `CLAUDE.md` excludes — a data-interchange format for
submitting forms to servers. It is not. §12.7.8.1 lists three uses and the third is a display
change and nothing else:

> It can also be used to export form data to stand-alone files that can be stored, transmitted
> electronically, and imported back into the corresponding PDF interactive form.

A field's appearance is laid out from its value (§12.7.4.3, ADR 0032), and §12.7.6.3's reset was
implemented in the ninety-seventh session on exactly that argument (ADR 0087): a reset changes
*which entry the value comes from*. An import changes *which file* it comes from, and everything
downstream of the value is the same code.

## Decision

**FDF is read, and an import is performed as a display change.** Three pieces, in three crates,
each of which already had its shape decided by an earlier session:

- `pdf-model/src/forms_data.rs` reads an FDF file. §12.7.8.1 makes this cheap in a way worth
  stating: "FDF is based on PDF; it uses the same syntax and has essentially the same file
  structure", and the four differences it then lists are all **relaxations** — the
  cross-reference table is optional, there are no incremental updates, the body is one required
  object, a stream length is direct. A reader written to survive damaged PDFs survives all four
  without a line of new code, so `pdf_syntax::Document` opens an FDF file unchanged and this
  module is the semantics on top.
- `ViewState::import` holds the new values beside §12.7.6.3's reset set, and `Field::read` takes
  one instead of walking the document's `/Parent` chain for `/V`.
- `viewer-ui` performs §12.7.6.4's import-data action, which is the half `pdf-model` cannot.

## §12.7.8.3.2 is one sentence, and both halves are executable

> Unless otherwise indicated in the table, importing a field causes the values of the entries in
> the FDF field dictionary to replace those of the corresponding entries in the field with the
> same fully qualified name in the target document.

**The same fully qualified name.** An FDF file states its fields as a `/Kids` tree and a document
matches them flattened, so `/T` is concatenated down the tree with §12.7.4.2's PERIOD — and the
pairing then runs through the *same* widget-by-name table §12.6.4.11's hide action and §12.7.6.3's
reset already use, so all three agree about what a field is called. A node with no `/T` of its own
is not a field and contributes no separator, which is §12.7.4.2's own rule read in the FDF
direction.

**Replace.** Nothing is written to the target document, for the reason `view.rs` exists at all.
An imported value is an override, and an FDF field stating no `/V` leaves the widget with **no
value** rather than with its old one — replacing with nothing is still replacing. That is the
same end state a reset leaves a field with no `/DV` in, and it draws the same way.

The words "unless otherwise indicated" are the flag entries, and Table 249 indicates precisely,
twice over: `/Ff` and `/F` replace, `/SetFf` and `/SetF` are applied before `/ClrFf` and `/ClrF`,
and the modifying pair "shall be ignored if an `Ff` entry is present". Two triples, one
arithmetic, written once in `FlagChange::applied_to`. The annotation triple is the one that
surprises: an FDF file with no values in it at all can still **hide a widget**, because Table 167's
Hidden flag is what `/F` replaces.

## Two clauses now replace a field's value, and they are kept disjoint

§12.7.6.3 and §12.7.8 answer the same question about one widget, and the answer is whichever was
performed last. Rather than write a precedence rule the standard does not state, `import` removes
a widget from the reset set and `reset_form` removes it from the imported map — so the two
collections are disjoint by construction and `FieldValue` has no precedence question in it.
`Field::read`'s `reset: bool` became `overridden: bool` for the same reason: §12.7.5.2.3 makes
`/AS` beat `/V` for a check box, and after *either* operation the file's `/AS` describes the state
that was just replaced.

## What is read and not applied, and why each

Principle 3's requirement is that none of this is silent. `FormsData::owed` names six things:
`/Pages` (template pages, which need §12.7.7's named pages), `/Annots` (Table 254's annotations,
whose `/AP` resolves in the FDF file's object space while the page they belong to is in the
target's — a second `Document` reaching the interpreter, which is a design question), `/JavaScript`
(principle 5's closed list), `/EmbeddedFDFs`, `/Differences` (the target's own incremental updates,
which applying would mean *writing* a PDF), and per field `/AP`, `/APRef`, `/IF`, `/A`, `/AA` and
`/RV`.

Table 246's `/Encoding` is the one refusal with a precedent behind it. Its default and its two
Unicode values are §7.9.2.2's own three and need nothing; its four registered character sets —
`Shift_JIS`, `BigFive`, `GBK`, `UHC` — are character-set standards published elsewhere, and
carrying their tables is the decision Table 116's predefined `CMap`s are refused under. The
refusal is **per string, not per file**: an FDF naming `Shift_JIS` may still carry a check box
whose value is a *name*, which no character set reaches. And the clause applies `/Encoding` to
"field name that is a string" as well as to a value, so an undecodable name is named too — such a
field is listed, matches nothing, and says why.

## The file name is the caller's decision, and the policy is written down

§12.7.6.4 says a processor "shall import data … from a specified file" and specifies nothing about
*which* files a document may name, because that is a property of the processor. So `pdf-model`
reads the action and performs none of it — `Request::Import`, exactly as `Request::Resolve` carries
a URI nobody opens (ADR 0070) — and `viewer-ui` states the narrowest policy that still performs the
action: a **single path component**, checked as a path rather than as a string so a separator this
platform recognises cannot slip through, resolved against the directory the open document is in,
and §12.7.8's FDF only. ISO 19444-1's XFDF is the same data in XML and is declined by name.

Where both files state §14.4's identifier, a mismatch is *printed and not obeyed*: the clause
states no rule against importing another document's data, and a form's fields may legitimately be
shared, so the person is told rather than refused.

## One change outside clause 12

§12.7.8.2.2 gives an FDF file the header `%FDF-1.n`, and §7.5.2's "byte offsets shall be
calculated from the PERCENT SIGN" applies to it unchanged. `xref::read` searched for `%PDF-` only,
so an FDF file with junk before its header would have had every offset short. It now searches for
`%PDF-` **first** and `%FDF-` only where that is absent — the order matters: a PDF whose first
kilobyte happens to contain the second marker must still be measured from its own header.

## The checker found a table the standard has

`tools/conformance`'s table-title check reported that ISO 32000-2 has no Table 246. It has one —
"Entries in the FDF dictionary" — and the conversion in `doc/md/` promoted its caption to a
markdown heading while every other caption in the same subclause is a bare line. This is the third
time a `doc/md/` artefact has been mistaken for a fact about the standard (Table 164's `/Di`
default was the first). The check now drops leading hashes before matching.

## Consequences

`silent` falls **14 → 1**, and the one left is §12.7.7's named pages. Twelve of the thirteen closed
rows are FDF and one is §12.7.6.4, which moved `reported → partial`. No gate moved — 89 corpus
documents incomplete, 65 contradicted pages, 97.8% text — which is the expected shape: no corpus
document carries an import-data action and none ships with an FDF file, so trap 8 applies in full
and every test here is synthetic.

The largest thing left undone is §12.7.8.3.4's annotations, and it is now a *named* design
question rather than a silence: drawing an annotation whose dictionary lives in one file onto a
page that lives in another means the interpreter taking two `Document`s, and nothing in this tree
does that yet.
