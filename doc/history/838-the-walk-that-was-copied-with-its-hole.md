# 838 — The walk that was copied with its hole, and a second feature there was none of

Date: 2026-09-01. On `main` directly, from `41006a27`.

ADR: 0765 — the walk that was copied with its hole, and a second feature there was none of.

Touched: `crates/pdf-model/examples/hollow_glyph_census.rs` (the recursion, and `measure` split
into `collect_descendants` and `measure_font`), `crates/pdf-font/examples/vertical_feature_census.rs`
(new — and the first `examples/` directory `pdf-font` has had),
`doc/conformance/ledger.toml` (§9.7.4.2, §9.7.5.1, §11.7.2), `doc/todo/21-font-substitution.md`,
`doc/verify.md`, `doc/adr/0765-*`, this file.

## The primary item, part (a): the same defect one census over

Round 837 found `vertical_form_census` walking `xref().object_numbers()` and therefore blind to
`issue11555.pdf`, which writes its whole `Type0` inline. It fixed that census and wrote the same
limitation into §9.7.4.2's ledger row for `hollow_glyph_census` — the walk it had been *copied
from* — as "its figures are a floor rather than a total". Correct, and the wrong place to leave it:
a diagnosed defect written down beside the code that still has it is a defect the tree still has.

So the same three-line recursion here, and the depth measured rather than guessed. Both walks were
built from one tree and run side by side over the 974 (trap 13's rule — a sweep for a defect is
believed only after being run against the defect):

| | old walk | with the recursion |
|---|---|---|
| `CIDFontType2` dictionaries | 268 | 275 |
| readable embedded `/FontFile2` | 221 | 227 |
| through a `/CIDToGIDMap` stream | 42, in 30 documents | 43, in 31 |
| a program some of whose glyphs are empty | 214 | 219 |

The document the recursion added is **`issue16553.pdf`** — this census's own `issue11555.pdf`, a
remapping-stream font written inline. Finding it by name is what makes the fix a measurement:
trap 25's whole subject is that a narrow population and a clean tree print the same output.

One crawl run, which is what the round was budgeted: 65 944 documents, 65 703 opened, 123 635
dictionaries, **11 184** streams over 4296 documents where the row said 11 095, **88** wholly
hollow programs over 79 documents where it said 86 — and the intersection is still **three**, the
same three files. The floor moved and the conclusion ADR 0350's fixture rests on did not, which is
the outcome worth having.

## Part (b): the measurement said no, and said it twice

`7311602.pdf` loses 33 codes, and the trace names them: eight distinct small kana — っ (17), ヶ (5),
ッ (4), ィ (3), ョ, ャ, ァ, ょ — through non-embedded `HGMaruGothicMPRO` and `MS-Mincho`
descendants stating Adobe-Japan1 under `Identity-V`. The question ADR 0764 left open was whether a
second registered vertical feature is worth consulting. It is a measurement before it is a
decision, and the measurement is unambiguous.

`examples/vertical_feature_census` prints two populations for the same reason its neighbour does —
the collection's, which is the same everywhere, and this machine's, which is not:

- Adobe-Japan1's own `CMap` pair states **251** characters with a distinct vertical form.
- Of **2652** face files here, **5** can draw あ, **2** state any vertical feature, and both state
  `vert` alone — 86 single substitutions, **46 of the 251** forms, none a small kana.
- **No face on this machine states `valt`, `vhal`, `vkna`, `vpal` or `vrt2`.**

So nothing was implemented. There is no second feature to consult: a `vkna` branch would be dead on
every face the chooser can pick, which is `doc/todo/21` §1's objection to the per-character
fallback with its sign reversed — there the catalogue made the test unstable, here it makes the
code unreachable. And the price says the same thing: 205 of 251 forms are missing from the chosen
face, so the eight kana are 3.2% of a **face**-shaped hole rather than a feature this tree fails to
read (§9.5's NOTE 5 is whose it is).

The same run retired a second written-down sentence. `doc/todo/21` §7 asserted "[o]nly `GSUB`
lookup type 1 is read … No face on this machine states one" with no instrument behind it; the
census prints the lookup shapes it finds under `vert`/`vrt2`, and here they are `single` and
nothing else. Same claim, now re-derivable — which is what `CLAUDE.md` asks of a fact that can be
counted.

## The spec-driven half: §11.7.2's Annex P

Taken off `--bin blockers`. §11.7.2 is `partial` and named two debts, one of which was
"Annex P's algorithm is the same subject and equally unimplemented" — while the *same row* three
paragraphs later says the presses come "from Annex P's order".

Annex P is **informative**; its opening sentence says it "illustrates a possible algorithm", so it
states nothing a row can owe. And three of its four steps are carried out, in code whose own
comment (`transparency::page_press`) has called the annex informative all along. The fourth — an
isolated group with a CIE-based `/CS` using it — *is* the `should` named in the sentence above it.
So the row counted one debt twice and named an informative annex as one of them. Status unmoved
(`partial` on the `should` and on §11.6.6's reported shapes); the account of why is now true.

## Gates

The whole §2 sequence, on `main`: `fmt` and `fmt --manifest-path fuzz/` clean, `clippy --workspace
--all-targets` and `clippy --manifest-path fuzz/` clean under `RUSTFLAGS="-D warnings"` (one
`doc_markdown` finding in the new example, fixed), `nextest` and the doctests green, and all ten
corpus-facing gate lines green with the sandbox worker and `pdfref-hayro` built first. Sweeps run:
`blockers` (which chose the ledger row), `parts`, `quotations` — which caught two real defects in
this round's own writing, an unescaped `"` inside a TOML basic string that broke the ledger's
parse, and a `**bold**` inside what claimed to be a verbatim quotation — `pointers` and
`undenominated`. §5's binaries rebuilt and installed.

## What a later round should know

- **A population defect found in one instrument is a population defect in every instrument copied
  from it**, and the copy is usually findable: `vertical_form_census`'s doc comment named the walk
  it took. This is the second census in two rounds with the same hole.
- **The question "should we read another feature?" is answered by the catalogue, not by the
  registry.** OpenType registers six vertical tags; this machine has one. A census over the
  *registered* set is a census of the clause; a census over the *installed* set is a census of the
  defect (trap 13's second shape).
- Both halves of the vertical-form work are now commands rather than sentences, so neither claim
  can go quietly stale the way the "floor" sentence did.
