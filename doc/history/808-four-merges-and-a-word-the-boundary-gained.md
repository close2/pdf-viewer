# 808 — Four merges, and a word the boundary gained

Merge round, on `main` from `5efead7b`. Merged `round-804` (`2a4ee90a`, two commits), `round-805`
(`d6b1f077`), `round-806` (`04b70ce7`) and `round-807` (`4868f3c6`, two commits), in round order,
each with `--no-ff`. **All four clean — no conflict in any file.** `doc/conformance/ledger.toml`
was the only path two branches both touched (804's five rows in clauses 12 and 14, 806's one in
§10.7.4); `ort` reconciled them and the merged file carries all six edits, which the diff against
the base confirms row by row and the conformance gate then passed on.

The briefing named 804's tip as `3ee4cdb4`; the tree said `2a4ee90a`, and the tree wins. The branch
was two commits as briefed and its content is what the briefing described.

Then the full §2 sequence **as the merged tree defines it** — 807 having added a fifth and sixth
core line to that sequence — one documentation correction this round owns, §5's install, the §4
sweeps against a pre-merge baseline, and the four worktrees closed.

## The contact the briefing warned about, and how it resolved

**805's vocabulary change and 804's `view.rs` work never touch.** 805 is `viewer-core`,
`viewer-confined`, `viewer-ffi` and `viewer-ui`; 804 is `pdf-model`'s `view.rs` and a new example.
No file is in both diffs, and the one place they could have met is real but one-directional:
`pdfv_field_name` in `crates/viewer-ffi/src/abi.rs` reaches `widgets_by_field_name` through
`viewer-core`, and 805 **added** two entry points (`pdfv_view`, `pdfv_set_view`) without touching
that one — checked by diffing the added `pub unsafe extern "C"` lines of 805's `abi.rs` change.
So 804's new behaviour — a dictionary with no `/T` anywhere in its ancestry is a widget rather than
a field, and a `/Fields` entry with a `/Parent` takes its ancestors' name — flows out through the
C ABI unchanged, and needed no reconciliation. Nothing was resolved by argument because nothing
had to be.

## The ABI and the wire, verified rather than assumed

- **`PDFV_ABI_VERSION` is `1u` and `PDFV_EVENT_KIND_COUNT` is `16u`** in
  `crates/viewer-ffi/include/pdf_viewer.h`, byte-identical to the same two lines at `5efead7b`.
  That is what 805 intended: a function *added* is a symbol an old caller never looks up, and no
  event kind was added.
- **The header's numbers are the library's**, proved rather than read:
  `viewer-ffi::header_and_library_agree::every_constant_in_the_header_is_the_number_the_library_gives_it`
  passes on the merged tree, as does
  `every_entry_point_is_declared_once_in_the_header_and_nowhere_else`.
- **The C consumer compiles and runs.**
  `viewer-ffi::a_c_program_drives_the_abi::a_c_program_opens_a_document_turns_a_page_asks_a_query_and_gets_pixels`
  passes — `crates/viewer-ffi/c/open_a_page.c`, which 805 extended by 52 lines to drive
  `pdfv_view`/`pdfv_set_view`, built against the crate's own header with `-Wall -Wextra -Werror`
  and linked against the `cdylib`. This is the one gate in the sequence that runs a C compiler, and
  it is the only instrument that can see a C-side ABI break at all.
- **The wire greeting reads `PDFVCF05` exactly once, as the current value.** `grep` over the whole
  tree finds one `const MAGIC: &[u8; 8]`, in `crates/viewer-confined/src/protocol.rs`, at
  `PDFVCF05`; every other occurrence of a `PDFVCF0*` string is a document narrating the history of
  the constant. `fuzz/seed_confined_wire.py` needed no edit and could not go stale: it reads the
  value out of `protocol.rs` with a regex and asserts the greeting against what it read — which is
  the repair the seven-hundred-and-thirty-sixth session made after the previous bump caught it one
  behind.
