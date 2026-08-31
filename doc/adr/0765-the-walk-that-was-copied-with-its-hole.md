# ADR 0765 — The walk that was copied with its hole, and a second feature there was none of

Status: accepted, 2026-09-01. Session 838. Cites ISO 32000-2 §9.7.4.2, §9.7.5.1 and its NOTE,
§9.7.5.2, §9.5's NOTE 5, and Annex P. Amends ADR 0764's population claim, ADR 0350's figures and
`doc/todo/21` §7's two open remainders; corrects §11.7.2's ledger row. Takes no decision about how
a page is drawn: every line of it is about what an instrument's population is.

## 1. A defect that was found once and fixed in one place

ADR 0764 found that `vertical_form_census` walked `xref().object_numbers()` and therefore could not
see `issue11555.pdf`, which writes its whole `Type0` inline in a page's `/Resources`. It fixed that
census and wrote the limitation into §9.7.4.2's ledger row for the census it had been *copied from*:

> That census recurses into nested dictionaries now and this one does not, so its figures are a
> floor rather than a total.

An honest sentence, and the wrong resting place for it. A known defect written down beside the code
that has it is a defect the tree still has, and the row's figures — quoted in §9.7.4.2 and standing
behind ADR 0350's whole justification — were a floor of unstated depth. So the first half of this
round is the same three-line recursion in `hollow_glyph_census`, and the depth measured rather than
guessed.

**Calibrated as trap 13 asks, by building both walks and running them side by side** over the 974
pdf.js documents from one source tree, one profile, one build directory:

| | old walk | with the recursion |
|---|---|---|
| `CIDFontType2` dictionaries | 268 | **275** |
| readable embedded `/FontFile2` | 221 | **227** |
| reaching glyphs through a `/CIDToGIDMap` stream | 42, in 30 documents | **43, in 31** |
| embedding a program some of whose glyphs are empty | 214 | **219** |
| a wholly hollow program under a stream | 0 | 0 |

**The one document the recursion added is `issue16553.pdf`**, which is this census's own
`issue11555.pdf`: a `CIDFontType2` with a remapping stream, written inline, invisible to a walk of
the objects the table names. Finding it is what makes the fix a measurement rather than an
assertion — trap 25's whole subject is that a narrow population and a clean tree print the same
thing.

**Over the crawl, one run**, 65 944 documents and 65 703 opened: 123 635 dictionaries, 121 175
readable embedded programs, **11 184** streams over 4296 documents (the row said 11 095), **88**
wholly hollow programs over 79 documents (the row said 86), 119 553 partly hollow — and the
intersection is **three**, the same three documents as before: `0100390.pdf`, `1776718.pdf`,
`1899548.pdf`. So the finding ADR 0350's fixture rests on survives the honest population unchanged,
which is the outcome worth having: the *floor* moved and the *conclusion* did not.

The lesson is one sentence and it is not about fonts. **A population defect found in one instrument
is a population defect in every instrument that was copied from it**, and the copy is findable —
`vertical_form_census`'s own doc comment named the walk it took. Fixing one and writing the other
down is how a tree keeps a defect it has already diagnosed.

## 2. A second registered feature, and there is none to consult

ADR 0764's crawl run named `7311602.pdf` — a face that supplies `VerticalText.pdf`'s brackets and
states no form for Adobe-Japan1's small kana — and left the obvious question open: is consulting a
second registered vertical feature warranted? That is a measurement before it is a decision, and
this round made it rather than reasoning about it.

**What the document loses.** `PDFVIEWER_TRACE_VERTICAL_FORM=1` over all four pages names 33 codes
and eight distinct characters: っ (17), ヶ (5), ッ (4), ィ (3), ョ, ャ, ァ, ょ (1 each). They are
shown through non-embedded `HGMaruGothicMPRO` and `MS-Mincho` descendants stating
`Adobe-Japan1` supplement 4 under `Identity-V`, and `installed_covering` answers all four of the
document's fonts with one face.

