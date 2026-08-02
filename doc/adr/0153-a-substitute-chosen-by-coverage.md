# ADR 0153 — A substitute chosen by coverage

Status: accepted, 2026-08-02. Session 183. The item ADR 0152 priced, taken the next session.

## What was owed

ADR 0152 made a substituted font that draws none of its characters say so, and eight of the ten
documents it named are blank pages: `issue8372.pdf`'s 目录, `noembed-eucjp.pdf`'s あいうえお, and
six more. The cause is not §9.10.2's third method, which works — the codes become CIDs and the
collection's `-UCS2` table makes them characters. It is the *face*: `substitute::installed` ranks
candidates by the generic family a descriptor implies, and a Chinese font's descriptor implies
"serif" or "sans serif" like any other. The face that wins is Latin and has no glyph for anything
the document shows.

## The rule, and the part of it that is a choice

`substitute::installed_covering(request, wanted)` keeps the family match when it covers `wanted`,
and searches the machine's catalogue otherwise.

**What `wanted` is, is a choice, and it is made in the one place the standard leaves for it.**
§9.10.2 says how to find out what a code *means* and nothing at all about which face to draw it
with; §9.8.3's substitution hints are about weight, width and serifs, none of which distinguishes
a face that has Chinese from one that does not. So the rule taken is the cheapest true statement
available: **a face that cannot draw a character every font for this collection contains is not a
face for this collection.** One character per registry-ordering, from the collection's own script:
Adobe-Japan1's あ, Adobe-GB1's and Adobe-CNS1's 的, Adobe-Korea1's and Adobe-KR's 한. An
`Identity` ordering yields nothing and keeps the family match it had, which is right: its codes
index a font nobody supplied.

**Which of the qualifying faces is a second choice, and the first version got it wrong.** Taking
the first in path order gave `KanjiStrokeOrders.ttf` — a teaching font that has 的 and 中 and not
much else — and two documents that had been drawing went blank. The face taken is now the one with
the **widest repertoire** among those that qualify, counted from its `cmap`, which asks the
question the sample is a proxy for: how likely is this face to have the *rest* of the document's
characters. On this machine that is `DroidSansFallback`, which is what `fc-match` answers too.

## Cost, measured

The search reads font files until it has tested them all — 215 ms the first time on this machine's
catalogue — so two things bound it. It runs only where a composite font is substituted *and* its
`/CIDSystemInfo` names a registered ordering, which is ten of the 974 corpus documents; and the
answer is memoised on the characters asked for, so a document with three Japanese fonts walks the
catalogue once. Interpretation of `issue8372.pdf` is 29 ms with the memo warm against 8 ms before
the change and 215 ms without it.

Candidates are read straight from the filesystem rather than through `read_cached`, deliberately:
a coverage search touches most of the catalogue, and caching every face it rejects would hold the
machine's whole font collection in memory to answer one question. Only the winner is read again
through the cache.

The catalogue is sorted by path, so the answer is deterministic on one machine. It is not
deterministic *between* machines, and cannot be: ADR 0133 is why only §9.6.2.2's fourteen are
compiled in, and a page whose font nobody embedded is a page whose appearance depends on what is
installed. That is what `CONTRADICTED_SUBSTITUTED_FONT` has recorded since the
hundred-and-forty-eighth session.

## What it bought

- Corpus: **86 → 79 incomplete**. Eight documents draw where they were blank; two —
  `issue11555.pdf` and `issue2128r.pdf` — report where they had been drawing a *little*, because
  the face chosen for their collection covers the sample and not the characters they show. Both
  are honest: the machine has no face for those characters either way, and before this change
  nothing said so.
- Oracle: complete pages 1672 → **1679**, and `agrees` 840 → **847**. Seven pages that drew
  nothing now agree with the reference consensus. Contradicted is unchanged at 70.
- Text gate unchanged at 98.2%.

## What is still owed

A **per-character** fallback. This chooses one face for a whole font, so a document mixing scripts
in one font — or one whose characters the chosen face lacks, which is exactly `issue11555.pdf` and
`issue2128r.pdf` — still loses the ones it cannot draw. Every real text stack falls back per
character; doing it here means `LoadedFont` carrying more than one face's bytes and an outline
lookup that says *which*, which is a larger change than this one and now has two documents' worth
of evidence behind it.