- **Every host still matches its closed enums exhaustively**, and the evidence is a compiler rather
  than a test: a non-exhaustive `match` on a closed enum is `E0004`, a hard error, and
  `RUSTFLAGS="-D warnings" cargo clippy --workspace --all-targets` completed with exit 0 over
  `viewer-gtk`, `viewer-qt`, `viewer-ui`, `viewer-host`, `viewer-ffi` and `viewer-confined`
  together with all their test targets. Beside it, 805's own instruments pass:
  `every_query_reaches_the_abi::the_samples_cover_the_whole_enumeration` and
  `every_query_variant_names_at_least_one_entry_point` (so `Query::View` reaches the C boundary),
  `viewer-confined::protocol::tests::a_view_crosses_and_comes_back_unchanged` (so the new greeting
  carries the new answer), and `viewer-core::headless`'s
  `a_view_answered_is_the_view_restored_exactly`,
  `a_view_is_not_what_a_host_asked_for_and_that_is_why_it_is_a_question` and
  `a_restored_view_announces_the_page_it_moved_to`.
- **`fuzz/`'s `confined_wire` target still compiles against the moved boundary** — the sixth core
  line, `RUSTFLAGS="-D warnings" cargo check --manifest-path fuzz/Cargo.toml --bins`, clean. It is
  the target a boundary change breaks and the one no parser round would think to run.

## Gitlink verification

`git ls-files -s doc/pdf.js doc/arlington-pdf-model` read `160000` for both paths before every
commit this round made, and reads `160000` now. The four merges staged themselves; the two
document commits named their paths. No `git add -A`, no `git add -u`, no `git stash`.
`cargo test -p conformance`'s `every_declared_submodule_is_still_tracked_as_one` passed on the
merged tree.

## ADR verification

`main` ended at **0735** pre-merge; the batch brought **0736** (804), **0737** (805), **0738**
(806), **0739** (807) — one per round as the briefings reserved, no collision, and nothing at 0740
or above.

## Gates (full §2 sequence on merged `main`, quiet machine, nothing beside it)

