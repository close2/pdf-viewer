# ADR 0273 — A clause number the errata gave to a different clause

Date: 2026-08-11 (session 437)
Status: accepted

## Context

Session 437 was a sweep round over `doc/conformance/ledger.toml` and `SOURCE_ROOTS`. Its tenth
sweep — a parent row's stated count of its children, against what the rows below it say — printed
`§14.8.4.7` as "See the three below" above **four** child rows, two of which carry the *same*
title:

| row | title | note |
|---|---|---|
| §14.8.4.7.3 | Ruby and warichu elements | `Ruby`, `RB`, `RT` and `RP` |
| §14.8.4.7.4 | Ruby and warichu elements | `Warichu`, `WT` and `WP` |

§14.8.4.7.3's note explained the duplication and got it exactly backwards:

> **This clause number appears twice in `doc/md/`**, once here and once in an annex; the citation
> checker finds the first, which is this one.

It is not an artefact of the conversion. **Errata Collection 3 Issue #133 inserts a new subclause
at that number and renumbers the ruby one**, and says so in its own editor's note, which the
Markdown carries verbatim:

```
## 14.8.4.7.3 Link elements
…
## 14.8.4.7.4 Ruby and warichu elements
## EDITOR NOTE: Clause is renumbered - existing text is unchanged.
```

So the ledger held §14.8.4.7.3's content under the number of a clause that had moved, and the
clause the number now names — **link elements**, with two `shall`s in it — had never been read
against this code. One row of §14.8's family already knew: §14.8.4.7.2's note, written in the
four-hundred-and-eighteenth session, cites "§14.8.4.7.3's link element" correctly. Two rows about
one mechanism, disagreeing — `doc/todo/01`'s seventh failure shape, one directory apart.

`crates/pdf-model/src/structure.rs` had the same lag in a second place: `Annot` and `Form` were
documented as "an association between content and …", the Table 368 wording Issue #437 replaced
with *encloses*, and which §14.8.4.7.2's ledger row was corrected for in the same session that
left the doc comments alone.

## Decision

**`ClauseIndex::title` takes a number's *last* heading rather than its first.**

A corrigendum is printed after the text it amends, so where one number carries two headings the
later is the current one. This is one line and it moves exactly one row's title, because the
population was counted before the change rather than after: of the standard's **1017** numbered
headings, **one** number occurs twice, and it is this one.

`ClauseIndex` already keeps every occurrence and searches all of them for a quotation — the module
comment has said "`14.8.4.7.3` does, once in the body and once in the corrigendum that renumbers
it" since Annex O arrived. What it did not do is choose between them when asked for a *title*, and
the ledger's row set is generated from that answer.

Two rows follow from the title, and they are the substance:

- **§14.8.4.7.3 is Link elements**, and its reader's share is one sentence: "the link annotation
  shall use the QuadPoint entry to denote the active areas on the page". `pdf_model::link::region`
  has done that since §12.5.6.5 was followed, with Table 176's three conditions for ignoring the
  entry honoured exactly — so the clause is met by code that predates it, and what was missing was
  the row saying so. The other `shall` — a wrapped span gets "a single object reference" — binds a
  producer, and this crate validates no element's children.
- **§14.8.4.7.4 is Ruby and warichu elements, whole**: Table 369's seven types, not the three the
  old row named. The split the ledger carried is a split the standard does not make.

## The quotation that was ISO 32000-1's

The old §14.8.4.7.3 row quoted `RP` as punctuation "used only when a PDF processor cannot place the
ruby annotation text adjacent to the ruby base text". **ISO 32000-2 does not contain that sentence
anywhere.** Its Table 369 says "used only when a ruby annotation cannot be properly formatted in a
ruby style and instead is formatted as a normal comment, or when it is formatted as a warichu".

The eleventh sweep (ADR 0249) did not print it, and the reason is worth writing down because it is
a property of the instrument rather than an oversight: that sweep reports a quoted span only where
it matches the standard for **at least five words and at least half the quotation** before
diverging, which is what separates a misquotation from a claim this project invented. A quotation
of the *older standard* is neither — it shares "used only when a" and then goes its own way — so it
lands in the 497 spans that occur in no document under `doc/md/` and are not reported at all.

**A ledger written against ISO 32000-1 is a population the quotation sweep cannot see**, and the
ninth sweep — which finds ISO 32000-1's *table numbers* — is the only instrument here that catches
that population at all. This is the second kind of ISO 32000-1 residue it has found.

## Consequences

- One line in `tools/conformance/src/clause.rs`, one test, and one regenerated title.
- Two ledger rows rewritten, one source comment pair corrected, and two tests added:
  `a_link_element_names_one_annotation_whose_quad_points_are_the_active_areas`, which is the
  clause's own EXAMPLE 1 — one element, one object reference, sixteen numbers, two active lines —
  and `every_ruby_and_warichu_type_is_inline` for Table 369's seven.
- **No page changes.** Every non-comment line this round touched in `crates/` is inside a
  `#[cfg(test)]` module, which is why no gate over pages can see the round and none was run to
  claim otherwise.

## What would make this decision wrong

A second number occurring twice for a reason that is *not* a renumbering — the errata quoting an
unchanged heading, say. The defence is arithmetic and is cheap to repeat: count the numbered
headings, count the numbers that occur more than once, and read the pair. It was 1017 and 1 in this
round.
