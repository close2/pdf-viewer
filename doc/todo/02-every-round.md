# What every round does

Status: **standing** — this one is never done.
Priority: 02

A "round" here is one session's worth of work. `CLAUDE.md`'s two tracks decide *what* it
contains; this file is what it does around that, in order.

**This file states commands and rules, never counts.** `tools/state.sh` prints the counts, and
that separation is the point: a round that can read a gate's number here can write it down
without running the gate. Every figure this file used to carry went stale at least once (ADR
0281).

## 0. The round, on one page — what every round reads of this file

The rest of this file is the argument and the lookup; this section is the contract, and a round
reads the section below it only where a line here sends it (ADR 1639).

1. **Read, in this order, and nothing else up front**: `CLAUDE.md`; this section; the rule block
   that opens `doc/environment.md`; `doc/traps/every-round.md`; `doc/habits/every-round.md`; then
   what `tools/round.sh <kind>` names for the kind of round you are, and what your contract names.
   `doc/HANDOVER.md` is the index to open when you need a file this list did not give you.
2. **The contract is ledger rows by number, or a named build.** Read the clause in `doc/md/`,
   never only the row's note; check the brief's premise in the text, the code and the data before
   building on it, test its hypothesis first (ADR 1748), and say in the report which of each
   held. Quotation marks mean verbatim from ISO 32000-2; every other text is cited by section and
   paraphrased; a `§` is ISO 32000-2's.
3. **Tier 1 is every round's, scoped to what you touched**: `rustfmt --check --edition 2024` on
   your files; `RUSTFLAGS="-D warnings" cargo clippy -p <crate> --all-targets` and
   `cargo nextest run -p <crate>` for each crate you touched; `cargo test -p conformance`. Tier 2
   only for the subsystem the change reaches, by section 2's rule list and map, behind the lock,
   each run declaring its kind: `--tree 6` for a walk under 6 GiB, `--clock` for one whose verdict
   is a time (ADR 1684), `--long` for a campaign or a census whose length is its own choice, which
   holds the second lane only (ADR 1756). Walks are granted in the order they asked, and a run
   stopped while it queues still leaves its line, `lane=-` (ADR 1790).
   Tier 3 is the merge's. **A figure a gate already holds is read from the gate's band, ratchet or
   held list, not re-measured**; a round re-measures what its change can move.
4. **Paperwork is one new record** `doc/history/<session>-<slug>.md` of at most forty lines with a
   `**Gates.**` paragraph stating each gate's exit status or count (ADR 1499) — `wc -l` it; an ADR
   only for a decision a later round must not re-litigate, numbered as assigned; a `Q` file only
   where the owner's word is needed, with a recommendation and what the tree does meanwhile;
   `doc/state-of-play.md`, the todo file touched and `doc/todo/65` kept true by small edits. No
   trap, no habit, no edit to any other round's file; a proposed trap goes in the report.
5. **Ledger edits are targeted and your own rows' only**: re-read the file immediately before a
   `str.replace` of your row's text, only `\\ \" \n \t` escapes, `cargo test -p conformance`
   right after; never `tools/state.sh ledger` or `--bin ledger` mid-batch; `over` and
   `Unconsidered` stay 0; a status change edits `doc/todo/65`'s map in the same pass.
6. **The report** names rows moved with the clause sentence, every file touched, every gate with
   its exit status, what is unfinished and why, and the premise that did not hold.

## 1. Take from both tracks

Demand-driven is what the corpus and the oracle name (todos `10`–`29`); spec-driven is the
ledger's `reported` rows and the notes on its `partial` ones (todos `00`–`09`). A project
running only the first finishes when the corpus goes quiet, which can happen with much of the
standard unimplemented and nothing able to say which parts; one running only the second ships
features no file exercises. This is a principle-5 rule, not a suggestion.

**But the map is not the territory.** Four of six findings in one run of ten rounds were on
no list at all: a `shall` hiding behind a silence about artwork (ADR
0109), a clause with two populations where the row named one (0110), a malformed optional entry
that erased a font (0111), and a font cache keyed by a name that drew wrong glyphs in silence for
thirty-one sessions (0115). None was `silent`, none was `reported`, and no gate could see the
last. Three were found by reading the clause beside the code; the fourth by measuring something
else.

**The six shapes a refusal takes when it has outlived its reason** — a reason that names a
vocabulary, a reason that names an architecture, a capability that arrived and announced nothing,
a capability that reached the crate and never reached the program, a row that would have
survived the capability arriving, and a row *corrected* by naming the capability that arrived while
the entry it turns on stayed unread — are in
[`doc/habits/the-ledger-and-claims-about-this-tree.md`](../habits/the-ledger-and-claims-about-this-tree.md),
beside the sweeps that find each. They are the highest-yield reading this project has.

## 2. Run the gates that can see what you touched

**The sequence is in three tiers, and the map after it says which of tier 2 a given change needs**
— a tier first, a rule second, a lookup third, and most rounds never reach the lookup. **Its rules
3, 4 and 5 are the whole of the map's safety** and none of them is relaxed by anything here.
`tools/round.sh` says whether this is a fifth round.

**Why there are tiers at all, in one sentence per tier** (ADR 1036, on the measurement in
`doc/reviews/1012-where-the-effort-goes.md` §3):

- **Tier 1 runs in every round because it is where the catching happens** — seventeen of the
  eighteen defects a round caught in its own work came off these lines, `cargo test -p conformance`
  alone accounting for seven of them, and the whole tier is about a tenth of the sequence's cost.
- **Tier 2 runs in the round that touched the subsystem, because a gate can only see what its own
  crate does** — these are the lines the change → gate map names, and a round that did not touch
  what they walk is paying for a walk that cannot move.
- **Tier 3 runs at the merge and not in an ordinary round, because that is where its catches
  actually happened** — one merge found ten converter-fixture failures and another the
  `archive_corpus` signature defect; in 98 sessions these lines caught
  **one** defect in the round that introduced it and raised **fourteen** false alarms, at about
  twenty-five minutes a round. Nothing here is deleted or weakened: the merge runs every one of
  them on `main`, once per batch instead of once per round, and rule 5 is why that is not a hole.

**Tier 1 — every round, whatever it touched.**

```sh
cargo fmt --all --check
RUSTFLAGS="-D warnings" cargo clippy --workspace --all-targets   # `RUSTFLAGS` is not optional
cargo nextest run --workspace
cargo test --workspace --doc                # the one doctest nextest does not run
cargo fmt --manifest-path fuzz/Cargo.toml --check                            # `fuzz/` is not a workspace member
RUSTFLAGS="-D warnings" cargo clippy --manifest-path fuzz/Cargo.toml --all-targets   # nor is it here
cargo test -p conformance -- --nocapture    # seconds, and it reads the citations and quotations out of the tree
```

**Tier 2 — the round that touched the subsystem.** The map below says which of these this round
owes; a round that owes none of them runs tier 1 and stops. **Every line is a walk**, and runs as
its `tools/state.sh --round <session> <section>` runs it: under the wrapper, in the lane its section
declares, with the sandbox worker it spawns built for its own profile as the wrapper's `--build`
inside the hold — Cargo does not build another package's binary for a test (trap 10), and a build
before the lock is as old as the moment the walk stopped queueing (trap 109, ADR 1710). The block's
first line is that shape; the lines after it are what goes after its `--`.

```sh
ulimit -u 8192; tools/bounded.sh --lock --round <session> --tree <6|12> --build '--profile gates -p pdf-sandbox --bins' (or `--release`, as the line's profile is) -- <the line>
cargo test  --profile gates -p pdf-model      --test corpus          -- --ignored --nocapture
cargo test  --profile gates -p pdf-model      --test raster_golden   -- --ignored --nocapture   # ADR 1016: our own output held by name — a change detector; PDFVIEWER_RASTER_GOLDEN=update regenerates
cargo test  --profile gates -p pdf-model      --test script_corpus   -- --ignored --nocapture   # RFC 0008 section 6.7's Tier 0 form: every field script of the census population committed once, every displayed value held by name (ADR 1579); PDFVIEWER_SCRIPT_CORPUS=update regenerates
cargo test  --profile gates -p pdf-script --features engine --test script_corpus -- --ignored --nocapture   # RFC 0008 section 6.7's Tier 1 column: every script Tier 0 does not run, run in its document's realm and held to the column's own ceilings (ADR 1625); a walk, through tools/bounded.sh --data 8 --tree 12
cargo test  --profile gates -p pdf-model      --test dates           -- --ignored --nocapture
cargo test  --profile gates -p pdf-model      --test xmp             -- --ignored --nocapture
cargo test  --profile gates -p pdf-model      --test jpeg2000        -- --nocapture
cargo test  --profile gates -p pdf-transform  --test gate            -- --ignored --nocapture   # RFC 0002 section 12's floor
cargo test  --profile gates -p pdf-syntax     --test on_disk         -- --ignored --nocapture   # every corpus document read from disk and from memory, object for object (ADR 0809)
cargo test  --release       -p viewer-ui      --test launch_path    -- --ignored --nocapture   # principle 2's numbers, the counted half (doc/verify.md runs the clocks); `--release` on purpose, see below; a `--clock`, whose script stage's worker is a second `--build` beside the sandbox's (ADR 1620; doc/checks/launch-path.toml spells both)
cargo test  --release       -p render-raster  --test turn_path      -- --ignored --nocapture   # doc/performance.md 3e's turn and step rows, banded in doc/checks/turn-path.toml (ADR 1513)
tools/batch.sh raster-examples   # ci.yml's fourteen raster examples, each with --check under Xvfb, one line each (ADR 1575)
```

