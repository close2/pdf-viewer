# 524 — The other content streams a clause names

**Finding.** ADR 0343 made a damaged page `/Contents` draw its prefix and say so, on §7.8.2's
first sentence: a content stream is "a sequence of instructions", so a prefix of one is a shorter
sequence of the same kind. **The next paragraph of the same clause names four more objects that
are content streams** — forms (§8.10), patterns (§8.7), Type 3 glyph descriptions (§9.6.4) and
annotation appearances (§12.5.5) — and this tree had been drawing all four of their damaged
prefixes without a word since the interpreter had them. `doc/todo/03` §10 recorded it as owed with
the count and the clause; this round took it.

The transfer is honest for every one of the five sites, and checking that is most of the work
rather than the wiring:

- **A form `XObject`** is §8.10.1's "self-contained description of any sequence of graphics
  objects" — the same thing under another name. Additive, drawn, reported.
- **A tiling pattern's cell** is stated as a content stream by §8.7.3.1 itself. The shortfall is
  *amplified* by the tiling, and is still additive: the shorter cell repeats at the file's own
  `/XStep` and `/YStep`, so what appears is a subset of what the producer asked for.
- **A Type 3 glyph description** is the one where the clause makes the rule *stronger* than
  elsewhere. Table 110 requires `d0` or `d1` as the stream's first operator, so a prefix carrying
  any mark carries the glyph's own declaration ahead of it, and `/Widths` rather than the
  description supplies the advance — the damage costs marks inside one glyph and never the
  position of the next. Exactly the property ADR 0343 found a damaged font *program* lacks.
- **An annotation appearance** §12.5.5 makes a form outright.
- **A soft mask's `/G`** is the one whose answer had to be derived rather than carried over, and
  the reason the round is not a formality. Its marks become mask *values* over other objects,
  which is the shape ADR 0356 refused for a sampled function. What decides it is that §11.6.5.1
  states the mask's value where the group painted nothing — the transfer function of 0.0 for
  `Alpha`, `/BC`'s luminosity for `Luminosity`. A place the damage took is a place the group did
  not paint, and the clause already answers for one of those. **Places, not values**, so the
  prefix stands.

**And one entry of §10's list is not a content stream at all**, which is worth more than the four
that are: it named `type3.rs`, whose `decoded_stream_data` reads the font's **`/ToUnicode` CMap**
(§9.10.3), not its glyph description — that is decoded in `content/text.rs`. A CMap paints nothing,
its prefix is a smaller *mapping*, and the codes it fails to name already land in
`codes_without_a_character` as `IncompleteToUnicode`. It stays silent, on ADR 0152's trade. §10's
list was written from file names rather than from what each site reads, and the correction is
recorded there.

**Date.** 2026-08-14.
**ADR.** [0359](../adr/0359-the-other-content-streams-a-clause-names.md).
**Touched.** `crates/pdf-model/src/content/run.rs` (`Interpreter::content_stream`, the whole
mechanism), `crates/pdf-model/src/content/report.rs`
(`Unsupported::DamagedContentStream`, `DamagedStream`), `crates/pdf-model/src/content/xobject.rs`,
`crates/pdf-model/src/content/pattern.rs` (`tiling` gains the name `scn` used),
`crates/pdf-model/src/content/transparency.rs`, `crates/pdf-model/src/content/text.rs`,
`crates/pdf-model/src/content/annotations.rs`, `crates/pdf-model/src/annotation.rs`
(`appearance_damage`, `Appearance::damaged`), `crates/pdf-model/src/type3.rs` (`glyph_name`),
`crates/pdf-model/src/content.rs`, `crates/viewer-core/src/report.rs` (the sentence a person
reads), `crates/pdf-model/tests/damaged_content_streams.rs` (new, six),
`crates/pdf-model/tests/oracle.rs` (two entries out of `AMBIGUOUS_DENSE_TEXT_AT_PAPER_SIZE`, with
why), `crates/pdf-model/examples/damaged_stream_census.rs` (a line measuring what the *tree* says,
split so the before is derivable from one run), `doc/conformance/ledger.toml` (§7.8.2, §8.10.1,
§8.7.3.1, §9.6.4, §11.6.5.1, §12.5.5), `doc/HANDOVER.md` (trap 5's list is ten, and the prefix
test is asked per consumer), `doc/todo/03-more-corpora.md` (§9 and §10 closed, §11 opened),
`doc/environment.md` (the `git stash` hazard below), `doc/adr/0359-*` (new), this file.

## Why a new report rather than `ContentIssue::Damaged`

