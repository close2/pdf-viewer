# 1012 — Where the effort goes: a measurement of the round

Status: **review** — read-only over `crates/` and `tools/` at commit `6c6a1d3d`, commissioned by the
project owner in these words: *"we are doing less and less real work … more than 50% is spent on
gates, merges, checks. If we advance the spec conformities that would be fine, but I think we were
currently wasting a lot of resources."*
Date: 2026-09-13. Session 1012, with four sibling rounds live in the same tree.
Proposals only — no ADR, on ADR 1005's precedent that a review proposes and the owner decides.
Record: `doc/history/1012-where-the-effort-goes.md`.

Every number below was produced in this session by a command over the repository. No corpus walk was
run: four rounds are using the machine, and where a number would need one the walk is named instead.
`§N` is ISO 32000-2 and nothing else; quotation marks mean verbatim.

---

## Verdict, in four sentences

The owner's read is right and the three-command check **understated** it: over the 58 sessions from
`2330a373` to `6c6a1d3d`, on the 875 clause rows that already existed, **exactly one row changed
status, and it moved backwards** — the entire `implemented` +5 is eight new clause-6 rows that ADR
0984 added. The 206 `partial` rows are not 206 pieces of work: **58 are aggregate parents that cannot
be worked on at all**, **60 of the remaining 148 are not gaps**, and the rows that are gaps collapse
into far fewer distinct debts than rows — §12.8's 22 rows are five debts. **31 leaf rows are closable
now**, ten of them for well under a round each, and the first four are hours. Against that, the
half-hour gate sequence's ~25 minutes of corpus-scale walking produced **one** self-caught defect in
98 sessions against **fourteen** false alarms, and seventeen of the twenty conformance sweeps have not
been run in sixty sessions.

The conclusion is not that the project is doing bad work. It is that the instruments, the gates and
the bookkeeping were each built by a round that had a reason, and **nothing has ever retired one**.
The cost is additive and the yield is not.

---

## 1. The ledger has not moved, and that is measurable

| | `2330a373` | `6c6a1d3d` | change |
|---|---:|---:|---:|
| rows | 875 | 883 | **+8** |
| `implemented` | 465 | 470 | +5 |
| `partial` | 204 | 206 | +2 |
| `reported` / `inapplicable` / `out-of-scope` | 17 / 67 / 114 | 17 / 67 / 114 | 0 / 0 / 0 |
| `writer-side` | 8 | 9 | +1 |

The eight new rows are `6.1`, `6.2`, `6.3`, `6.3.1`, `6.3.2`, `6.3.2.1`, `6.3.2.2`, `6.3.2.3` — ADR
0984's finding that `TECHNICAL_CLAUSES: 7..=14` had put the whole of clause 6 outside every
instrument. Six of the eight came in `implemented`, one `partial`, one `writer-side`.

Subtract them and diff the 875 rows that were common to both commits. **One row changed status in 58
sessions:**

```
8.6.5.5      implemented -> partial
```

§8.6.5.5 went *down*, because session 987 found the row had claimed `/Range` was read for nine
hundred sessions while `parse_icc_based` read `/N` and `/Alternate` and nothing else. That is an
honest correction and the right thing to have done — but the arithmetic it leaves is stark:
**net advancement of the pre-existing ledger over 58 sessions is −1.**

The long view is kinder and still shows the stall. `partial` peaked at **249** on 2026-08-08 and has
come down to 206 — 43 rows in 35 days — but 40 of those 43 landed before 2026-09-04. The last nine
days produced the −1 above.

Meanwhile the notes grew: the ledger file went from 2,300,606 to 2,377,985 bytes in the same window,
**+77 KB of prose against −1 row of status**. Sessions are writing into the ledger; they are not
moving it.

---

## 2. Are the 206 `partial` rows closable?

### 2.1 The arithmetic of the 206, before any reading

**58 of the 206 are aggregate parents** — a row with at least one descendant row, every one of which
has a `partial` or `reported` descendant. Checked mechanically: for each `partial` clause, is there
another row whose clause number is a strict descendant, and is any of them unsettled? 58 yes, and for
**all 58** at least one descendant is unsettled. Not one parent has an independent debt of its own.

These rows cannot be assigned to a round. They flip when their last unsettled child flips, and a round
that "works on §12.8" is working on §12.8.3.3.1. A further **six** leaf rows are sibling-aggregates of
the same kind (`7.7.2`, `8.7.4.1`, `8.11.1`, `8.11.4.1`, `12.1`, `12.8.2.1` — a "General" head whose
note says outright that it carries what the clauses beside it owe).

**The workable population is 148 rows, not 206.** An allocation of four round-slots in six aimed at
"`partial` rows" is aimed at a set 31% of which is bookkeeping.

### 2.2 The five classes

