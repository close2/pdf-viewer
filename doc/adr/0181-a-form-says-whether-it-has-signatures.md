# ADR 0181 — A form says whether it has signature fields, and the clause says to ask

Status: accepted, 2026-08-04 (session 277).

## Context

ADR 0179's launch timeline put `pdf_model::signature::signatures` on the launch path at **1.7 ms**
for ISO 32000-2 — a document with 28 form fields and no signature at all.
[todo 42](../todo/42-the-launch-path.md) item 3 recorded it as "the empty answer should be
reachable without it" and left the question open.

Two things had to be settled before touching it, and both were measured rather than argued.

**Is it work page one pays for anyway?** No. The field objects live in object streams page one
never expands: `signatures` costs 1.9 to 2.4 ms whether it runs before or after `interpret`, and
page one costs 4.1 to 4.4 ms either way. Nothing is shared, so it is 1.7 ms added to every launch
of every document with a form.

**Is `signatures` load-bearing anywhere a `shall` depends on?** No, and this is the part that
decided the shape of the fix. It has exactly one caller — `viewer_core::notes::about`, which says
out loud what a document claims about itself the moment it opens. §12.8.2.2.1's `shall` about
preventing changes a `/DocMDP` signature forbids, and §12.8.2.3's obligation on a processor that
writes, both reach the signature through `signature::permissions`, which reads the *catalog's*
`/Perms` and never comes through the field walk.

## Decision

**Read §12.7.3's Table 225, bit 1, and skip the walk when the form declares no signature fields.**

> If set, the document contains at least one signature field. This flag allows an interactive PDF
> processor to enable user interface items (such as menu items or push-buttons) related to
> signature processing without having to scan the entire document for the presence of signature
> fields.

The second sentence *is* this decision. The flag exists so that a processor need not scan; reading
it is conformance rather than optimisation, and the row that said otherwise is corrected below.

Table 224 gives `/SigFlags` a default of 0, so an absent entry is a statement and not a silence:
this form declares no signature fields.

**Counted before it was trusted**, which is the habit §12.8.2.3's row earned in the
hundred-and-ninety-eighth session. Over the 974 corpus documents:

| | |
|---|---|
| documents with an `/AcroForm` | **163** |
| of those, stating `/SigFlags` at all | 9 |
| of those, with bit 1 set | **6** |
| of those six, with a signature field in the tree | **6** |
| of the 154 omitting the entry, with a signature field | **0** |

Nothing disagrees in either direction. And the standard's own worked example in §12.8.5 writes
`/SigFlags 3` beside its single signature field, which is what a conforming producer does.

**1.681 ms → 0.017 ms** on ISO 32000-2, a hundredfold, and the launch keeps the report.

## What it costs, stated

A file with a signature field that omits or clears bit 1 has its signature unreported. That file
is malformed by Table 225, no corpus document is one, and the loss is a *note* rather than a
rendered pixel or a permission: §12.8.6's permissions dictionary is unaffected, so a `/DocMDP`
that forbids an edit still forbids it. The unit test
`a_form_declaring_no_signature_fields_is_not_walked` asserts both halves of that on the same
objects — no signature reported, `Modification::FormFilling` still read — so the boundary is a
test rather than a paragraph.

## The ledger row that was wrong, and how

§12.7.3's row said:

> Not read: `/SigFlags` and `/CO`, which are signature and calculation *behaviour*

`/CO` is behaviour — §12.6.3's calculation order needs the script engine principle 5 excludes.
`/SigFlags` bit 1 is **not behaviour**: it is a statement about the document's *contents*, and one
this program was answering the expensive way three feet from where the file states it. This is
`doc/todo/01`'s failure shape 4 — the "what is not done" half of a note being wrong — and it
resisted the greps because the entry *was* named, with a reason that read plausibly.

Bit 2, `AppendOnly`, is genuinely behaviour and needs nothing: it asks a processor to warn
somebody "requesting a full save that signatures will be invalidated", a *may*, and this program
cannot reach it — `pdf_syntax::write` performs §7.5.6's incremental update and nothing else, so
every save it makes is the append the flag exists to steer a person towards (ADR 0121). That is
worth recording as *met by construction* rather than as unread.

## The lesson

**A cost on the launch path can be a clause nobody read.** The three ways this project has found
stale ledger notes all ask what a row *claims*; this one was found by a profiler, and the row was
waiting at the other end with a wrong reason in it. Startup measurement and conformance reading
met in the middle, which is the two tracks `CLAUDE.md` asks every round to take from, arriving at
one entry from opposite directions.