**What the face carries.** `examples/vertical_feature_census` is the instrument, and it prints two
populations for the same reason `vertical_form_census` does — the collection's, which is the same
everywhere, and this machine's, which is not:

- Adobe-Japan1's own `CMap` pair (`UniJIS-UCS2-H` against `UniJIS-UCS2-V`, Table 116's names) gives
  **251** characters a distinct vertical form. That is the collection's statement, read the way
  `predefined::is_vertical_form` reads it, over the whole UCS-2 code space rather than one
  character.
- Of **2652** face files in the directories `substitute` walks, **5** can draw あ — the character
  `script_sample` judges an Adobe-Japan1 substitute by — and **2** state any vertical feature at
  all. Both are Droid Sans Fallback, and both state `vert` **alone**: 86 single substitutions,
  supplying **46 of the 251** forms, none of them a small kana.
- **Not one face on this machine states `valt`, `vhal`, `vkna`, `vpal` or `vrt2`.**

**The decision: no second feature is read.** Not because the reading would be wrong, but because
there is nothing to read. A `vkna` branch would be a code path no face the chooser can pick
exercises, and the only assertion available about it would be "on this machine, in September 2026"
— which is `doc/todo/21` §1's own objection to the per-character fallback, met here with the sign
reversed: there the machine's catalogue made the test unstable, here it makes the feature dead.

**And the price says the same thing from the other side.** 205 of the collection's 251 forms are
missing from the chosen face. The eight small kana are 3.2% of that, so the shortfall is
face-shaped rather than feature-shaped, and §9.5's NOTE 5 is the clause that says whose it is. What
would open the question is a *face* — one stating `vkna` or `vrt2` where `vert` is silent — and the
census above is what says whether one has arrived.

**A second sentence became a command on the same run.** `doc/todo/21` §7 carried "[o]nly `GSUB`
lookup type 1 is read … No face on this machine states one", written as an assertion with no
instrument behind it. The census prints the lookup shapes it finds under `vert` and `vrt2`, and on
this machine they are `single` and nothing else. The claim is the same; what changed is that it is
now re-derivable, which is what `CLAUDE.md` asks of a fact that can be counted.

## 3. §11.7.2's Annex P, which was never a debt

The round's spec-driven half, taken off `--bin blockers`. §11.7.2's row is `partial` and names two
things owed: the clause's CIE-based `should`, and then —

> Annex P's algorithm is the same subject and equally unimplemented.

Both halves of that are wrong, and the row disproves itself three paragraphs later by saying the
presses come "from Annex P's order".

**Annex P is informative.** Its title line says so and its opening sentence says it "illustrates a
possible algorithm", so it states nothing a `partial` row can owe. **And three of the four steps it
illustrates are carried out**, in code whose own comment has called the annex informative all
along (`transparency::page_press`): a non-isolated group, or an isolated one with no `/CS`,
inherits from its parent; a device blending space "first appl[ies] the default colour space
mechanism", which is §8.6.5.6's `/DefaultCMYK`; and a page group with no parent inherits "from the
output device, or from the output intent", which is §14.11.5's. The fourth — an isolated group
whose `/CS` is CIE-based using it, and the ancestor search behind it — **is** the `should` named in
the sentence above, so the annex illustrates this row's one debt instead of adding a second.

The status does not move: §11.7.2 stays `partial` on the `should` and on the shapes §11.6.6's row
still reports. What moves is the account of *why*, which had counted one debt twice and had named
an informative annex as one of them. `doc/todo/01`'s sixth failure shape, inside a single row.

## What this round did not do

- **Nothing that draws.** No interpreter, rasteriser or font-loading path changed; the two census
  examples and three ledger notes are the whole diff. The `codes drawn upright` silence line is
  unmoved, which is correct — the measurement found no reason to move it.
- **No `vkna`, and no widening of `Downward::form_of`.** §2 above is the argument.
- **No second crawl run.** The census walks the crawl once, and the pdf.js and curated runs are the
  control beside it, stated separately rather than merged — ADR 0490's rule.