I read the `note` of all 206 rows (969,033 characters) and classified the 148 leaves. The classes are
the ones the brief suggested, with the evidence forcing one refinement: the largest class is not any
kind of blockage.

| class | rows | what it means |
|---|---:|---|
| **A — closable now** | **31** | a code gap this tree could fill; nothing outside it blocks |
| **B — blocked on a text** | 4 | names a specification not in `doc/` |
| **C — blocked on a capability** | 52 | names code, a host surface, a backend or an upstream release that does not exist |
| **D — blocked on a witness alone** | 1 | the gap is real and writable, and no document anywhere exercises it |
| **E — not a gap** | **60** | the row records a decision, a permission declined, or a claim that has decayed |

Class D is almost empty on purpose, and that is a finding rather than an omission: trap 8 already
tells a round to **build** the witness, and dozens of rows record having done exactly that
(`tests/ccitt_bound.rs`, `tests/jpx_channels.rs`, `tests/reference_xobjects.rs`, the §12.8 fixtures).
A missing corpus witness is a cost inside a row, not a blocker on it. The one genuine D is §12.7.8.3.3,
where no corpus document carries an FDF file at all.

**Class E is the headline.** Sixty of 148 leaf rows are `partial` for something that is not work:

- **Deliberate departures with the cost measured and written down** — §7.4.2 (skipping a stray hex
  byte rather than failing the stream), §7.5.5 (`/Size`, priced at 66 documents that would lose their
  page tree), §7.4.8 (`/ColorTransform` left unread because obeying it breaks the only file that
  exercises it), §10.4.2.5 (§10.3's ICC route outranks it), §8.6.5.7 (the declined `should`, measured
  at under 0.88 of 255), §10.7.5, §12.11.6 ("the `shall not continue` is not obeyed, by choice").
- **Rows whose own note says nothing is owed.** §8.6.4.4 ends *"Nothing is owed and the status does
  not move: `partial` still records the documented choice between the two answers §10.4.2.1 ranks."*
  A status whose definition is "some requirements are executed and some are not" is being used to
  record a choice between two the standard itself ranks. §12.5.6.3 — *"Nothing is owed for
  rendering"*. §11.4.8 — a clause that "states no requirement of its own".
- **Entries with no in-scope consumer**, four rows for one reason: an object-level `/Metadata` packet
  is interchange and no clause in scope consumes one (§7.7.3.3, §8.9.5.1, §14.6, §14.6.2). §7.7.3.3's
  seven values and §7.7.4's six name trees are owed to features `CLAUDE.md` excludes by name or that
  PDF 2.0 deprecates.