**Tier 3 — the merge, on `main`, and every line of it.** These are the corpus-scale walks. An
ordinary round does not run them; the round that merges a worktree into `main` runs all of them,
and so does every fifth round (rule 4) and any round whose own subject *is* one of these walks —
a change to `pdf-transform`'s writers, to `pdf-archive`'s validator or to `pdf-vfs` is a change to
what these lines assert, and the map says so. Each line runs as tier 2's do, its section's lane and
its worker's `--build` inside the hold.

```sh
cargo test  --profile gates -p pdf-model      --test oracle          -- --ignored --nocapture   # a `--clock`, whose second `--build` makes the reference program `pdfref-hayro` (trap 10 again, see below)
cargo test  --profile gates -p pdf-model      --test text_extraction -- --ignored --nocapture   # three gates
cargo test  --profile gates -p viewer-core    --test selection_census -- --ignored --nocapture
cargo test  --profile gates -p viewer-core    --test accessibility_census -- --ignored --nocapture
cargo test  --profile gates -p pdf-model      --test save_round_trip -- --ignored --nocapture   # §7.5.6's update over the corpus, read back by this tree, poppler and mupdf; its counts ratchet (ADR 1011)
cargo test  --profile gates -p pdf-model      --test actions         -- --ignored --nocapture   # §12.6.3's page-scoped triggers counted over the corpus and held both ways, and §12.6.4's embedded go-to opened
cargo test  --profile gates -p render-raster  --test corpus          -- --ignored --nocapture
cargo test  --profile gates -p pdf-model      --test fixed_documents -- --ignored --nocapture
cargo test  --profile gates -p pdf-transform  --test writer_corpus   -- --ignored --nocapture   # RFC 0002 section 9: the writer over the corpus
cargo test  --profile gates -p pdf-transform  --test split_corpus   -- --ignored --nocapture   # RFC 0002 section 9: split over the corpus, layers 2 and 3
cargo test  --profile gates -p pdf-transform  --test merge_corpus   -- --ignored --nocapture   # RFC 0002 section 9: merge over the corpus, layers 2 and 3
cargo test  --profile gates -p pdf-transform  --test pages_corpus   -- --ignored --nocapture   # RFC 0002 section 9: pages over the corpus, layers 2 and 3
cargo test  --profile gates -p pdf-transform  --test optimize_corpus -- --ignored --nocapture   # RFC 0002 section 9: optimize over the corpus, layers 2 and 3 and its idempotence gate
cargo test  --profile gates -p pdf-transform  --test foreign_corpus -- --ignored --nocapture   # RFC 0002 section 9: the five writers' output read by poppler, mupdf and qpdf
cargo test  --profile gates -p pdf-archive    --test corpus          -- --ignored --nocapture   # the validator against the veraPDF corpus, clause by clause per target; `over` is the column that matters (ADR 1015)
cargo test  --profile gates -p pdf-transform  --test archive_corpus  -- --ignored --nocapture   # the converter over the same corpus: conforms in, conforms out, no glyph moves — the walk that found ADR 1006's signature
cargo test  --profile gates -p pdf-archive    --test cross_check     -- --ignored --nocapture   # A61: the survey's resource selections held to the interpreter's, within the constructs the survey walks; a disagreement names document, page, stream, operator and name (ADR 1055)
cargo test  --profile gates -p pdf-vfs        --test write_corpus   -- --ignored --nocapture   # RFC 0003 section 5.2: the five write verbs over the corpus, through the core
cargo test  --profile gates -p pdf-vfs        --test read_corpus    -- --ignored --nocapture   # RFC 0003 section 4: the whole layout listed, stat'd and read through the confined worker, over doc/pdf.js whole and a class-balanced sample of every other corpus on the disk
cargo test  --profile gates -p viewer-confined --test awkward_classes -- --ignored --nocapture   # the other confined program over the same classes from every corpus on the disk; what fails it is a death (ADR 0879, ADR 1015)
```

**`foreign_corpus` is the line to reach for last and the one to suspect first.** Zero self-catches
in 98 sessions, five false failures, 76–214 s and 6.70 GiB — and one of those rounds' records
(`doc/history/999-…`) says it *"cannot be run beside another round's copy of itself, and the failure
looks like a defect."* On a machine running five rounds that is a gate whose dominant output is a
false alarm, which is why it is a merge's line rather than a round's.

**`RUSTFLAGS="-D warnings"` on the clippy line is not decoration**, and it is the same sentence
`doc/verify.md` already writes over the cross-target checks: the workspace's lint levels are `warn`
so that an ordinary build stays usable, and CI turns them into errors — so a lint run without it is
a *weaker gate than the one that gates a push*, and `CLAUDE.md` principle 1's "warnings are errors
in CI" was true of one machine only. A round that runs it without the flag can be silent here and
red there, which is the shape of failure ADR 0450 was written about.

`tools/state.sh` runs the whole of all three tiers and prints each gate's own summary lines; run
that when what you want is the state — which is a merge's job or a measuring round's, not an
ordinary round's — and run the tiers above when what you want is a gate to fail. Either
way the numbers come off the run, never off a document. **It runs neither of the first two lines**,
and honestly so — its whole subject is figures, and a silent lint run has none — so a round that
reaches for the script has not linted or checked formatting at all, and owes those two here.

**This section owns the sequence, and nothing else states it.** `doc/HANDOVER.md` used to carry a
second copy under "Verify it" and the two drifted — one said 1369 tests where the gate printed
1371, and it never listed `render-raster`'s corpus gate at all (ADR 0232 §4). Two documents stating
one command is how they drift, so one owns it.

### The change → gate map

**The rule is first and settles most rounds; the table after it is a lookup for the change it
does not settle.** Read the six, apply them, and open the table only if none of them answered. All
six are about **tier 2**: tier 1 is unconditional and tier 3 is the merge's, so what a rule decides
is which of tier 2's lines this round owes.

**1. Tier 1 is every round's**, whatever it touched — the four workspace lines, **the two
`fuzz/` lines with them**, and `cargo test -p conformance`. It is about a tenth of the sequence's
cost and it is the only thing that sees a lint, a broken doctest or a test somewhere else in the
workspace. (This rule said *the core*, four lines, for most of its life; the tier is that core with
the conformance line moved up beside it, which is where the measurement put it.) **One of them now carries a cost floor as well**:
`cargo nextest run --workspace` runs `pdf-vfs`'s `tests/a_face.rs`, which walks three documents'
whole trees twice and fails if any generator ran twice for one subject (ADR 0894). It is there
rather than in a corpus line because a cost defect found by a walk has already been merged, and it
is affordable there because it is a *count* — ten seconds, and no clock to be wrong about.
`tools/pdfref/tests/end_to_end.rs` carries the second
one, on the same rules: a page asked for three times from two work directories runs one renderer,
and it fails naming the key if a second spawn was not excused by the first having been kept
nowhere (ADR 0898). **The `fuzz/` pair is part of the core and this line said "the first four"
for as long as either of them has existed**: `--all` and
`--workspace` mean *every package in **this** workspace*, so not one of the four reads a line of
`fuzz/`, and what covers it has to name its manifest.

**2. Eight crates are under everything**, and this is the rule that answers most rounds — it was
the map's *first row*, which is what a round's own record usually calls it.
`pdf-render`, `pdf-syntax`, `pdf-font`, `pdf-model`, `pdf-signature`, `pdf-spec`, `pdf-sandbox`
and `render-cpu`
are what draws the page and what every corpus-scale gate rasterises with, so a change in any of
them runs **the whole of tier 2** and there is nothing to look up. It no longer runs the whole of
tier 3, and that is the change ADR 1036 made: the walks stay, and the merge runs them once for the
batch rather than each round running them for itself. `pdf-signature` is here because
`pdf-model` depends on it for §12.8.2.2's `/DocMDP` level and §12.8.6's usage rights (ADR 1020);
the day that dependency goes, this list is seven again and the lookup table gets a row.

**3. A round that can change a pixel runs tier 2, and looks at a page.** That is any change to
the crates in rule 2, and it is not a judgement about how small the diff looked: trap 1's whole
subject is that a change nobody expected to draw differently did. `raster_golden` is tier 2's
change detector over this tree's own output and `pdf-model --test corpus` its counts; what ranks a
changed page against the other renderers is `oracle`, which is tier 3 and therefore the merge's —
so a round that moved a pixel **says so in its record**, and the merge is where the verdict lands.
Trap 1's rule is unchanged by any of this and is not satisfied by a gate: render the page and look.

**4. A round run alone, outside a batch, runs everything every fifth round — all three tiers**,
whatever it touched, because a map is a claim about the crate graph and a claim decays. A round
run in a batch owes no such sequence: the merge runs all three tiers on every batch, which checks
the map five times as often as the fifth-round rule did, and six rounds each owing a twenty-five
minute walk the merge then repeats is the duplicated work ADR 1638 retires.

**5. A merge runs everything, always — all three tiers**, and the paragraph below the table is not
relaxed by any of this. This rule is what makes tier 3's demotion safe rather than a hole: every
walk still runs over every change, once per batch on `main`, which is where its catches have
actually happened.

**6. A documents-only change** — `doc/`, `CLAUDE.md`, a `tools/*.sh` — is under nothing the gates
rasterise: run **tier 1**, whose last line is `cargo test -p conformance`, and which reads citations
and quotations out of the tree; plus `--bin quotations` and `--bin pointers` where the change moved a document or
a pointer.

**What none of the six settled is in the table**, and *reach* there is the crate graph rather than
the file's own crate.

#### The lookup

Rules 2 and 6 were this table's first and last rows and are stated above instead; that is the
whole of the reordering (ADR 0983).