The vocabulary `doc/todo/03` §10 pointed at is reused where it is reusable — `Damage`, and the
argument under it — and not where it is not. `ContentIssue` is Table 31's noun: every variant is
indexed by a *part of `/Contents`*, and a form reached through `/XObject` or an appearance reached
through `/AP` has no such index. Same sentence, different subject.

## The one place the report cannot be made where the stream is read

§12.7.4.3 regenerates a widget's appearance by **splicing** new marks into the stored stream's
`/Tx BMC` … `EMC` region, so what reaches `draw_appearance` is a constructed copy with no stream
behind it. A report taken at the draw would go quiet for exactly the annotations a reader has typed
into. So `annotation::appearance_damage` asks where the stream still exists and `Appearance::damaged`
carries the answer to the drawing.

## What it matched, and what it cost

The census interprets every page of every document holding a damaged stream and counts the reports
naming damage, split into ADR 0343's route and this one — so the before is the total minus the split
and needs no second build. Populations are the files' and did not move; only the last column is
this tree's.

| population | damaged streams | of them form `XObject`s | reports naming damage | of those, new |
|---|---|---|---|---|
| pdf.js, 974 | 57 in 20 documents | 7 | 6 in 4 documents | **5** |
| `govdocs1-error-pdfs`, 54 | 296 in 29 | 0 | 100 in 27 | **0** |
| SafeDocs crawl, 65 944 | 2260 in 726 | 46 | 1182 in 432 documents | **295** |

`govdocs1-error-pdfs` is the useful negative: its damage is `/Contents`, images, font programs, an
ICC profile and a function, and not one of §7.8.2's other four — so the new report adds nothing
there. The crawl ran as 145 processes, 0 failures, and the census's older lines reproduce session
521's to the digit, which is what says the instrument did not move under this round's changes.

**295 new reports against 46 damaged form `XObject`s** is the crawl's own surprise. A report is per
*page*, so one damaged form drawn on forty pages is forty of them; and the rest are tiling patterns,
glyph descriptions and appearances, which the census files under **unclassified** because a content
stream's dictionary carries nothing saying who reads it. The role table's largest silent bucket is
substantially what this round made loud, and no count inside it could have said so.

**The one thing that costs work rather than a judged page was measured.**
`annotation::appearance_damage` decodes a stored appearance that `draw_appearance` then decodes
again — a cache hit plus the filter chain it is keyed on, once per annotation per interpretation.
Interpreting all 974 first pages is 47.5–54.1 s with the change against 48.4–52.1 s without, five
runs each side: the same distribution.

**Three pages left the oracle's judged set**, which is trap 11's price in its own units: 1694 pages
called complete before, 1691 after. They are `comments.pdf` page 1 and `highlights.pdf` page 1, both
`ambiguous` and both listed in `AMBIGUOUS_DENSE_TEXT_AT_PAPER_SIZE` — whose entries the gate itself
demanded be deleted, and are, with the reason written into the group — and `issue3885.pdf` page 1,
which `agrees` and was in no group. The corpus gate's incomplete list rises 61 → 64 by exactly those
three. The pdftotext gate loses the same three documents: 1180 words, **all of them matched**, so
its 179 unmatched words are the same 179 either side and the rate's 99.3% → 99.2% is arithmetic on a
smaller denominator rather than extraction that got worse.

**Nothing drawn moved.** `examples/display_list_digest` over all 974 pdf.js documents is
byte-identical across the change, and the three witnesses' page-one PNGs are byte-identical too — so
`doc/todo/00` step 7 needs no re-run and no quorra lane can have moved.

## The witnesses, looked at

`comments.pdf` and `highlights.pdf` are a PLDI paper with a reader's markup over it. Object 694 is
the form an ink annotation's appearance invokes; 851 bytes inflate and then the stream ends, with
the last of them a completed `S` closing a curve of the ink path. The green loop drawn round the
paper's title stops where the producer's stylus data stops. `highlights.pdf`'s object 667 is the
same shape one operator over — 648 bytes ending in a completed `f` for a highlight quad. Neither
page said anything about it before this round, which is trap 5's own sentence in a picture.

## `git stash` is shared between worktrees, and this round lost ten minutes to it

`refs/stash` lives in the common git directory, not in the worktree. This round stashed its changes
to take a baseline measurement, a parallel round pushed its own stash in the meantime, and `pop`
took the neighbour's half-finished `pdf-font` edit — leaving both trees wrong and neither saying so.
Recovered by `git checkout --` on the files it applied, `git stash store` to put the neighbour's
commit back at `stash@{0}`, and `git stash pop stash@{1}`. The rule and the recovery are in
`doc/environment.md` now; the replacement is a patch of one's own (`git diff > x.patch`,
`git apply -R`, measure, `git apply`), which touches nothing shared.
