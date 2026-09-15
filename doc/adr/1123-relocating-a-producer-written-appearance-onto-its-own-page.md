# 1123 — Relocating a producer-written appearance onto its own page

Status: accepted. Session 1113.
Builds: the construction ADR 1120 reserved this number for — the clause-driven relocation its
amendment (`CLAUDE.md`'s authoring exclusion, fourth amendment) put in scope. ADR 1099 §4 declined
it on purpose and built the appended page as the fallback it stays; ADR 1025 is that page's
mechanism.
Context: `crates/pdf-transform/src/archive/{preserve,prepare,rewrite,report,mod}.rs`,
`crates/pdf-transform/tests/{archive,archive_corpus}.rs`.
Clauses: ISO 32000-2 §7.7.3.3 (Table 31), §7.9.5, §8.4.2, §8.9.7, §12.5.3 (Table 167), §12.5.5;
ISO 32000-1:2008 Annex C Table C.1.

## 1. What was built, and what it replaces

A forbidden annotation's normal appearance is a form `XObject` the producer wrote, and §12.5.5
fixes where a reader draws it. ADR 1099 kept those marks by *appending a page* that invoked the
stream, because writing operators into a producer's page was on the far side of a ratified
amendment. ADR 1120's amendment moved it to the near side — the marks are the producer's, the
placement is the standard's, nothing is invented — and reserved this ADR to build it.

So the primary answer for an annotation's marks is now **relocation onto the producer's own page**;
the appended page is the **fallback** for the two costs below, and stays the whole answer for
content that was never on a page (a metadata packet, ADR 1025).

## 2. The construction, and why §8.4.2 is the rule

The page's `/Contents` becomes an array (§7.7.3.3's Table 31 admits it; a single stream is flattened
to it by reference, no producer byte moved): a **`q`-prepend** stream, the producer's own streams
unchanged, then a **closing** stream. The closing stream issues `depth + 1` `Q` operators —
`depth` being what the producer's content leaves open — which closes every state the producer left
and the prepended one, restoring the page's default user space; then it draws each appearance under
§12.5.5's own matrix. Every `q`/`Q` written is balanced by another, so **§8.4.2** holds: "Occurrences
of the q and Q operators shall be balanced within a given content stream (or within the sequence of
streams specified in a page dictionary's Contents array)." The rule is §8.4.2, the *balance*, not
§8.4.4. `depth` is counted by lexing the concatenated content, skipping §8.9.7 inline-image data via
`pdf_model::inline_image::scan` so a `q` byte inside a JPEG is not a save. The appearance is named
in the page's `/Resources` `/XObject`; nothing else on the page changes and no mark is composed —
the operators added *are* §12.5.5's placement of the producer's own stream.

## 3. The two refusals, each falling back to the appended page

**A producer that pops further than it pushes.** A `Q` with no matching `q` makes the running
balance go below zero; the prepended `q` would be what it restores, so every later mark would draw
under a state this conversion introduced. Refused by name (§8.4.2), and the marks take an appended
page. A `depth` past ISO 32000-1:2008 Annex C Table C.1's implementation limit of 28 (ISO 32000-2
prints no such table) is refused the same way.

**A remaining annotation over the moved marks.** §12.5.5 composites "with a backdrop consisting of
the page content along with any previously painted annotations", so marks moved into the content go
under every annotation that stays. The population is ADR 1120 §4's, and the load-bearing correction
is that **`/Annots` order is not z-order** — the standard states no painting order among annotations
at all. So the refusal cannot depend on order: it fires for **every remaining annotation** (not
removed) that §12.5.3's Table 167 does not hide (clear of `Hidden`, bit 2, *and* `NoView`, bit 6)
whose normalised `/Rect` (§7.9.5) overlaps, half-open, the moved appearance's device rectangle —
which is the removed annotation's own `/Rect`, since §12.5.5 fits the appearance to it. This
over-refuses relative to the unknowable true order, which is the safe direction. That appearance
alone takes an appended page; the page's other appearances still relocate.

## 4. That the costs are legible, not silent

`CLAUDE.md` principle 1: a difference a reader would find on the page is the failure to avoid. Each
refusal is a sentence on the preserved row (`Preserved::declined`) and in `--json`, so the report
says which of the two constructions each preserved thing got — ADR 1014 §5's fourth bullet asked of
a placement that appended nothing. The census in `tests/archive_corpus.rs` counts, over the corpus,
how many documents drive the remedy and how many marks each refusal sent to an appended page; the
three fixtures in `tests/archive.rs` are its calibration (trap 13), one planting each outcome.

## 5. What this does not do

It does not draw. An appearance `pdf_model::appearance` constructs is this program's picture, and
`A48` forbids relocating one and calling it the producer's — ADR 1099 §5 already refuses that case.
It does not touch the producer's bytes: §7.8.2 makes the `/Contents` array one stream, so the two
new streams are appended to it, never spliced into a producer's. And it inherits ADR 1099's
structure-tree refusal whole: a tagged document's added content would not be described, so the
preserve is refused rather than relocated — the appended page refuses the same case, so the parity
is exact.