**A row that names a tier-3 walk names it because that walk's *subject* is the crate in the first
column** — the writers, the validator, the virtual filesystem. That is rule 5's one exception in
the other direction: a round whose change is what a walk asserts about runs that walk, and a round
merely upstream of it does not. Everything else in tier 3 is the merge's.

| a change in | is under | so run, beyond tier 1 |
|---|---|---|
| `render-raster` | the third rasteriser only | the quorra gate, and its second coverage lane where the change is a quorra release or the zoom path; and `--test turn_path` (release) where the change is in raster's encode, an image decode or anything a page turn crosses — `doc/performance.md`'s turn and step rows, banded (ADR 1513) |
| `raster/crates/raster-gpu/src/` | CI's `raster-examples` job, and the gate that runs it here | `tools/batch.sh raster-examples`, behind the lock through `tools/bounded.sh` as every walk is: each example `.github/workflows/ci.yml` names, built `--release` and run with `--check` under `xvfb-run`, one line per example with its exit and its log. `cargo test` builds no example, so an example's assertions run nowhere else here, and two went stale where only the owner's CI ran them (ADR 1563); the merge runs it as `t2-raster_examples` (ADR 1575) |
| `render-gpu` | no gate at all | the workspace tests are the only judge — `cargo test -p render-gpu --test headless_gpu`, **without** `--ignored`: none of its tests is ignored, so that flag runs zero of them and exits 0; say so, and consider `doc/verify.md`'s cross-backend runs |
| `viewer-core`, `viewer-accessibility` | the two censuses | `selection_census`, `accessibility_census` |
| `viewer-ui`, `viewer-gtk`, `viewer-qt`, `viewer-ffi`, `viewer-host`, `viewer-confined`, `confined-transport`, `pdf-fuse`, `pdf-vfs-ffi`, `kio/` | the launch-path gate for the first of them, and the awkward-class sweep for `viewer-confined` | the core, which builds and tests them; section 5's `tools/batch.sh install` rebuilds what a person runs at the merge. **And `--test awkward_classes`, with its `--bins` line, where the change is in `viewer-confined`, `confined-transport` or anything `pdf-view-worker` links** — the sweep of the other confined program, in the sequence (ADR 1015). **And `--test launch_path` where the change is in `viewer-ui`, `viewer-core` or anything the launch path crosses**, which is `CLAUDE.md` principle 2's four numbers and is the only gate in this sequence that can see them. **`confined-transport` is under two crates**, so a change there is a change to `viewer-confined` *and* `pdf-vfs`, and both of their worker binaries have to be rebuilt before their tests are believed — trap 10 twice. **`pdf-fuse` and `pdf-vfs-ffi` are the two faces and neither has a gate of its own**: the workspace lines build and test both, and `pdf-vfs-ffi`'s own tests need `pdf-vfs`'s worker beside them, which `cargo nextest run --workspace` and `cargo test -p pdf-vfs-ffi` both produce (they build a package's bin targets) — the trap-10 shape would bite only if a `--profile gates --test` line were added for this crate, as it did for `pdf-vfs`. **`kio/` is not in the workspace at all** and no `cargo` line reaches it; what builds it is `crates/pdf-vfs-ffi/tests/the_kio_worker.rs`, which runs CMake and a KIO client and **skips, printing what is missing**, on a machine with no `cmake`, ECM, Qt 6 or KF6 — so this sequence stays green with no KDE installed, which is the whole reason that directory is outside the workspace (ADR 0869) |
| `pdf-script` | the Tier 1 column, and the Tier 0 form above it | `--features engine --test script_corpus`, the line in tier 2: every script Tier 0 does not run, run in its document's realm and held to its own ceilings (ADR 1625); and `-p pdf-model --test script_corpus`, the displayed values held by name, where the change reaches the view state's script sites |
| `tools/conformance`, `doc/conformance/ledger.toml`, a doc comment citing a clause | the conformance gate | `cargo test -p conformance` |
| `pdf-archive` | the validator's corpus walk, and the converter's | `cargo test --profile gates -p pdf-archive --test corpus -- --ignored --nocapture`, the validator against every witness the veraPDF corpus holds, per target, with `over` — a document its author built to conform, failed here — the column to read; and `-p pdf-transform --test archive_corpus`, because the converter's whole contract is this validator run twice, before and after, so a validator that changes its reading changes what the converter is held to. Plus `cargo test -p conformance`: the crate's table cites ISO 19005 by section and the ledger's rows for it. **The validator's walk reports and does not yet hold** — its own header says turning an adjudicated expectation into a gate is a later, deliberate step — so what fails the line today is a panic or an unreadable witness, and the `over` column is read by the round rather than by the test. Both walks are `#[ignore]`d for the submodule's size, skip loudly without it, and cost seconds; neither interprets a page, so neither needs the sandbox worker and both say so (ADR 1015). **And a third walk since ADR 1055, `-p pdf-archive --test cross_check`**: the survey's named-resource selections against `pdf_model`'s interpreter's over the same corpus, both recording through `pdf_model::content::ledger`, scoped to the constructs the survey walks; a change to `survey.rs`'s walk, to `pdf-model`'s `resources.rs` or to any site that enters a nested stream is a change to what it asserts. Tier 3 like its neighbours: it interprets every page, costs about two minutes, and its catches are the merge's |
| `pdf-transform` | the transform gate, and the writers' walks | `cargo test --profile gates -p pdf-transform --test gate -- --ignored --nocapture`, which carries RFC 0002 section 12's perf floor and holds the verbs' inventories to the document; it needs the sandbox worker beside it like the rest. And the seven corpus walks named in the sequence above — `writer_corpus`, `split_corpus`, `merge_corpus`, `pages_corpus`, `optimize_corpus`, `foreign_corpus` and `archive_corpus`, the last of which is the converter held to the validator (the `pdf-archive` row above). **What each of them asserts is in its own `//!` header and not here**: every one of them opens with the clauses it holds its output to, the layer of RFC 0002 section 9 it is, and what it does when the programs it needs are not installed. `foreign_corpus` is the only one that asks **somebody else** — qpdf, poppler and mupdf over each of the five writers' output, every foreign reading compared with that *same* reader's reading of the source page and never with ours. All six are corpus walks, so they run under `tools/bounded.sh` (`doc/environment.md`) |
| `pdf-vfs` | both sides' walks | `--test write_corpus`, which drives RFC 0003 section 5.2's five verbs over every corpus document the core opens, and `--test read_corpus`, which lists, `stat`s and reads the whole of section 4's layout through the **confined** worker over `doc/pdf.js` whole plus a class-balanced sample of every other corpus on the disk. **What each holds its answers to is in its own `//!` header**, including the cost floors, which are *counts* rather than clocks (`Vfs::questions`, `Vfs::forgotten` — trap 33) so that a neighbouring round's load cannot fail them. Both are corpus walks, so they run under `tools/bounded.sh`. **The `--bins` line above them is not optional and is trap 10**, on a distinction worth keeping: `cargo nextest run --workspace` and `cargo test -p pdf-vfs` both build a package's bin targets, so under those the `pdf-vfs-worker` beside the test binary is this build's; a `--profile gates --test` line builds **one test target and nothing else**, as it does for `pdf-sandbox`, so under that line the worker would be whatever an earlier round left. This crate's own tests are `cargo nextest run -p pdf-vfs`, which the workspace line already runs: `tests/a_face.rs`, `tests/a_write.rs` and `tests/confined.rs`, the last of which re-executes itself under the confinement to check that a forbidden system call kills — and that a font looked for on the machine does *not* (trap 31). A **death** — `killed by signal N` — fails a walk wherever the sentence appears |
| `raster-compare`, `test-scenes`, `pdfref` | whichever gate names them | the core, plus the gate whose harness they are — `raster-compare` and `pdfref` are the oracle's and quorra's. **`pdfref` reaches three gate lines rather than one**, and it carries their cost floor: `pdfref::Runs` counts how many times a reference renderer or an extractor was actually *spawned*, how many of those were for a key the run had already run, and how many produced something the cache kept nowhere — and `oracle`, `text_extraction` and `selection_census` each fail on a repeat the ceiling does not excuse. It is a count and not a clock, for trap 33's reason: `Statistics`'s hits and misses cannot see a lookup that never reached the cache, nor tell a second miss on one key from a first miss on another |

**A merge is a round of its own, and it runs this sequence on `main`.** Green in a worktree
establishes nothing about `main`: a parallel round's gates are the truth about a tree that
branched before its neighbours' files existed. The proof is in this tree — eleven
`clippy::pedantic` warnings lived on `main` for five rounds while four rounds truthfully recorded
the lint run silent in their own worktrees, and two more parallel rounds broke the quotation gate
the same way, on merge (`doc/history.md`'s 455–484 block summary). So whoever merges a worktree
round into `main` owns this section for the merged result, before the next round branches from
it. There is no exemption for a merge that "only touched docs" — the five-round breakage was in
an example file nobody thought about either.

**This sequence is half what it cost before every step of it was measured** and four
things changed; ADR 0222 has the table and the argument, and `Cargo.toml`'s profiles carry the reasoning beside the settings. What binds here —
and this line said *five notes* over ten bullets, because each round that added one counted the
list it had read rather than the list it left:

- **`--profile gates`, not `--release`.** Release-grade optimisation with cheap linking, because a
  fat whole-graph link *per gate binary* was most of a round. All eight gates were run under
  both profiles and their output compared line by line — every verdict, every page, every
  citation, **every field identical**. `--release` still works and is still the same gate; it is
  only slower. **`[profile.release]` did not change**, and §5's binaries are still built with it.
- **`cargo nextest` is a user-local install** — `cargo install cargo-nextest --locked`, or the
  prebuilt from `https://get.nexte.st/latest/linux` into `~/.cargo/bin`. Without it,
  `cargo test --workspace` is exactly the same gate at three times the wall clock, and that is
  what CI runs. `nextest` skips doctests, which is why the line after it is there.
- **A round that adds a test writes down nothing about the count.** This file carried the test
  count and its arithmetic for dozens of rounds and each was separately wrong at least once — the
  sharpest instance being a round that copied the gate's own number into the line it was told to
  update and left the *sum* beside it untouched, seven rounds behind. Two rules survive that, and
  they are about running rather than about writing: a number is current only for a round that ran
  the gate **last**, after its final edit; and a round that writes a number it did not watch print
  writes the previous round's.
- **Run the sequence on a quiet machine, and run nothing beside it.** Three of these lines spawn
  *other programs* with time budgets — the oracle's poppler, mupdf and ghostscript, the text line's
  `pdftotext`, quorra's device — and a budget is wall clock rather than work, so a reference
  renderer that would have finished loses to a `cargo` build running in another terminal. A round that ran the ledger sweeps beside the
  sequence saw the oracle report **38 not comparable and 873 agreeing in 218 seconds**; the identical tree, run alone,
  reported **13 and 907 in 57 seconds**, and the section's exit status went from 101 to 0. Nothing
  had changed but the load. **A gate that spawns a reference is a measurement of two programs, and
  a loaded machine is a silent third**: the failure is legible as a regression in the thing being
  measured, which is the worst shape a false result can take. Sweeps, censuses and background
  builds go before the sequence or after it, never during.

  **And it is not only the clock that moves.** `foreign_corpus`'s `bookmarks` lane failed on `bug1997343.pdf` with a §14.7 fault
  — "mupdf resolves the source page's parent-tree entry to 90 entries and ours to 79" — beside a
  neighbouring round's walk at a load average of 32, and the identical tree re-run alone reported
  `bookmarks: §14.7 faults: 0`. Nothing there is a duration: what changed is **how much structure
  the foreign reader resolved**, which reads as a defect in *our* carry rather than as contention,
  and which no `slow` marker and no budget counter fires on. So the rule above is wider than its
  own wording: a contended reference can give a *wrong answer* as easily as a late one, and any
  foreign-reader disagreement seen on a loaded machine is re-run alone before it is diagnosed.

  **And it is not only the lines that spawn a reference.** `tests/corpus.rs` carries a wall clock
  of its own — no document may take longer than 30 s to open and draw — and that threshold has produced a false
  positive: `ContentStreamCycleType3insideType3.pdf`, the `MAX_FORM_DEPTH` cycle that costs 3.8 s
  in ADR 0810, took **32.23 s** and failed the gate at exit 101 while a neighbouring round was
  building and running `cargo nextest run --workspace` in its own worktree at a load of 26. The
  same gate on the same tree passed twice quiet in the same round, the whole walk in 11.5 s with
  `0 slow`. An eightfold margin is not proof against a machine running two rounds, so a `slow`
  failure is a thing to re-run alone before it is a thing to diagnose — and `ps` rather than
  `pgrep -af '…corpus…'` is what finds the neighbour, because a build is not a walk and the
  memory rule's own grep does not name it.

  **And the answer to all three of these is not always a better band.** Two of this tree's cost
  gates are now *counts* rather than clocks — `pdf-vfs`'s `repeated <= forgotten` (ADR 0894), held
  in `tests/a_face.rs` and `tests/read_corpus.rs` — and a count has none of this paragraph's
  problem: a neighbouring round's load cannot move it by one, so it needs no band, no calibration
  probe and no re-run alone. Where a cost property can be stated as *how many times the expensive
  call ran*, that is the gate to build; a duration is what is left when it cannot, and ADR 0884 is
  the five-part construction a duration costs here.

  **The third witness is the largest swing recorded**:
  `pdf-transform`'s gate carries RFC 0002 section 12's floor of 40 pages a second and reported
  **33.3** while a neighbouring round ran a `render_at` sweep at 120 % of a core with a
  fifteen-minute load average of 19.69. The same tree, quiet, reported **198.3** — five times the
  floor, six times the failing figure. Two things follow, and the second is the one that is new.
  **The predicate a round waits on has to match the gate's kind**: the memory rule's "one corpus
  walk at a time" is about *memory*, so waiting on other rounds' gate binaries is right for a
  walk and blind to a sweep or a build, which are what a *clock* competes with. And **a poller
  that reads command lines can match itself** — `ps -eo args=` prints the polling `grep`'s own
  arguments, which contain the very path pattern being searched for, so the predicate never goes
  empty and the wait never ends. `readlink /proc/PID/exe` is what a process *is* rather than what
  it was asked to be, and no pattern of the poller's can appear in it. One round's deadlock was
  the same mistake in its other direction, and both are the reason a wait predicate is worth as
  much care as the gate it guards.
- **One of these commands runs a C compiler**, and it is the only gate in this sequence that does.
  `viewer-ffi::a_c_program_drives_the_abi` builds `crates/viewer-ffi/c/open_a_page.c` against the
  crate's own header with `-Wall -Wextra -Werror`, links it against the `cdylib` — which it asks
  cargo to build, because `cargo test` does not — and runs it on a document. It **skips** where
  there is no `cc` or `gcc`, printing why: a machine without a C compiler cannot run it, and
  failing there would make the gate a coin toss. CI has one, so on CI it is not a skip.
  **A C++ compiler runs too and is not a gate**: `clippy --workspace` and `test --workspace` build
  `viewer-qt`, whose `build.rs` compiles `cxx-qt`'s generated bridge, and on a **cold** build that
  prints `cargo:warning=` lines beginning `viewer-qt@0.1.0:` — `-Wmaybe-uninitialized` inside
  `rust::cxxbridge1::Vec<T>::Vec()`. They are **gcc's, about generated code, and not clippy
  lints**, and a warm build prints none of them, which is exactly what makes them easy to read as
  a regression.
- **`-- --ignored` runs every ignored test in the binary, which is not always what the line wants.**
  It is a switch on the whole binary rather than a filter, so a test that carries `#[ignore]` to mean
  *run me explicitly* is run by every gate line that names its file. The oracle's binary held one for
  thirty-nine rounds — a derivation whose own doc comment says it "is not itself a gate" — and the two
  walked the corpus side by side under `rayon`, which doubled the line's wall clock and inflated the
  per-page spans it prints (ADR 0282). **The rule that came out of it is where the fix goes, not what
  the fix was**: a test in a gate binary that must not run in the gate declines *by itself*, because
  an invocation can be copied without its guard and a test cannot be run without itself. Nothing here
  changes when one is added.
- **The `selection_census` line is the one that clicks**, and it is here rather than in
  `doc/verify.md` because two of its three properties are exact and it is what catches a defect in
  the loop from a press to a selection — which is the loop `doc/traps/the-interactive-loop.md`'s trap 12a is about and which nothing else gates. Its *drag fraction* is
  printed and **not** ratcheted, by `doc/todo/05`'s standing rule; what fails the line is a
  selection that is not the interpreter's readback, a caret whose own point lands somewhere else,
  or a panic. Six seconds with a warm extraction cache, which it shares with the line above it.
- **The `accessibility_census` line is a *ratchet* and says so**, which is the third of ADR 0323's
  instruments and the shape it was designed with: no other implementation puts a comparable tree on
  AT-SPI, so there is nobody to disagree with us and a count that cannot fall is what is honestly
  available. It entered this list only after its counts had held across
  rounds (ADR 0425), on `doc/todo/05`'s own rule. Twenty seconds. What fails the line is a capability count falling, a defect
  class growing, a panic, an untagged page given a structure it does not state, or a line whose
  characters disagree with its own text. **A tree without the `doc/pdf.js` submodule prints why it
  is not ratcheted instead of failing**, because a smaller population is the one reason a floor can
  break that is not a regression. **And a build with no `pdf-sandbox-worker` beside it now stops
  the line instead of moving the ratchet by nine elements**, which it did, deterministically, while
  four rounds diagnosed it as a build directory, as staleness and as Cargo feature unification
  (trap 16, ADR 0557).
- **The `text_extraction` line is three gates and gains no line**, which is the other side of the same
  mechanism and is correct: `the_text_we_draw_agrees_with_pdfboxs_frozen_extraction` is a gate. It
  compares documents against the `PDFTextStripper` output Apache PDFBox checked in beside them — a
  *frozen* second reference, which cannot drift under this tree the way the machine's poppler can
  — and it costs a fraction of the pdf.js gate, because its reference is a file rather than one
  `pdftotext` invocation per document. That is what "earned a place" was supposed to mean. ADR
  0259.
- **The `fixed_documents` line is the merge round's, and it is the only gate that sees the
  SafeDocs crawl.** Every other gate here walks `doc/pdf.js`; a fix found by ranking the crawl is
  measured once, by the round that makes it, in a tree without its neighbours' work — so two
  branches touching no common line can defeat each other with every one of these lines green, and
  one did (ADR 0458). `doc/checks/fixed-documents.toml` is the appendable half and
  `doc/todo/03` §20 the rule. Half a minute. **It needs the worker**, which is trap 10 and is the
  line above the corpus gate; it **skips rows whose document is absent** and counts them, because
  `corpus-cache/` is machine-local, and it **fails rather than passing quietly** when none of them
  is there.

- **The `launch_path` line is `CLAUDE.md` principle 2's, and it is the one line here that is
  `--release`.** That principle names four numbers — cold open, time-to-first-page, page-turn
  latency, memory high-water — makes cold graphics bring-up a fifth gate of its own, and this line is the
  command that prints them. `crates/viewer-ui/tests/launch_path.rs` is the harness, `doc/checks/launch-path.toml`
  holds a band on each figure, and ADR 0884 is why a wall-clock gate can be believed here at all:
  every clock figure is the minimum of nine fresh processes pinned to the machine's fastest cores,
  and a calibration probe decides whether the clock is judged, so a loaded machine prints
  `NOT JUDGED` where it would otherwise have produced a fourth entry in this file's list of false
  failures.

  **This line runs the half of the gate that has no clock in it, and that is the owner's answer to `doc/questions/Q29`** (`A29`: options 1 and 2
  together). Without `PDFVIEWER_LAUNCH_CLOCKS` in the environment the gate measures one sample of
  each phase and judges the figures a machine cannot move — the bytes an open reads, **the read
  calls it makes, the instructions it executes**, what it costs in memory, and what page one has
  allocated — twenty-one of them, on any machine at any load, in **three seconds**. The figures
  that are wall clocks are claims about a machine, and `doc/verify.md` says to set the variable and
  run the whole thing when a round has the machine to itself. What made that split affordable is
  the counted open: `open_kinstructions` answers principle 2's own question about a cold open —
  *did opening a document become more expensive* — exactly, under callgrind, with no stopwatch in
  it (ADR 0917).

  The profile is not a slip. `[profile.gates]` costs `Document::open` 4.06% to 12.30% against
  `[profile.release]` (`Cargo.toml`'s table, ADR 0666), which is wider than the bands, and a launch
  number is a claim about the program a person runs — so this line takes `release` and the harness
  prints-without-judging under anything else. Its walk's `--release` worker build is trap 10 in
  the same profile: a `--release --test` line builds one test target and the worker beside it
  would otherwise be whatever an earlier round left, in the wrong profile or not at all. A cold
  `release` link of `viewer-ui` in a fresh worktree is about two and a half minutes and nothing
  when warm; section 5's `tools/batch.sh install` builds the same binaries at every merge.

- **Every line above that interprets a page refuses to run without the sandboxed decoder**, and the
  `cargo test -p conformance` line at the end of the sequence is what keeps that true. §7.4.6's
  `CCITTFaxDecode`, §7.4.7's `JBIG2Decode` and §7.4.9's `JPXDecode` are decoded by a *separate
  program* that Cargo will not build for a test of another package (trap 10), so a gate line run on
  its own measures a build that draws every other image and none of those three — silently, and by
  enough to move a ratchet (trap 16, ADR 0557). Each such line calls `require_the_sandbox()`;
  a gate that needs no decoder — `dates` and `xmp` were the first two, and the object-graph walks
  added since carry the same line — says so in a line beginning `// no sandbox worker:`, with the
  reason; and `tools/conformance/tests/sandbox_gates.rs` reads **this section's own command
  block** and fails a line that does neither. **So a gate added here owes one of the two**, and
  the check will say so.
- **Every `#[ignore]`d test file is named in this block or says why it is not, and that is
  checked.** `cargo nextest run --workspace` skips an ignored test, so a walk that is `#[ignore]`d
  is run by this sequence or by nobody — and the direction review found six in no line here
  (`doc/reviews/984`, Finding 3): the validator's walk over the veraPDF corpus, the converter's
  walk over the same corpus (the one that found ADR 1006's lying signature, on a merge), the
  save round-trip, §12.6.3's trigger census, `pdf-syntax`'s on-disk comparison and the confined
  viewer's sweep of the awkward classes. All six are lines above now, each a count and none a
  clock, and together they cost the sequence well under a minute (ADR 1015; ADR 1011 for the
  save). `tools/conformance/tests/state_sections.rs` derives the population from the index —
  every tracked `.rs` under `crates/` and `tools/` with an `#[ignore` attribute — and fails by
  name on a test file this block does not name unless the file carries a line beginning
  `// not a gate:` with a reason a reader can check; it fails the other way on an excuse in a
  file this block *does* name. **So a walk added to the tree owes a line here and a
  `tools/state.sh` section, or the excuse**, on the day it is added. The review's count was seven,
  and the seventh was a doc comment saying a test *had been* ignored: the check matches the
  attribute where an attribute is, at the start of a line (trap 11).
- **The `awkward_classes` line is here although `pdf-vfs`'s read walk gates the same class of
  defect over more questions** (ADR 1015). It shares that walk's
  population and its filter and not its *program*: `pdf-view-worker` is the process a person reads
  pages in, and a system call in code the two workers do not share is caught by this line alone.
  Its program is its own package's, which Cargo builds for that package's integration test as it
  builds `pdf-vfs-worker` for the `pdf-vfs` lines (ADR 1718 section 2); it decodes its three
  confined codecs in-process so no `pdf-sandbox-worker` can change what it holds, and runs under
  `tools/bounded.sh` like the other walks — it is the heaviest of the six by memory, at a few
  gigabytes over its process tree.
- **`pdfref-hayro` is the oracle's fourth reading and nothing built it.** It is a *program*, found
  beside the running test binary, and its absence costs no verdict — `Reference::Hayro` never
  votes — but it is what a person looks at on a page the three references cannot settle. It
  existed under `target/release/` only because some earlier round happened to run
  `cargo build --release -p hayro-compare --bins`. It is the oracle walk's second `--build` now,
  after the sandbox worker's and inside the same hold, as `tools/state.sh oracle` spells it (trap
  109, ADR 1710).

- **The quorra gate runs one of two coverage lanes, and the other one is a round's to ask for.**
  `PDFVIEWER_RASTER_COVERAGE=gpu` points the same gate at the lane `viewer-ui` switches to past ten
  times magnification; paired with `PDFVIEWER_RASTER_SCALE=4` it is the population that lane
  actually draws for. Both turn the ratchets off, and the run says so. **Two kinds of round owe
  this run**: one that takes a quorra release, because the release may be entirely inside a lane
  §2 does not exercise — which `74c4994d` was, and it took 24 refusals off that lane at 4× while
  moving nothing at all on the default one (ADR 0283) — and one that changes the zoom path.

- **The quorra gate runs at the quantum this product ships, and that is the line above rather than
  a fourth run.** It is the same invocation: `tests/corpus.rs` takes
  `render_raster::options()`'s `glyph_quantum` instead of forcing it off,
  so the sequence's own line measures the shipped configuration and **costs nothing extra** — it is
  in fact the *faster* of the two settings, because the atlas reuse the quantum exists for is what
  the run then gets. `PDFVIEWER_RASTER_GLYPH_QUANTUM=off` is the isolation column, and like the
  other knobs it turns the ratchets off.

  **What that column would have caught is the reason it is here.** For the whole of this gate's
  life it turned the setting off, on reasoning that sounded like isolation — and quorra's ADR 0073
  then found `GlyphPlacement::of` rounding a fractional phase up to the bucket count and taking
  `% q` of it, which seated 3.1% of sub-pixel phases per axis a whole device pixel from where the
  placement asked, on the lane that draws text. Run at the shipped quantum against the pin before
  the fix, this gate reports **155 pages differing where its ratchet holds 22**, and fails on the
  list rather than on a statistic. The only gate that could see it at all was
  `real_pages.rs::the_glyph_quantum_cost_stays_bounded`, an envelope over a mean, a worst tile and
  an SSIM — and a 3% population of whole-pixel misplacements moves an envelope without breaking it,
  which is exactly what it did. ADR 0498. **The general rule is worth more than the instance: a
  gate that turns a shipped setting off is measuring a configuration nobody runs**, and the
  isolation it buys is worth having as a second column and not as the only one.

- **`fuzz/` is not in the workspace, so nothing above it builds the targets**, and the two `fuzz/`
  lines above are the whole fix. `fuzz/Cargo.toml` declares a `[workspace]` table of its own so that
  `cargo-fuzz`'s sanitiser and profile settings stay off the tree, and `--all` and `--workspace`
  therefore reach neither the fuzz crate nor any of its binaries — **for the formatting line as well
  as the lint one**, which is where two rustfmt diffs sat under a green gate. The clippy line is not
  a `cargo check`: it wants no nightly and no sanitiser, and `fuzz/Cargo.toml` restates the tree's
  lint levels because cargo resolves `workspace = true` against *this* workspace and offers no way
  to point at another's. The mechanism, what the levels were worth when they arrived, and the
  general shape are `doc/traps/instruments-and-reports.md`'s trap 23 (ADRs 0739, 0742).

  **What keeps it closed is `tools/conformance/tests/workspaces.rs`**, and it is derived rather
  than listed: `cargo locate-project --workspace` is asked, for every tracked `Cargo.toml`, which
  workspace root governs it, and every root that comes back must be named by a `cargo fmt` line of
  this section, by one of its `cargo clippy`, `cargo check` or `cargo build` lines, and by a
  `cargo clippy` line under `RUSTFLAGS="-D warnings"`, while stating the same lint levels the
  tree's own root does. A third workspace added to this tree fails that gate on the day it is added
  rather than on the day somebody notices.

  **Why the question *do the targets still compile against the tree they fuzz* is a line here at
  all: they did not, for fourteen rounds.** One round reshaped `Answer::Frame` to carry a
  page apiece and another reshaped the accessibility answer the same way; `confined_wire` matched on the old shapes and no local gate saw it, because
  the only instrument that builds `fuzz/` is a CI job that was itself failing for an unrelated
  reason the whole time. Principle 3 makes that worse than a compile error — fuzzing is meant to be
  continuous from the first parser commit, and between those rounds there was none.

  The tell is worth keeping: **the target that broke was `confined_wire`**, the one speaking
  `viewer-confined`'s protocol, so it is exactly the target a *boundary* change breaks and exactly
  the one no parser-touching round would think to run. A round that changes a `Command`, an `Event`,
  a `Query` or an `Answer` is a round that owes this line, and now every round runs it.

**The fuzz targets are `doc/verify.md`'s**, and `tools/state.sh counts` says how many there are.
Three rules bind a round rather than a count:

- **A round that touches a parser runs the target that covers it**, and a round that touches
  `pdf-font`'s glyph-table repairs runs `sfnt` **with its corpus seeded**, because unseeded it
  never forms a table directory and tests nothing. Five of the targets need a seeded corpus;
  `doc/verify.md` says which.
- **A target is only as good as the code its binary contains, which is a question for `nm`.** The
  `page` target exists because `nm` found `pdf_model::interpret` — clauses 8, 9 and 11 — in one of
  the thirteen binaries that preceded it, and that one calls it on a page with no `/Resources`
  (ADR 0264).
- **`page`'s corpus is the expensive part, and the merge that reduces it has been spent.**
  libFuzzer's fork mode merges the corpus before it fuzzes, one execution per seed, and on the
  seeds `fuzz/seed_page.py` produces that merge had become most of every run's wall clock — one
  round spent fifty minutes inside it and got nothing back. `cargo fuzz cmin page` writes the
  reduced set back, and run once it took **the corpus down to
  about a quarter of its files and a seventh of its bytes, at no cost in coverage at all** — a
  `cmin` keeps exactly the distinct-coverage set, so the reduced corpus carries the same edges and
  the same features the whole one did — `cmin`'s own `MERGE-OUTER` line says so, and a fork-mode
  run over the reduced corpus reports the same two figures back, with no crash, timeout or OOM.
  `du -sh fuzz/corpus/page` and `ls | wc -l` are where the level is.

  **What it bought is less than this file expected, and the reason is worth having.** This bullet
  said a `cmin` would "make the stated invocation an hour's job rather than an afternoon's". A
  fork-mode start over the reduced corpus is about **a third** of what the same pass costs over the
  whole one — not a quarter, which is where the *file count* went. **`cmin` throws away the cheap
  seeds**: what distinguishes a seed is coverage, and the ones with distinct coverage are the large
  slow documents. Its own rate says it, falling from 256 executions a second at the start to 14 at
  the end. So the merge is still most of a short run's wall clock, and a round with an hour rather
  than three still passes a smaller `-runs`.

  **The merge that does the reducing is not free and is not a round's default**: about three
  quarters of an hour, one execution per seed. A round `cmin`s when the corpus has grown rather
  than as a habit. **And a `slow-unit-` artefact is the sanitiser's, which is checkable rather than
  assumed**: render the largest of them under `examples/render_at` against a release build before
  believing one.
- **`cargo-fuzz` is installed and always was**; it is in `~/.cargo/bin`, which is not on `PATH`,
  which is what two rounds read as its absence.

`doc/verify.md` has the rest — `cargo deny`, the fuzzers, the cross-target checks, the callgrind
counters and the census examples — and says which of them a change needs.

## 3. Leave the ledger non-`unreviewed`

Every clause a change touches gets its row in `doc/conformance/ledger.toml` brought up to date.
This is `CLAUDE.md`'s rule and not a courtesy: a row that describes what the code *should* do is
how this project has been wrong four times.

## 4. Sweep, after a round that adds a verb

The sweeps live in [`01-ledger-partial-rows.md`](01-ledger-partial-rows.md), which says what each
one asks and what its first run found; that file is the reading, and this is the rule. **Run them
over every crate of the tree as well as over `ledger.toml`** — `conformance::roots::source_roots`
reaches all of them, derived from the workspace manifest rather than listed, because `raster/`'s five crates were once
outside every sweep for four months (ADR 1029) —
and run the grep-shaped ones over `doc/adr/` too, for the one thing an unmaintained
document can get wrong: a claim a later round disproved and left standing. The ledger has a gate
and the source does not, which is why one session found four claims in the code false for between
forty and two hundred sessions, including `pdf-model`'s own crate documentation and a doc comment
that had *predicted* its own expiry.

What each sweep is worth is in that file with its evidence. What belongs here is the shape they
share, the ones that break it, and the rule each of those leaves on a round:

- **Most sweeps read a row's stated *reason*** — a blocker that has expired, a capability the tree
  now has, a string a correction retired — and are therefore blind to a row with no reason at all.
  The sweep for that one prints every `partial` row whose note names nothing owed, which breaks
  the ledger's own definition of the status.
- **Six judge a claim against something that is not a ledger row's reason**, and each leaves a rule
  a round owes. What each asks, what its first run found and how to read its noise are
  [`01`](01-ledger-partial-rows.md)'s, under *The sweeps as commands*; the ordinal is the ordering
  and the anchor there.

  | sweep | judges a claim against | the rule it leaves a round |
  |---|---|---|
  | `--bin overstated` (18th, ADR 0475) | a **descendant row's** denial of what its parent claims — the only sweep that opens no source file, and the shape neither the seventh's sign nor the fourteenth's population can see | — |
  | `--bin overtaken` (19th, ADR 0491) | `doc/adr/`'s numbering, the only date this project has, over the oracle's page-list notes | a round that rewrites a note cites its own ADR in it |
  | `--bin quoted` (20th, ADR 0495) | the oracle's **printed output**, which is why it is the one that takes an argument | a round quoting a gate figure in a note quotes it to the precision the gate prints |
  | `--bin unpriced` (21st, ADR 0606) | that same output, for the bound a contradicted page actually fails — ADR 0497's sixth criterion made mechanical | a round that writes or rewrites a group note names that bound, in the gate's own words |
  | `--bin parts` (22nd, ADR 0709) | the **workspace's own membership** — member directories, each package's `src/bin/`, `.gitmodules` — for a cardinal counting this tree's own parts | a round that adds a part to this tree runs it |
  | `--bin undenominated` (23rd, ADR 0758) | the **corpora on disk**, for a sentence that quantifies over a corpus and does not say which | a round that widens a population runs it, and a round writing a count over a corpus names the corpus in the same sentence |

  **`parts` is a decay detector, so most of what it walks is correct sentences** — read the closest
  rung, which is a crate the whole population depends on. **Each of those six rows used to open
  *and it is the newest*, and five of the six were false**: every round that added a sweep wrote the
  phrase and none went back to the round before it, which is this file's own subject happening to
  this file.
- **Two check a *number* rather than a claim.** One is arithmetic on the ledger: every row that is
  `partial`, `reported` or `unreviewed` while every one of its direct children is settled. The
  other is every `Table NNN`'s `/Key` citation against the entries ISO 32000-2 actually puts in
  that table — `tools/conformance` verifies a cited table *exists* and prints its title, so a
  number that exists and names the wrong table reads exactly like a right one, and those arrive in
  **blocks**, a run of consecutive rows written in one sitting against the older standard. That one
  is `--bin tables`, and the catalogue below says how to read it.
- **The rest are a catalogue rather than a rule**, and it lives with the reading:
  [`01-ledger-partial-rows.md`](01-ledger-partial-rows.md)'s *The sweeps as commands* holds every
  one of them unchanged — what it asks, the command that runs it, what its output's noise looks
  like and which hits to read first. Most are `cargo run -p conformance --bin <name>` and
  seconds apiece — the default profile, because a release build of the crate's binaries costs two
  minutes of linking per edit to it and buys a few tenths of a second a run (ADR 1463); the errata ones are `tools/spec-errata`'s `check`, `emit`, `moved`, `renumbered`
  and `applied`,
  and **a round implementing a clause runs `emit` on that document *before* it writes, rather than
  `check` afterwards alone**. `tools/state.sh counts` is where a population goes, not a sentence
  here.
- **And they have a cadence now, because they had none.** Seventeen of the twenty conformance
  binaries had not been run in sixty sessions when
  `doc/reviews/1012-where-the-effort-goes.md` measured it — not because they find nothing, but
  because nothing said when. **The merge runs one, rotating in the order they are listed in
  `doc/todo/01`, and the merge's record names which one and what it printed.** One round ran
  all twenty once and wrote a verdict for each in
  `doc/history/1025-the-lint-that-remembers-and-twenty-sweeps-woken.md`: eleven print a reading
  list a round can act on, five print a backlog whose *level* is noise and need their populations
  partitioned before a cadence is worth anything, two want a tier-3 oracle log and so belong to
  the merge that runs it, one wants the nouns the round retired, and `--bin ledger` already runs
  inside `tools/state.sh`.

## 5. The binaries a person runs are installed by the merge, and a measurement builds its own

**What a person runs is in the main checkout's `target/`**, because that is where
`doc/running-the-viewer.md` sends them and the agent's build directories are somewhere their shell
never looks. **One command puts it there: `tools/batch.sh install`**, run by the orchestrator at
the batch boundary — after the fast-forward, before `close` (section 8 step 5). It refuses a worktree
holding uncommitted work and a branch whose HEAD is not `main`'s, so that nothing is installed that
no commit describes; it builds in the batch's own build directory, installs every file into the main
checkout's `target/` — the one place outside the worktree that script writes, and gitignored — and
writes `target/installed-from` beside them: the commit, and each file's SHA-256, because the
binaries carry no hash of their own (`quorra --version` opens a file called `--version`).
`tools/state.sh binaries` reads that record back, says how many commits `main` is past it and
whether every file is still the one installed; `tools/round.sh` fails while `main` is past it
(ADR 1511).

**A round installs nothing and measures nothing from there.** It may not write in the main
checkout, and a binary of the last merge is a measurement of the past — a page turn was once reported as "still lags"
against a binary three hours and six
commits old, one of which was the 40x page-turn fix. So **before any measurement** — of the launch
path, a page turn, a frame, a memory high-water, anything section 2's gates do not print — a round
builds `--release` what it measures, in its own build directory, and runs it from there:

```sh
built=$(cargo metadata --no-deps --format-version 1 | jq -r .target_directory)/release
ulimit -u 8192; tools/bounded.sh --lock --clock --round <session> --tree 12 \
    --build '--release --bin quorra --bin pdf-sandbox-worker' -- \
    "$built/quorra" --trace=launch doc/PDF20_AN001-BPC.pdf   # what the measurement runs, and its workers, built inside its hold
```

**Which directory that is has to be *asked for*, never written down.** The main checkout builds
where `~/.cargo/config.toml`'s `target-dir` says, a batch worktree where its `.cargo/config.toml`
says (`tools/state.sh disk` prints both), and a literal path installs or runs a **neighbour's**
binary: one round rebuilt the GTK host three times, installed it
three times, ran a feature that was working and saw nothing, because every run was of another
branch's program. It is trap 15's own subject, reached through an instruction rather than a habit,
and `install` asks Cargo in the worktree for the same reason.

**The names are written once, in `tools/batch.sh`** (`install_binaries`, `install_libraries`, and
`install_featured` for a program behind a feature, built in a Cargo run of its own so that the
feature reaches no window — `pdf-script-worker` with `engine`, ADRs 1616, 1625), and
`tests/batch.rs` holds them against the workspace's own manifests: every program of a package under
`crates/` is installed, every name installed is a binary target — `quorra-retrieve` is the one from
`tools/`, because a person runs it — and every package that builds a C library is installed. A copy
of a name list in a document drifts the way copies do: this section's loop once
installed three pre-rename names, and that could not fail, because Cargo removes nothing it no
longer produces, so `install` found months-old artefacts under the old names and copied them while
the renamed programs never reached `target/` at all.

**All of them, beside each other.** Each worker is looked for beside the running executable, and a
program that cannot find its worker refuses the work: `pdf_sandbox::WORKER_PROGRAM` is the
executable the viewer spawns for JBIG2 and JPEG 2000, with deliberately no in-process fallback;
`pdf-view-worker` is the whole viewer confined, which `viewer_confined::Confined` spawns and
`quorra-confined` is the window around it (ADR 0713), while `quorra` does not (ADR 0218); `quorrafs`
is RFC 0003's mount, and it opens no document without `pdf-vfs-worker` — the same relationship, and
trap 10 one directory over. `pdf-script-worker` is the confined engine every window spawns at the
first script the reader's `Scripts` level lets run, and without it that script is refused with the
sentence saying the worker was not found (ADR 1616). `quorra-retrieve` is not a window but a program whose whole output is
text a caller pipes (ADR 0257).

**`libviewer_ffi.so` and `libpdf_vfs_ffi.so` are there because a person links against them**: a C
program with `include/quorra.h` and no `-L` pointing at `/home/AI` is the only way somebody outside
this tree can try the ABI, and `kio/`'s CMake build takes the path to the second as a *required*
variable rather than searching — a `find_library` would pick up a copy of another revision, which is
what `quorra_vfs_abi_check` exists to make loud (ADR 0869).

**One invocation for the programs, a second for the libraries.** Each program is a whole-graph fat
link, and Cargo runs them beside each other where separate commands run them one after another —
measured both ways after touching one file in `pdf-model` (ADR 0222); `--bin` cannot name a
library, so those are the second. `--release` is deliberate and is the one place that still pays
for `lto = "fat"`: these are what a person runs and what every launch measurement is taken from,
and `--profile gates` exists so that the gates stop paying for it.

## 5a. Sweep the build directory when it passes a hundred gigabytes

`tools/state.sh disk` says what it is. A *clean* tree is about 17 GB of dev artefacts plus about
1 GB per release-grade profile; the rest is superseded output that Cargo on stable has no command
to remove — `cargo clean --gc` is nightly-only. So it is swept by hand, and `target/tmp/` is what
the sweep must **not** take.

**"It" is the build *root*, and for a long time neither instrument printed that.** `disk` reports
the round's own `target-dir`, which is deliberate and stays — trap 15 is what put it there — but
from a worktree that is a few hundred megabytes while the root holding it is over a hundred
gigabytes, and this threshold is about the second number. Both instruments print it now: `disk`
adds the root beside the round's own directory, and `tools/worktree.sh list` breaks it down by
whose each directory is, which the sweep below needs because **the root holds directories this
project's tools did not make and cannot judge** (ADR 0752).

The commands, which directory each takes and when, are `doc/environment.md`'s build-directory
entry; the shape of all of them is a profile directory by name and never `tmp/`:

```sh
rm -rf /home/AI/cargo-target/pdf-viewer-batch/debug   # at a batch boundary — never tmp/, see below
```

`target/tmp/pdfref-cache` is the reference-render cache (ADR 0020), and deleting it costs the next
oracle run about a thousand seconds of `pdftoppm`, `mutool` and `gs`. `cargo clean` takes the
whole directory including that, which is why the sweep names its subdirectories instead.

The cost of the sweep is one cold build, measured on the swept tree at about three minutes for
`cargo test --workspace --no-run` plus the whole gates profile, with `release` on top of that only
when §5 runs. It buys no speed — the warm no-op build was 0.42 s with the directory at 311 GB — so
it is hygiene, on its own schedule rather than every round. **`sccache` pays most of the cold
build**, which is the reason two rounds in a row declined the sweep for a cost that had already
been paid: the whole of §5's release set came back in a little over two minutes on a swept tree.

**Three things a sweeping round should expect, and none of them is a number.** Most of what comes
back is the *main checkout's own profiles*, not orphaned directories — a parallel round's
directory is live until its worktree is closed, and `tools/worktree.sh close` is what takes both.
**The root crosses the threshold again as soon as new rounds open**, because each new worktree
builds twenty to thirty gigabytes of its own within its first hour, so a sweep is a recurring
cost rather than a fix and the *rate* is set by how many rounds run at once. And **the sweep can
move a measurement**: the launch gate's cold arm reads a copy it makes beside the build directory,
so a swept tree hands it freshly allocated extents — `doc/todo/42` has what that cost one round
and how to tell it from a regression.

## 6. Write it down, then commit

**Check the file, not the script's exit status.** Twice in two consecutive rounds a Python edit put its `assert` *after* the replacements and
*before* the write, so a failed assertion left the file untouched while every other file in the
same commit moved. `grep` what you wrote back out of the file before committing it, which is the
same rule as trap 1 one directory over: the instrument that says a change happened is not the
change.

- The ADR, if the round made a decision. The argument goes there, not in the handover.
- **The session's record is one new file in [`doc/history/`](../history/README.md)**, named
  `<session>-<slug>.md`, and nowhere else, and it states the round's gates in a paragraph opening
  `**Gates.**` with each one's exit status or pass count (ADR 1499). **Created, not appended to**: a round adds a file that
  did not exist, and edits no other round's — not `doc/history.md`, whose table is closed at 445,
  and not the neighbouring file. A number, a date or a session reference in any other document is
  bookkeeping and belongs in that file; a citation of an ADR for an *argument* is a pointer and
  stays where it is. (ADR 0281.)
- `doc/HANDOVER.md`, `doc/state-of-play.md`, the `doc/traps/` group the round was in,
  `doc/todo/README.md` and this file: only if what they *claim* stopped being true. None of them
  holds a number, so a round that only moved numbers writes nothing here. **A new trap goes in the
  group whose rounds would spring it**, keeps the next free number, and gains **two** entries in
  `doc/HANDOVER.md`: a row in the index *and* its number in that group's row of the table above it.
  Traps 14, 30 and 31 each reached the index and not the table, so a round opening the group it was
  told to open read one trap short of what the file holds — the numbers are consecutive across the
  five files, not inside one, which is why neither entry can be derived from the other by eye.
- The todo file: delete it if the item is done, correct it if the round changed what it owes.

## 6a. A question for the owner is a file, not a sentence in a report

Stated by the owner. A round that needs the owner's word writes it into
[`doc/questions/`](../questions/) as a `Q` file, **in the same commit as the work that raised
it** — a question that lives only in a history file or an ADR is a question nobody can find.
[`doc/questions/README.md`](../questions/README.md) owns the convention and the four things every
`Q` file says; the one rounds forget is **what the tree does meanwhile**, and naming it is what
keeps a question from reading like a blocker when it is not.

An answer is an `A` file of the same name whose three-line header — `Status`, `Given`,
`Owes` — is the index, because an `A` file means **the owner has spoken**, not that the
question is settled: answers can defer (`A03`), dissolve the question (`A19`, `A49`),
answer half of it (`A51`), or be replaced by a later one (`A15`, `A17` → `A46`). A round
transcribing an answer given in conversation writes the header and quotes the owner
verbatim, and never paraphrases the round's own reading in the owner's voice.
`tools/state.sh questions` prints the whole index; the same `README.md` owns the rest.

## 7. Three habits these rounds added, which belong here rather than in a trap

- **A closed form taken from one renderer is not a limit.** `doc/todo/00`'s step 6 climbs a
  reference to eight times the resolution because its departure from the geometry shrinks with
  the pixels — and on a tiling pattern `poppler` goes the other way, its strokes thinning rather
  than its edges sharpening. Take two ladders: one cannot tell convergence from drift, and two
  also say when *neither* has converged.
- **A count that improves is not a picture.** One round took the corpus's incomplete list down
  by two and both documents were still wrong — one of them blank.
  Trap 1's oldest sentence, and the second finding was three steps beyond the first, in a
  function neither document was about. **The inverse holds too**: a count that does *not* move is
  not evidence that nothing happened, which is what a round finding a defect no corpus document
  carries discovers.
- **A round that changes what gets drawn re-runs `doc/todo/00`'s step 7.** Our ink minus the
  lightest reference's, over **every ambiguous page** — the gate's own output, not
  `ambiguous_undiagnosed.txt`, or diagnosing a population would take its pages off the one
  instrument that sees content this tree is *not drawing*. Three minutes, from artefacts already
  on disk. Drop a reference whose ink is zero first, and read the result beside the corpus's
  incomplete list: a page this tree reports is expected to be light. Its first defect was a text annotation attached to a point, drawn as
  nothing — on a page the ranking rated harmless because a nearly blank page resembles a nearly
  blank page.

## 8. A batch of rounds, and the merge that follows it

This is the loop an orchestrating session runs; a single round reads §1–§7 and stops. It exists
here because a session that resumes after a quota should find the loop in the tree, not in the
memory of the session the quota ended. `tools/batch.sh` is the command; this is the reason.

1. **Open one worktree for the whole batch**: `tools/batch.sh open batch-<first>-<last>`. Six
   rounds share it on one short-lived branch, gitignored data symlinked in, and every submodule
   pinned `--skip-worktree` so no `git add` can stage a symlink over a gitlink. `open` also starts
   the batch's first `dev` build in the worktree's own build directory, detached, into
   `scratchpad/open/build.log`: write the briefs while it runs, and the rounds find it warm rather
   than six of them meeting it cold behind one lock (ADR 1451). It then starts `tools/batch.sh
   arms`, detached, into `scratchpad/open/arms.log`: HEAD's six corpus arms, digest by page, into
   `/home/AI/arms-<first>/`, which a pixels round compares against and never exports (ADR 1650).
2. **Brief each round with the ledger rows it must close — by number, never by topic — from
   `doc/todo/_brief-template.md`, and from nothing else.** A round briefed "work on partial rows"
   writes prose; one briefed "close §8.4.5 and §9.9.1" writes code. Over 58 sessions of topic
   briefs, one row of 875 changed status and it went backwards; over the first 24 rounds of row
   briefs, 27 did (ADR 1036, `doc/reviews/1012`). The template is the fixed shape: a common part
   that points at section 0 above and at the every-round files rather than restating them, and a
   per-slot part that carries the contract, the owned files, the premise's evidence and the slot's
   reading pointer. The last batch's lessons are not a paragraph in the brief: a lesson that
   changes how a round works is a line in section 0, the environment rule block or a habit, made
   once, and the brief points there (ADR 1638).
   Take from both denominators: four slots on ledger rows, one on what the corpus
   names, one on instruments — and a sweep's count is not a finding until ten of its hits have
   been read against the standard.
3. **Verify each report against the standard before believing it.** `grep -n` the quoted sentence
   in `doc/md/`, and find its enclosing `##` heading. Rounds have cited a real sentence under the
   wrong clause, named a file that does not exist, and attributed their own lint errors to a
   sibling; the merge is where that is caught.
4. **When all are in**: `cargo fmt --all` (now safe — nobody is editing), tier 1 in full, then
   `tools/batch.sh gates` for tiers 2 and 3 (one line per gate in the log with its `wall` and its
   `wait` on the lock, a failure's tail beside it; `tools/state.sh gates-cost` prints which gate is
   dear). The log's last line, and one line per round with the duration and tool uses its
   notification reported, go into the batch commit's body — the one place a batch's clock is kept,
   and `tools/state.sh batches` is what reads it back (ADR 1476): one line per commit carrying a
   paragraph opening `Round durations`, with the figure `N of M green, T s of gate wall time`, the
   sum of that paragraph's figures written `<n> s`, how many figures it summed and how many rounds
   the paragraph names. **The paragraph's exact shape**, one `;`-separated entry per round opening
   with its session number:

   ```text
   Round durations (wall, tool uses): 1327 4736 s, 161; 1328 1094 s + 2366 s, 17 + 59 (cut and resumed); 1329 9501 s, 219.
   ```

   Every wall-clock figure is `<n> s` and no other figure in the paragraph is; a cut round is two
   figures, `<n> s + <n> s`, each with its unit, so the line reads one more figure than rounds and
   says why. A round entry with no `<n> s`, or a `<n> s + <m>` whose second half has no unit, is
   printed by the line as not summed rather than quietly left out of the sum (ADR 1500). The gates
   line goes in as the log printed it. A moved ratchet is moved *with its reason above the constant*; a bare
   count that can only rise is replaced by a named population (`REFUSED_OPEN` is the shape).
5. **Commit in the worktree, then fast-forward `main` — from the main checkout,
   `git merge --ff-only <branch>`, never from inside the
   worktree.** **The commit, the fast-forward and the close are three commands, run separately,
   each one's output read before the next is typed** — never chained with `;` or `&&`. The commit
   is `tools/batch.sh commit <message-file>` (the file under `scratchpad/` or outside the tree): it
   stages by name every path `git status` reports outside `scratchpad/`, prints the population's
   count against the index's, and refuses to commit when they differ. `close` refuses a worktree
   holding anything uncommitted outside `scratchpad/` and has no `--force` (ADR 1313). Run inside the worktree, that command merges the branch into itself, exits 0, and
   prints the branch's own HEAD where a reader expects `main`'s; one batch's merge
   did exactly that and then closed the batch, deleting the only ref to its commit (recovered from
   the object store because nothing had run `gc`). `tools/batch.sh close` now refuses a branch
   `main` lacks commits from, and refuses to run from inside the worktree — but the order is the
   rule, the guard is the net. Leave nothing uncommitted on `main` while a batch is open: an
   uncommitted file there refuses the fast-forward. A fast-forward makes `main` byte-identical to
   the tree the gates ran on, so no second sequence is owed. Check `git show --raw HEAD | grep -E
   '^:1[26]0000'` prints nothing, and `git log --oneline -1` on `main` names the batch commit.
   Then `tools/batch.sh install`, from the main checkout and as its own command: it builds what a
   person runs from the commit `main` now names and installs it into the main checkout's `target/`
   with `target/installed-from` beside it (section 5, ADR 1511); its last line names the commit.
   Then `tools/batch.sh close`, from the main checkout. Its last lines, and the first lines of
   the next `open`, name the batch directory's `debug` and the `rm -rf` that prunes it when it is
   over `doc/environment.md`'s hundred-gigabyte rule; the prune is run between the two, never
   after `open`, whose warm build writes there (ADR 1526).
6. **Commit only, never push** (owner, 2026-09-07). Then the next batch.
7. **When a quota kills a batch mid-flight**, the notification's last visible line ("I'll start by
   reading…") is the round's *first* message, not its last act. One batch's six rounds
   died with 967 insertions across 21 files, a finished record, an ADR and an
   unbuildable crate in the worktree. Before relaunching: `git -C /home/AI/pdf-viewer-rounds
   status --short` and `cargo check --workspace --all-targets`; and `ps -eo pid,etime,args | grep -E
   'xargs|bounded.sh|examples/'` for **launchers the dead process left running** — three `xargs`
   feeders of a crawl census outlived the 2026-09-15 crash and kept starting a pre-fix binary
   that leaked a zombie per document until the scope's `pids.max` refused every fork; stop them by
   pid before anything is resumed. **A launch can fire more agents than it was briefed** — batch
   sixteen (2026-09-16) got ten where six were briefed, two per contract writing the same files:
   after every launch, `ListAgents` and confirm the count equals the slots; on a surplus, freeze
   every suspect by message (stop writing, report round + `git status` + complete-or-mid-edit),
   keep the one that is ahead per contract, stand the other down, and never let two edit
   `ledger.toml` at once. If the harness reports the
   rounds as *stopped* with their ids and transcripts saved (batch fifteen, 2026-09-15), resume
   each by message rather than by a new brief — "the process exited; your files on disk are …;
   check `git diff` on your paths, rebuild, finish the contract as briefed" — because a resumed
   round keeps its whole context and a relaunched one has to re-read its predecessor. Only where
   a resume is impossible, relaunch the *same*
   contracts, and message each new round the list of its predecessor's files — "you own it:
   read it, keep what is right, finish or revert" — naming the crate that does not build and
   which round owns it. Each resumed round read its predecessor against the clause and found
   real defects in the draft (a decoded-to-empty value, a wrong media type, a struck sentence
   quoted as current), which is why the handover says *read*, not *continue*.
8. **When a worktree is lost with work in it**, it is rebuilt from the rounds' transcripts, not
   from memory. The harness keeps every subagent's transcript as JSONL — `tasks/<id>.output` under
   the orchestrating session's temporary directory links to `subagents/agent-<id>.jsonl` under its
   project directory. Recreate the worktree at the last commit, extract every `tool_use` input
   (`Write` contents, `Edit` replacements, `Bash` commands carrying heredocs, python splices or
   `sed -i`) from all six transcripts, sort them by timestamp, and replay the file-mutating ones in
   that order; builds, tests and reads are skipped. Then each round, resumed by message, verifies
   its own files read-only against its report before anything is committed. Batch thirty-five's
   128 files came back this way (ADR 1313).

Where the closable rows are is a question for the ledger, not this file:
`cargo run -p conformance --bin ledger` prints the status counts and writes nothing, and `doc/todo/01` is the reading
list. Only `--bin ledger -- --write` regenerates the file, and only a round editing the ledger runs
it, on a tree nobody else is editing; every `tools/state.sh` section is read-only (ADR 1487). Open questions for the owner are `doc/questions/Q*.md` with no `A` beside them (§6a).