| gate | result |
|---|---|
| `cargo fmt --all --check` | clean |
| clippy, `-D warnings`, `--workspace --all-targets` | clean, exit 0 (the only `warning:` lines are gcc's on the cold `cxx-qt` bridge and the `proc-macro-error2` future-incompat note) |
| `cargo nextest run --workspace` | **2782 tests run, 2782 passed, 18 skipped** |
| `cargo test --workspace --doc` | ok, 24 suites |
| **`cargo fmt --manifest-path fuzz/Cargo.toml --check`** (807's new core line) | clean |
| fuzz `check --bins`, `-D warnings` | clean — `confined_wire` compiles against 805's `PDFVCF05` |
| `cargo build --profile gates -p pdf-sandbox --bins` | ok (trap 10) |
| pdf-model corpus | ok — 974 documents in 3.6 s, 0 unopenable, 8 locked, 2 encrypted beyond us, 6 pageless, **66 incomplete**, 0 slow; composition the file 56, neither one 9, this reader 1 |
| `pdfref-hayro` build | ok |
| oracle | ok, exit 0 — 1945 pages in 77.6 s, **983 agree, 61 contradicted, 836 ambiguous**, 3 our geometry, 2 reference geometry, 42 not comparable, 18 no render; 100.0% reference-cache hit rate, 0 renders produced |
| text_extraction (three gates) | ok — pdftotext 99.2% (22834/23013), PDFBox 99.8% (14257/14281) in both orders, position verdict 10971/11163 (98.28%), **487 of 508** documents fully in bounds |
| selection_census | ok — readback differs on 0 of 966, caret 2094 offsets over 459 fields, drag 1000/1011 (98.91%, printed not ratcheted), panicked 0 |
| accessibility_census | ok — 102853 elements, 876 of 876 untagged pages answering the honest empty tree, 0 disagreeing lines, panicked 0; no ratchet moved |
| dates, xmp, jpeg2000 | ok |
| render-quorra corpus | ok — 957 pages in 26.0 s, **932 agree, 22 differ, 3 refused, 17 not comparable** |
| fixed_documents | **41 checked, 0 absent, 41 rows** |
| `cargo test -p conformance` | **208 passed** — 11768 citations, 875 subclauses, 0 `unreviewed` in any clause family, 1107 quotations all verbatim, ledger prose naming 2902 clauses and 282 tables |

**The nextest count is the batch's exact arithmetic**: the base's 2773 plus 804's three, 805's
four, 806's one and 807's one — nine `#[test]` items added across the four diffs, none removed, and
2773 + 9 = 2782. The conformance binary's 207 becomes 208 for exactly one reason: 807's
`tools/conformance/tests/workspaces.rs`, run here on its own as well as in the sequence, and it
passes.

**Every verdict this batch could have moved landed where its round left it.** 804's `view.rs`
change is the only one in the batch that can reach a drawn page, and the corpus, the oracle, the
text gates, quorra and `fixed_documents` all print exactly the figures the briefing named — 66
incomplete, 983/61/836, 487 of 508, 932/22/3/17, 41 rows. 806 changed no drawn output at all: its
`outline.rs` edit is a module comment correcting a caller enumeration, and everything else it
touched is a test. So `doc/todo/00`'s step 7 is not owed by this merge — nothing in the merged tree
draws differently from the trees the four rounds measured.

## §5 — the eight artefacts

Rebuilt in `--release` from the **main tree's** build directory, asked for rather than written
down: `cargo metadata --no-deps --format-version 1 | jq -r .target_directory` printed
`/home/AI/cargo-target/pdf-viewer`, with the shell in `/home/cl/projects/pdf-viewer` — the value
follows the *working directory*, not `--manifest-path`, which is trap 15's shape. One invocation
for the seven binaries, a second for `viewer-ffi`'s `cdylib`; `install -Dm755` for all eight.
`tools/state.sh binaries` shows all eight at this round's timestamp. (`target/safedocs` is older
and is not one of the eight.)

`tools/round.sh` will report `target/pdf-viewer is older than HEAD` afterwards and be right without
anything being wrong: the commits after the install are this round's own documents.

## §4 — the sweeps, against a pre-merge baseline

**The before-half was taken in a checkout of its own**, not by checking files out and back.
`git worktree add --detach .claude/baseline-808 5efead7b`, `doc/md/`, `doc/*.pdf` and the two
submodules symlinked in from the main tree, and the sweep binaries **built inside it** with a
target directory of its own — which is the whole point, because a binary follows
`CARGO_MANIFEST_DIR` and one built in `main` would have measured `main` from anywhere. Nothing in
the working tree was touched, so nothing could be restored away; the checkout and its build
directory were removed together afterwards. `doc/todo/01`'s footgun paragraph gained that method
beside the one it already carried — this round's one documentation change.

Twenty sweeps run on both sides and diffed. **Four are byte-identical** — `callers`, `entries`,
`overstated` and `unpriced` — as are errata `moved`. Every remaining delta is accounted:

| sweep | delta | attribution |
|---|---|---|
| `blockers` | three line numbers (`query.rs:331→352`, `pdf-viewer-confined.rs:865→887`, `oracle.rs:12106→12184`); no hit added or retired | 805 and 806 inserting above them |
| `capabilities` | 27 line numbers, summary line **identical** (196 sentences, 161 witnessed, 118/78) | 805's insertions; normalising line numbers makes the two outputs equal |
| `counts` | 8862→8930 sentences, 443→450 attributed counts, **all seven** in the "attributed to a clause with no row" bucket; agreeing stays 150, "no such way" stays 58 | 804's ledger notes and its todo additions |
| `tables` | 6976→7000 sentences, 2584→2596 attributed key citations, **all twelve agreeing**; absent stays 101, denials stay 6 | 804's and 806's notes |
| `quotations` | 6733→6763 quotations in 1089→1097 documents, 2814→2823 verbatim; **diverging still 38 and 2** | the four ADRs and four history files; no new misquotation |
| `owed` | 3966→3974 terms; **222 rows, 182 debts named by no source and 110 clean rows all unchanged**; §8.9.6.2 only moves position in the list | 804's five rewritten notes name more identifiers, all already in the tree |
| `inapplicable` | 312→315 terms, 254→257 named, 244→247 with a cousin; **58 confirmed claims unchanged**. The three new terms are *Collection*, *Issue* and *This*, all under §14.2 | 804's rewritten §14.2 note; all three are shared-vocabulary noise words at 101, 72 and 371 naming files |
| `unread` | one witness list gains `crates/pdf-model/examples/unnamed_field_census.rs` for `/Fields` | 804's new census example |
| `parts` | two line numbers, and one more cardinal in the *dated record* bucket (331→332); **agreeing stays 39** | no part was added to this tree; 805's and 807's document additions |
| `pointers` | 9064→9131 path pointers (+46 live, +11 unrooted, +2 a form, +8 not carried), +2 symbol pointers; **absent stays 98, in another crate stays 23, undefined stays 13** | the batch's new documents — see the caveat below |
| `retired` | `closed form` 228→239 mentions with **corrections still 4**; `Reopen::page` 0→1; `empty name` 19→26 | 806, 805 and 804 respectively; nothing retired is still being quoted as current |
| `quoted` | line numbers only; 237 figures read, 123 confirmed, 101 contradicted — **unchanged** | 806 inserting above the notes |
| `overtaken` | **49 → 46**, the one substantive delta | 806 — see below |
| errata `check` | one line number | 805 |
| errata `applied` | 60371→60637 places read, 914→932 naming an erratum; **212 dropped `#NNN` tokens unchanged** | 804's fourteenth use of the errata rule |

**The two documents this round writes were themselves put through the map.** `doc/todo/01`'s new
paragraph and this file are a documents-only change, so `cargo test -p conformance` ran again after
them (**208 passed**, 0 failed) together with `--bin quotations` and `--bin pointers`, which the map
names for a change that moves a document or a pointer: quotations 6763→6765 with **diverging still
38 and 2**, pointers 9131→9134 with **absent still 98 and undefined still 13**. The one clause this
file quotes verbatim — §12.7.3's *"shall not be considered a field but simply a Widget
annotation"* — landed in the verbatim column rather than the diverging one.

**One caveat on `pointers`, and it is the baseline's rather than the merge's.** Three pointers
report `looked in: tmp/hayro/hayro-jbig2/src/file.rs` on the merged side and `no file of that name`
on the baseline side. `tmp/` is machine-local and untracked, so the fresh checkout had no hayro
tree; the delta is an artefact of how the before-half was taken and not something a branch did.
The counts it affects — `absent` at 98 both ways — did not move.

### The one delta that is a finding, and it is 806 discharging 803's

`overtaken` falls from **49 to 46**, and the three notes that left are exactly the three the
seven-hundred-and-third round filed as owed after ADR 0735 overtook them:
`AMBIGUOUS_EVERYONE_OVER_THE_GEOMETRY`, `AMBIGUOUS_TILING_CELL_CLIP` and `DIFFERS_IN_SHAPE`. 806
read each against the construction that overtook it and cited its own decision, which is the
nineteenth sweep's rule working end to end: a round rewrites a note, cites its ADR in it, and the
sweep goes quiet about it. The decision-record population moved 625→629 in the same run, so the
right-hand side grew while the hits fell.

## Worktrees closed

`tools/worktree.sh close 804 805 806 807` — all four checkouts and all four build directories gone
in one act. `tools/worktree.sh list` afterwards names only `main` and `r809`, a sibling round that
had already branched from this round's merge result, with its gitlink guard on at 6/6.

No `pkill` of any form was run this round, per `doc/environment.md`'s corrected bullet; the two
background builds this round started were waited on by the harness and needed no killing at all.

## What the batch is about, taken together

Four rounds, and each is about a **name that reached the wrong population** — which is the same
family as 803's four and a different member of it each time.

**804 — a dictionary no name reaches.** §12.7.3 says a dictionary with no `/T` "shall not be
considered a field but simply a Widget annotation". This tree keyed every such dictionary under the
*empty* name, so all of them shared one entry in `widgets_by_field_name`: one control stood for all
of them, and a value written to one was written to every one. They are excluded now, and the other
half of the same clause is honoured beside it — a `/Fields` entry that has a `/Parent` takes its
ancestors' name rather than standing alone. A key that means "no name" is not a name, and a map
keyed by one merges things the standard says are distinct.

**805 — a view is a question, and the answer has to go back.** The confined worker can die and be
restarted, and the reader would come back to a page but not to a *view*: `GoTo` and `Zoom` are
absolute and `Scroll` is not, so the third part of where a reader was could be asked for and never
stated. `Query::View`/`Answer::View(Viewing)` asks it and `Command::View(Viewing)` puts it back,
with the host echoing the value rather than composing one — because 16% of `f32` pairs in a device
pixel's range do not survive `have + (want - have)` at all. It cost two consumers a compile error,
two C entry points, a struct passed by value, and `PDFVCF04` → `PDFVCF05`; it also drew the line
ADR 0734 had left as a tension, and 803 had filed as owed — `ConfinedError` is `#[non_exhaustive]`
on purpose because its population is the *kernel's*, and what a host decides about one of them is
not which it is. That owed item is discharged.

**806 — the closed form that was 1% wrong.** A derivation in the oracle's own notes added twenty
tiling rules' area to two borders' and counted 3.18 sq pt of shared edge twice, so the number it
compared against was 1.0% high. It is a test on the real document now rather than a sentence, and
the round also corrected three overtaken notes and an outline module's caller enumeration.
`doc/todo/00` gained the lesson. A closed form is an instrument, and an instrument nobody tests is
a claim.

**807 — the formatting gate that could not see fifteen files.** `cargo fmt --all` means *every
package in this workspace*, and `fuzz/` declares a `[workspace]` of its own — so for as long as
that has been true, fifteen files were outside the only gate that reads them, and two of them had
standing diffs. §2 gained a fifth core line and 807 took the two diffs; more usefully,
`tools/conformance/tests/workspaces.rs` now *derives* the rule from the tree rather than restating
it — every tracked `Cargo.toml`'s governing root must be named by both a fmt line and a compile
line of §2 — so the next workspace this tree grows fails a test instead of going quiet.

The moral the four share, which none of them went looking for: **a name, a key, a greeting and a
glob are all claims about which things are covered**, and in each of the four the claim was wider
or narrower than the population it was applied to. 804's key covered too many, 805's vocabulary too
few, 806's closed form counted one edge twice, and 807's `--all` covered less than the word says.

## Owed, named

- **CI's verdict awaits the owner's push.** `main` is far ahead of `origin/main` and unpushed by
  instruction; `origin/main`'s last run is a pre-existing failure
  (`gh run view 33121581297 --log-failed`). Nothing in this batch was measured against CI.
- **`doc/rfc/` awaits the owner's review** and was not touched.
- **QUORRA_FEEDBACK §40** is still open.
- **`doc/todo/15`'s remainder** — 805 took the view across a restart, and what the file still names
  is `doc/todo/10` §6's four rules, which bind every road.
- **`fuzz/` is still unlinted.** The sequence now formats it and checks that it compiles; putting
  it under `clippy` is a larger decision, because that crate takes no `[lints] workspace = true`
  and has never been under this tree's lint levels. ADR 0739 says why that is a round's subject.
- **A fuzz target's exit status still says nothing about whether it fuzzed** — round 800's `page`
  run, 86,912 iterations at zero coverage, exit 0, for want of seeding. Same shape, same treatment
  owed: a round, not half of one.
- **The owner's `git stash` entry** is still on the stack and still dead; only somebody with the
  permission can `git stash drop` it. `doc/environment.md` says why it is safe to ignore and costly
  to pop.
- **The outline ratio test flakes under sibling load**, seen by 804:
  `pdf-model::outlines::an_outline_resolves_against_the_page_tree_once` asserts a ratio of two
  wall-clock spans and failed once with three neighbours building, then passed alone and passed
  again in a full run. It did not flake here — this round ran the sequence with nothing beside it —
  but the test is the shape §2's quiet-machine paragraph is about, and whether a bound on an
  algorithm should be asserted in wall clock at all is a question nobody has taken.
- **The device path still has no interrupt to offer.** A long draw on the graphics device cannot be
  asked to stop.
- **`pdfv-759-before` / `pdfv-759-after`** remain orphan build directories `tools/worktree.sh list`
  cannot see, its pattern being `pdfv-rNNN`.