- **Debts that belong to somebody else** — §14.11.6.2 (all three entries writer-side), §9.8.3.3
  (enforcement is a validator's job), §12.8.4.2 (the `shall`s are addressed to a validator).

None of this is dishonest; every one of those rows argues its case well. But **60 rows are sitting in a
status that means "owed" while saying "not owed"**, and a round allocated to `partial` rows will
spend its reading on them and produce a correction to a note.

### 2.3 One debt, many rows

The 148 are fewer debts than rows. Counted by grepping the notes for the blocker each names:

| debt | `partial` rows it holds |
|---|---:|
| trust store and revocation ("question 3") | **12** |
| compositing in a group colour space | 7 |
| optional content `/Configs`, `User`/`Language`, `Zoom` | 7 |
| four ECDSA/EdDSA curves (brainpool ×3, Ed448) | 5 |
| Reference XObjects (Table 95) | 5 |
| a shape channel beside alpha | 4 |
| reconstructing the signed revision | 4 |
| the `/AIS` both-readings refusal | 4 |
| object-level `/Metadata` with no consumer | 4 |

**§12.8 alone is 22 `partial` rows and about five debts** — a trust store, revision reconstruction,
four curves, a deliberate DER tolerance and three PAdES attribute OIDs whose definitions this tree
does not hold. §11.* is 27 rows over perhaps eight constructions. §8.11 is 7 rows over three small
entries.

So the honest shape of the 206 is: **~58 parents + ~60 non-gaps + ~88 rows over roughly 45 distinct
pieces of work**, of which 31 are cheap.

### 2.4 What is genuinely blocked, and on what

**Blocked on a text (4).** Each names the document:

- §8.6.5.9 — **ISO 18619** defines what black point compensation *is*; the tree implements ADR 0012's
  stretch instead. Purchasable from ISO; the project already buys sponsored copies.
- §7.4.9 — **ISO/IEC 15444-2** defines the JPX *baseline* feature set the clause restricts a PDF
  codestream to. `doc/` holds 15444-1 (2016 and 2019) and not part 2. Purchasable. `doc/questions/Q51`
  already asks.
- §12.10.2 — the **EPSG registry** and **ISO 19162**'s WKT grammar, to turn a projected `/GCS`'s
  eastings and northings into degrees. ISO 19162 is purchasable; the EPSG registry is free and large.
- §12.8.3.4.3 — `content-reference`, `content-identifier` and `content-hints` are defined outside the
  documents held.

**One of these is not blocked at all.** §12.8.3's note says Errata Collection 3 issue #404 defers
SHAKE256's output length to *"the applicable stipulations on algorithm identifiers in RFC 8702, 3.1
and RFC 8419, 3.1, 3.2"* and calls them "two documents this tree does not hold". **RFCs are free**, and
`doc/memory` records that this shell has verified DNS/TLS/HTTPS since 2026-09-03. That is a
five-minute fetch standing as a blocker.

**Blocked on a capability (52).** The large ones, with a rough size:

| capability | rows | size |
|---|---:|---|
| a trust store and a revocation path | 12 | large, and it collides with principle 3 — the renderer has no network. Needs an owner decision about *where* it runs before a line is written |
| a shape channel carried beside alpha, on three backends | 4 | large |
| the knockout own-backdrop construction, three backends | ~4 | large; ADR 0307 has the arithmetic, ADR 1009 narrowed it once |
| compositing a painted group in its own colour space | 7 | large; needs press-to-press conversion the tree does not have |
| reconstructing a signed revision (`Document` holds one view) | 4 | medium-large, and it touches the immutability `CLAUDE.md` protects |
| Reference XObjects: find the target by `/F` and `/ID` | 5 | medium; and no document on this disk states one |
| four ECDSA/EdDSA curves | 5 | **small, and re-measurable** — the row says the blocker is "a package's release state". Worth re-checking each quarter rather than reading |
| host surfaces: a measuring tool, a URL opener, a collection presentation chooser, a printer's-mark panel, an article reader | ~8 | each is UI work, not clause work |

### 2.5 The closable list, cheapest first

These are the 31 class-A rows ranked by what they cost. The first four are hours, not rounds; the
first ten together are perhaps three rounds and close **fifteen** ledger rows.

| # | clause(s) | what to do | closes | cost |
|---:|---|---|---:|---|
| 1 | **§8.4.5** | Read Table 57's `/FL` in `content/ext_gstate.rs` and discard it exactly as the `i` operator's value is discarded. The row's own words: *"at a cost of nothing, because the parameter is discarded either way."* | 1 | **~30 min** |
| 2 | **§8.7.3.1**, §8.7.3 | Read Table 74's `/TilingType`. Values 1 and 3 ask for a device-grid adjustment this renderer does not make; value 2 is what it already does. Report the departure on 1 and 3. §8.7.3's note says it is `partial` for this and nothing else. | **2** | ~1 h |
| 3 | **§9.9.1** | *"If Length3 is 0 … the 512 zeros and cleartomark … shall be added by the PDF processor."* Append them. The row records that this changes no outline, so a byte-count unit test is the whole witness — a stated requirement executed. | 1 | ~1 h |
| 4 | **§9.8.3.1**, §9.8.3 | Read Table 122's `/Lang` (BCP 47) and pass it into `substitute.rs`'s face comparison as a hint. §9.8.3's note: *"`partial` for `/Lang` alone."* Two non-embedded CIDFont witnesses already exist (`noembed-eucjp.pdf`, `noembed-sjis.pdf`). | **2** | ~½ round |
| 5 | **§11.7.5.3 + §8.6.5.8 + §8.9.5.1** | One round, three rows: select the profile's `A2B0` for `Perceptual` and `A2B2` for `Saturation` instead of always `A2B1`; read the `/RI` and `/UseBlackPtComp` in force at the `Do` in `colour::sample_press`; and stop passing black point compensation as a literal `true` at `image.rs`'s six `Compositing::paint` calls. §11.7.5.3 says outright *"the moment it would be read at is already the right one"*; §8.9.5.1 says `/Intent` *"is the one that can move a pixel"*. | **3** | 1 round |
| 6 | **§14.8.3.3 + §14.8.5.4.5** | Derive the allocation rectangle: the content rectangle adjusted by `/SpaceBefore` and `/SpaceAfter`. Both rows name this one quantity and nothing else; the content rectangle is already computed by `content/marked.rs`. | **2** | ~½ round |
| 7 | **§14.8.5.8 + §14.8.2.2.2** | Read Table 385's `/Type` and `/Subtype` on an `Artifact` structure element (including PDF 2.0's `Inline`) and route them to the consumer `Artifact::read` already gives the marked-content form. §14.8.2.2.2's "third form" is this same element type. | **2** | ~½ round |
| 8 | **§12.5.6.7 + §12.5.6.9** | Read `/IT` and `/Measure` on line, polygon and polyline annotations. `measurement.rs` already reads a `/Measure`; the tree's only reader of `"IT"` today is §12.5.6.6's callout intent. (§12.5.6.9's `/Path` curves are separate and stay open.) | 0–2 | ~½ round |
| 9 | **§14.12.4.1 + §14.13.8** | Enumerate the DPart tree from Table 408's `/DPartRoot` rather than only from a `GoToDp`'s jump. Both rows are `partial` for exactly this walk. No corpus witness — build one (trap 8). | **2** | ~1 round |
| 10 | **§7.6.6** | `/AuthEvent` is *"read only far enough to know it does not gate the body."* Read it and act on it, or report the values this handler does not honour. | 1 | ~½ round |

Then, roughly in increasing cost: §8.6.5.5 (`/Range` plus the ICC `'Lab '` profile class — a real
correctness gap, since such a space silently falls to `DeviceRGB` today); §12.5.2 (`/P`, `/M`, `/AF`,
`/Lang`, three of which have readers elsewhere in the tree); §8.7.4.5.7 + §8.7.4.5.8 (derive
`PATCH_STEPS` from §10.7.3's smoothness tolerance, which `/SM` already supplies to
`Ramp::resolution_for` — **2 rows, one change**); §7.10.2 (`/Order 3`'s cubic spline); §12.7.5.4 (draw
which option a list box's `/V` names); §14.7.4.2 + §14.8.5.3 (the `NSO` owner priority, a paired
decision); §12.5.6.22 (the media-box origin translation); §12.3.5.2 (a folder's `/CreationDate` and
`/ModDate`); §14.8.2.3 (fold U+00AD in the product, not only in the harness); §8.11.4.5 (reapply on a
zoom change — `ViewState` has carried the magnification since ADR 0168); §9.7.5.4 (`beginbfchar` in an
`Encoding` CMap — a decision plus small code, and `bug920426.pdf` is the witness); §12.5.6.2
(`/Contents` as §14.9.3's alternate description); §8.5.3.3.1 (the degenerate subpath's single device
pixel — cheap, and the clause itself calls the result *"not generally useful"*).

**Estimate: the first ten items are about three rounds and close fifteen rows. All 31 class-A rows are
perhaps ten to twelve rounds.** That is a real answer to "is the allocation sound" — it is, for about
twelve rounds, and then it runs into class C.

### 2.6 The one thing to do before the allocation runs

**Re-status the sixty class-E rows.** They are not work, they are notes, and they are the reason a
round aimed at `partial` rows produces a prose correction instead of code. Whether the right target is
`implemented`-with-a-documented-choice, `inapplicable`, or a new status is the owner's call — but the
ledger currently uses one word for "we owe this" and "we decided this", which is the exact vocabulary
collapse the file's own header says the statuses exist to prevent.

---

## 3. The gates: the cheapest tenth catches almost everything

`doc/todo/02` §2 is **39 command lines** — 35 test/lint gates and four `--bins` builds that exist only
to satisfy trap 10. Over sessions 912–1011 (98 history files), classified into (a) caught a defect
*this round* introduced, (b) passed, (c) failed on a sibling's mid-edit work, contention or population
drift:

| tier | gates | (a) self-catches | (c) false alarms | cost |
|---|---|---:|---:|---|
| **the core** | `fmt`, both `clippy` lines, `nextest`, `cargo test -p conformance` | **17** | many, mostly `fmt`/`clippy` on siblings | "about a tenth of the sequence's cost"; conformance alone "costs seconds" |
| everything else | 34 lines | **1** | 14 | ~25 minutes |

`cargo test -p conformance` alone accounts for **7 of ~18 self-catches** and costs seconds
(sessions 912, 924, 940, 977, 989, 997, 1005). Session 924's own record already drew the conclusion:
*"`cargo test -p conformance` costs seconds and is the last line of a sequence that costs half an
hour."*

**25 of the 39 lines produced no recorded self-catch in 98 sessions.** Six of them (`save_round_trip`,
`actions`, `on_disk`, `pdf-archive corpus`, `archive_corpus`, `awkward_classes`) only entered at
session 991/995 and have 16 rounds of exposure, and `raster_golden` entered at 996 — so they are
unproven rather than disproven. That leaves **eighteen lines with 98 sessions of exposure and nothing
to show.**

The worst offender is measurable by name: **`pdf-transform --test foreign_corpus` — zero self-catches,
five false failures (sessions 914, 926, 939, 960, 999), 76–214 s, and 6.70 GiB**, the heaviest memory
consumer in the sequence. Session 999's record says it *"cannot be run beside another round's copy of
itself, and the failure looks like a defect."* On a machine running five rounds, that is a gate whose
dominant output is a false alarm.

The one genuine catch outside the core is worth keeping and worth naming: **`viewer-ui --test
launch_path`, session 991** — `open_kinstructions` 26,773.8 against a band top of 26,760, bisected to
round 990's lexer salvage losing a `String` and becoming small enough to inline. A 0.04% regression
caught by a counted instrument. That is principle 2 working. It also has **six** (c) failures from
driver-allocation drift.

### Proposed tiering

**Tier 1 — every round, before anything else (~3 minutes).** `fmt`, both `clippy` lines,
`nextest`, `cargo test --workspace --doc`, `cargo test -p conformance`. These are where 17 of 18
self-catches came from.

**Tier 2 — every round, cheap and load-bearing (~2 minutes).** `pdf-model --test corpus` (10–17 s),
`dates` (0.9 s), `actions` (1 s), `on_disk` (2 s), `pdf-archive --test corpus` (2 s), `pdf-transform
--test gate` (3 s), `archive_corpus` (6 s), `save_round_trip` (12 s), `awkward_classes` (12 s — a
death fails it, which is a security property and not a defect count), `jpeg2000` (7.8 s), `xmp`.
None of these has caught its own round's defect, but each costs seconds and several are too young to
judge.

**Tier 3 — the round that touched the subsystem, plus the merge (~25 minutes).** `oracle`,
`raster_golden`, `text_extraction`, both `viewer-core` censuses, `render-raster --test corpus`,
`fixed_documents`, all six `pdf-transform` corpus walks, both `pdf-vfs` walks, `launch_path`. **The
merge runs the whole of tier 3 once**, which is where the ten converter-fixture failures of session
1005 and the `archive_corpus` signature defect of session 985 were both actually found — at a merge,
not in a round.

**Saving: roughly 25 minutes per round, kept once per batch instead of once per round.** With five
rounds to a batch that is about two hours of machine time per batch, and — more valuable — it removes
the fourteen false alarms that each cost a round a diagnosis.

**One line to delete outright rather than demote:** the four trap-10 `--bins` builds are not gates,
they are prerequisites of the walks below them. They belong inside whatever runs those walks.

---

## 4. The instruments: seventeen of twenty have not run in sixty sessions

`tools/conformance` has **20 binaries** (all auto-discovered; there are no `[[bin]]` entries) and
22,042 lines. In the last 60 sessions **three** were run: `quotations` (7 runs), `pointers` (5),
`ledger` (1). The other seventeen are dormant, and `--bin callers` was last mentioned at session
**525** — 487 sessions ago.

Most of the dormant seventeen are not failures. `owed`, `overstated`, `unread`, `entries`, `tables`,
`counts`, `retired`, `inapplicable`, `undenominated`, `permitted`, `unpriced`, `overtaken`, `quoted`
each found real defects when they were written and each is a **sweep**, not a gate: it is run once over
the whole tree, everything it finds is fixed, and it then finds nothing until the tree drifts again.
That is the correct life-cycle for a sweep. What is missing is a **cadence** — they are not scheduled,
so they are run when a round remembers.

Two have never produced an acted-on finding *as programs*:

- **`--bin capabilities`** — `doc/todo/01:2781` records its run: *"164 mentions, 4 nouns carrying both
  shapes, 0 defects."* Its one real catch (sessions 201, 220) predates it being a program.
- **`--bin callers`** — every catch attributed to it (§12.5.6.19's `/H`, §8.11.4.3's `/ListMode`,
  `must_cover_whole_file`) is from sessions 253/254/278, before it was a program.

### The two standing figures

**`--bin pointers`' "185 absent" is noise, and the number has never gone down.** Trajectory: 104
(s537) → 118 (s645–671) → 185 (s967) → 193 (s998) → 211 (s1005) → 221 (ADR 1023) → **226** (s1010).
Monotone increasing across 470 sessions. ADR 0372 said why on the first run: *"Of the 104 absent, 84
are in `doc/adr/` and in `doc/todo/01`'s own records of earlier runs … and is **not** a defect"* —
history files are never edited and ADRs are not edited to follow a moved file. ADR 0993 is blunter:
renaming a `doc/todo/` file *"turns live pointers into absent ones in files nothing may ever repair,
and `--bin pointers` would carry them for the life of the project."* Rounds now quote it as "185
absent before and 185 after" — **a delta detector wearing an absolute number.** Proposal: partition
the output into repairable roots (`crates/`, `tools/`, `doc/todo/`, `doc/traps/`, `doc/habits/`,
`CLAUDE.md`, `doc/PLAN.md`) and unrepairable records (`doc/adr/`, `doc/history/`), and print the first
as the finding. The second belongs in a footnote or nowhere.

**`--bin quotations`' "49 diverging" stood unread for about 40 sessions and is ~60% false positive by
its own sample.** Trajectory: 22/40 (s501) → 1 (s553–667) → 38 (s741–833, stable across 90 sessions)
→ 49 (s967) → 51 (s1010). It never fell. Session 1010 (ADR 1029 §6) is the first round to read any of
them, and read five: **2 were real defects and fixed**, 2 are false by construction (a correction
quoting the sentence it retired; `doc/errata-read.md` quoting Errata Collection 3 replacement text
against unamended `doc/md/`), and **1 is an instrument bug left unfixed** — §11.3.6, where *"its
matcher takes the first candidate rather than the best, which is a finding about the instrument."*

The *ledger* half of the same sweep is healthy and did move under action: 5 → 3 diverging at session
1010. **Proposal: fix the matcher, exempt the two false-by-construction shapes, and then the residue
is small enough to read to zero in one round.** Until then the 49 is a number rounds copy forward.

### The denominator defect, and why the figures before session 1010 are wrong

`tools/conformance/src/roots.rs` records that `SOURCE_ROOTS` was a hand-written `["crates", "tools",
"fuzz"]` sitting beside a glob manifest, so **1,884 clause citations in 243 of 283 Rust files sat
outside the citation and quotation gate for four months, producing no findings because nothing read
them.** Session 1004 found it, session 1010 fixed it and added
`every_workspace_member_is_scanned`. This is the same shape as session 1004's fuzz target that had
never been runnable and ADR 1024's `clippy.toml` threshold configuring a nursery lint no manifest
enabled — *"an instrument that had never run once."*

**That is three instances in one session of the same defect: an instrument whose population was
smaller than it claimed.** It is worth one round to check the remaining denominators — every sweep's
root list, every census's corpus, every gate's `#[ignore]` set — against a manifest rather than a
literal.

### One command in the documentation is wrong

`doc/verify.md:160` says `cargo run --release -p viewer-gtk --example outline_census`. The example is
at `crates/viewer-host/examples/outline_census.rs`; `crates/viewer-gtk/examples/` does not exist. Every
other `-p X --example Y` pairing in `doc/verify.md` was validated against the on-disk package names and
this is the only mismatch. `outline_census` also has zero history mentions.

---

## 5. What a round pays before it starts

The preamble corpus — `CLAUDE.md`, `HANDOVER.md`, `todo/README.md`, `todo/02`, `environment.md`, the
six habit files, the five trap files, `state-of-play.md`, `PLAN.md`, `history.md`,
`todo/00-ambiguous-bucket.md` — is **9,847 lines / 170,948 words** at `6c6a1d3d`.

What a round actually must read is smaller: HANDOVER's "every round, whatever it is about" table is
`CLAUDE.md` + `HANDOVER.md` + `todo/README.md` + `todo/02` + `environment.md` = **1,953 lines /
26,940 words**, plus one trap group (186–917 lines) and typically one habit file. A realistic floor is
**~2,150–2,900 lines**; a thorough round reaches ~5,000.

**It is still growing, and neither compaction reversed it.**

| point | lines | words |
|---|---:|---:|
| session 880 | 8,544 | 154,251 |
| session 960 | 9,476 | 169,167 |
| **before ADR 0974** | 9,476 | 169,167 |
| **after ADR 0974** (s964) | 9,465 | 165,532 |
| session 990 | 9,781 | 169,189 |
| **before ADR 1023** | 9,839 | 170,416 |
| **after ADR 1023** (s1000) | 9,838 | 170,810 |
| now (`6c6a1d3d`) | 9,847 | 170,948 |

**+1,303 lines (+15.3%) over 131 sessions**, ~10 lines a session, monotone apart from two one-line
dips. ADR 0974 is **−11 lines** (it did remove 3,635 words, so it was a prose-density compaction);
ADR 1023 is **−1 line and +394 words** — a rewrite of `doc/PLAN.md`, not a reduction. Both were
recouped within a handful of sessions. ADR 0983's habits split took `habits.md` from 1,041 to 45 lines
and produced six files totalling **1,334** — the habits half grew **+338 lines while presenting itself
as a compaction**.

### What a round demonstrably uses

Trap citations across 563 history files and 880 ADRs, with "recent" = sessions 912–1011:

| trap file | traps | recent citing rounds | last cited |
|---|---:|---:|---:|
| `instruments-and-reports.md` | 23 | **32** | 1009 |
| `pixels-and-rasterisers.md` | 7 | 7 | 1008 |
| `oracle-and-references.md` | 4 | 5 | 974 |
| `parsers-and-streams.md` | 5 | 3 | 944 |
| **`the-interactive-loop.md`** | 6 | **0** | **798** |

Five traps (1, 5, 8, 11, 13) carry the large majority of all citations ever; trap 13 alone appears in
120 history files and 142 ADRs. **Trap 12c and trap 38 have never been cited anywhere** outside their
own file and the index — and 12c's only two other appearances are audits noticing it was *missing from
the index*. Trap 38 is new (~session 960), so that is unproven rather than stale.

Habit files, created at session 969:

| habit file | recent citing rounds | last cited |
|---|---:|---:|
| `measuring.md` | 2 | 922 |
| `the-ledger-and-claims-about-this-tree.md` | 2 | 985 |
| `reading-the-specification.md` | 2 | 1005 |
| `tests-gates-and-reports.md` | 2 | 985 |
| **`code-bounds-and-dependencies.md`** | **0** | **never** |
| **`judging-against-other-implementations.md`** | **0** | **never** |

`tools/round.sh` puts `judging-against-other-implementations.md` on the reading list for an oracle
round, and **no oracle round has ever cited it back**.

**467 lines — `the-interactive-loop.md` plus those two habit files — that no round has drawn on in 100
sessions.** That is not a lot of reading, but it is the visible part of a pattern: the corpus grows
monotonically and nothing is ever retired from it, which is the same shape as the gates and the
instruments.

---

## 6. The ADR and history convention

**The ADR convention earns its keep. The history convention mostly does not, and it is the cheaper of
the two.**

`doc/adr/` is 874 files / 126,006 lines (mean 144). `doc/history/` is 558 files / 58,923 lines (mean
106, and 91 over the last 40). Together **184,929 lines of bookkeeping against 540,724 lines of `.rs`
under `crates/` + `tools/` — about 34% of the source tree by line count.**

Of the 52 ADRs in the window, classified by whether anything cites them:

- **26 record a decision a later round needed** — cited from `crates/`/`tools/`, or amended by a later
  ADR, or pointed at from `doc/todo`/`doc/traps`/`CLAUDE.md`. ADR 1022 has 12 code citations, 1008 has
  9, 0971 has 10.
- **25 are instrument or process** — and these are *not* low-value: 1016 has 8 code citations, 0970
  has 6, 1015 has 4 tool citations. A process ADR that a gate cites is load-bearing.
- **1 is pure narration**: ADR 0979, "A capability floor that ran on one machine", with zero citations
  anywhere outside its own file and `doc/history/`.

**A 2% narration rate on a 52-ADR window is not a convention in trouble.** The real weakness is
**citation discipline, not volume**: ADRs 0957 and 1019 argued decisions that shipped into
`crates/pdf-transform/src/archive/` (`Answer::AsUnderlying`, `apply`) and the code does not cite the
ADR that argued them. Eleven of the 52 have no code citation at all. That is fixable by a lint —
`--bin pointers` is already the right instrument — not by writing fewer ADRs.

The price: **~238 lines / ~2,584 words per session**, one ADR (156 lines) and one history file (82).
**20 of the 51 history files carry the same slug as an ADR from the same round** — two files, one
title, one session.

`CLAUDE.md`'s claim that no round reads `doc/history/` is accurate and enforced in code:
`retired.rs:163` has `NOT_SWEPT = "doc/history"` with a test; `tools/governing-quotations.py:74`
exempts it; `round.sh:119` reads only the filename digits. **One exception, and it is a live cost:**
`doc/history/` is *not* in `prose::NOT_READ` (`tools/conformance/src/prose.rs:73`), so
`--bin quotations` walks all 558 history files on every run and checks **131 blockquote lines and 338
long quoted spans** in files no round may edit. That is a permanently growing, permanently
uncorrectable quotation population inside one of the two instruments still being run — and it is part
of why the 49 never falls.

---

## 7. What to stop doing, ranked

Each with the measurement behind it and what it saves.

1. **Stop running tier 3 of `doc/todo/02` §2 in every round; run it in the round that touched the
   subsystem, and in full at the merge.** *Measurement:* 25 of 39 lines, ~25 minutes, produced 1
   self-caught defect and 14 false alarms in 98 sessions; 17 of ~18 self-catches came from the five
   cheapest lines. *Saves:* ~25 min of machine time and, more importantly, the diagnosis cost of a
   false alarm per round. **Start with `foreign_corpus`** — 0 self-catches, 5 false failures, 214 s,
   6.70 GiB, and a record saying it cannot run beside a sibling's copy of itself.

2. **Stop treating the 206 as 206.** *Measurement:* 58 aggregate parents cannot be worked on; 60 of
   148 leaves are not gaps; §12.8's 22 rows are five debts. *Saves:* about 40% of the round-slots the
   new allocation would otherwise spend on prose corrections. **Re-status the 60 class-E rows first**
   — one round of ledger work that makes the next twelve rounds hit code.

3. **Stop copying `--bin pointers`' and `--bin quotations`' absolute figures forward.** *Measurement:*
   `pointers` has risen monotonically 104 → 226 over 470 sessions and ADR 0372 recorded on the first
   run that most of it is unrepairable by construction; `quotations`' 49 was unread for 40 sessions and
   3 of the first 5 read were noise, one of them an unfixed matcher bug. *Saves:* two standing numbers
   that currently cost a paragraph a round and buy nothing. **Partition `pointers` by repairable root;
   fix `quotations`' matcher and exempt `doc/history/` via `prose::NOT_READ`, then read the residue to
   zero once.**

4. **Stop writing a history file whose slug duplicates the round's ADR.** *Measurement:* 20 of 51 do;
   nothing reads `doc/history/` (three tools name it to skip it, `round.sh` reads only the digits);
   82 lines and ~850 words a session. *Saves:* ~850 words a round. **Keep the history file where it
   archives prose deleted from a navigational document** — `doc/history/1003` is a real use of the
   "a move is not an edit" rule — and where the round has no ADR.

5. **Stop carrying instruments with no cadence.** *Measurement:* 17 of 20 conformance binaries have not
   run in 60 sessions; `--bin callers` last in session 525; `--bin capabilities` has produced 0 defects
   as a program. *Saves:* not machine time — **attention**. **Put the fourteen dormant sweeps on a
   stated cadence (one per merge, rotating) and delete `callers` and `capabilities`**, whose catches
   all predate their being programs.

6. **Stop leaving retired reading in the preamble.** *Measurement:* `doc/traps/the-interactive-loop.md`
   last cited at session 798 (213 sessions ago); `habits/code-bounds-and-dependencies.md` and
   `habits/judging-against-other-implementations.md` never cited, the second of them on `round.sh`'s
   oracle reading list. 467 lines. *Saves:* small directly; large as precedent — **the corpus has grown
   15% in 131 sessions and neither named compaction removed a line.** A compaction that does not reduce
   the line count is a rewrite.

7. **Stop reading "this project does not hold that document" as permanent.** *Measurement:* §12.8.3
   names RFC 8702 and RFC 8419 as documents not held; RFCs are free and the shell has verified network
   access. Three of the other four class-B blockers name purchasable ISO texts the project already buys
   sponsored copies of. *Saves:* four rows' worth of permanent blockage for one afternoon of fetching
   and one purchase decision.

8. **Stop deferring the denominator audit.** *Measurement:* session 1004 found three instruments in one
   session whose population was smaller than claimed — a fuzz target never runnable, a `clippy.toml`
   threshold for a lint no manifest enabled, and `SOURCE_ROOTS` excluding 243 of 283 files for four
   months. *Saves:* unknown, and that is the point — **every figure produced before session 1010 by the
   citation and quotation gates was measured over an incomplete tree.**

---

## 8. What is fine as it is

The owner asked for an honest read, so:

- **The ADR convention is fine.** One narration ADR in 52, 30 of 52 cited from compiled code. Do not
  write fewer; fix the citation discipline.
- **The core five gate lines are fine and should never be demoted.** `cargo test -p conformance` costs
  seconds and catches more than the other 34 lines combined.
- **The sweeps are fine as sweeps.** `owed`, `overstated`, `unread`, `entries`, `retired` and the rest
  each found real defects and then correctly found nothing. They need a cadence, not deletion.
- **The deliberate departures are fine and well argued.** §7.5.5's `/Size`, §7.4.8's `/ColorTransform`,
  §10.4.2.5, §8.6.5.7 — each names its cost in documents or in levels of 255. The problem is the
  *status they wear*, not the decisions.
- **`launch_path` is fine and is the best gate outside the core.** It caught a 0.04% instruction-count
  regression in session 991 and principle 2 is why that matters.
- **Trap and habit files that are used are heavily used.** Trap 13 appears in 120 history files. The
  instruction corpus is not bloated everywhere; it is bloated in the parts nobody cites.
- **`doc/history/1003`'s shape is fine** — a history file that parks prose deleted from four
  navigational documents is the "a move is not an edit" rule doing real work.

---

## 9. Numbers a walk would be needed for

Not run, per the constraint. Named so a later round can produce them:

- **Which tier-3 gates would have caught the defects merges found**, rather than which caught their own
  round's: needs the six `pdf-transform` corpus walks and `pdf-archive`'s replayed over the merge
  commits of sessions 985, 991, 1005.
- **Whether the 31 class-A rows have corpus witnesses**: `cargo run --release -p pdf-model --example
  absence_audit --crawl` for `/Lang`, `/FL`, `/TilingType`, `/Order`, `/AuthEvent` and Table 385's
  `/Type`. Item 1's answer is already known to be zero and does not need one.
- **The true cost of tier 3 on an idle machine**: every figure in §3's cost column was recorded under
  sibling load and the ranges are wide (`merge_corpus` 53–201 s, `read_corpus` 278–1504 s).
