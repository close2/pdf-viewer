# 629 — The merge that had to decide something

Third merge round, five branches, and the first with conflicts that were not textual. 618 found
nothing, 623 found a claim that did not survive and mis-attributed it, and this one had to make two
decisions that neither branch could have made alone — which is the clearest statement yet of what
this round is for.

## What was merged

`round-624`, `round-625`, `round-626`, `round-627`, `round-628`, branched from `1504d839`.

## Two conflicts, and what decided each

**`doc/todo/03` — two rounds each wrote a section 20.** 624's is the check mechanism a chunk leaves
behind; 625's is the chunk it took. Both survive: 624's keeps the number, because `doc/todo/02` §2
now cites it, and 625's becomes §21, which is where a chunk section belongs (§16–§19 are the four
before it). **Neither was a candidate for dropping** — one is a rule and the other is evidence.

**`crates/viewer-ui/src/bin/pdf-viewer/surface.rs` — the real one.** 627 moved the processor's
composition into a new private module `composer`; 628, from the same base, added to it *in place*
and gave `software` a `&self`. Textually the region was a pure addition and git took it; semantically
two rounds had restructured one path. Two edits closed it:

- `Self::software(window)` → `self.software(window)`, mechanical rather than a judgement: `self` is
  used two lines above it and 628 is the round that made `software` a method;
- and 628's free `fn on_the_processor` was **deleted**, because 627's `composer` owns that role now
  — established by finding `viewer_ui::software::compose_pages` reached from `composer::put_up`,
  not by inference from the warning.

**The warning is the point.** `-D warnings` — which 614 put on §2's clippy line four rounds ago, and
which was silent locally for this project's whole history before that — is what named the orphan.
A merge that dropped a superseded function silently, or kept a dead one, would have compiled either
way without it.

## The sequence, whole, on a quiet machine

Run after the last edit, load average 1.8–2.7, **nothing else building** — which matters, because
626 established that a loaded machine starves the oracle's reference renderers and makes the gate
report red for a reason not in the tree.

| | |
|---|---|
| `fmt --check` | clean |
| `clippy --workspace --all-targets` under `RUSTFLAGS="-D warnings"` | silent |
| `nextest --workspace` | **2321 passed, 17 skipped** |
| doctests, `-p conformance` | clean (157 + 5 + 1) |
| corpus | 974 documents, **68 incomplete** |
| oracle | 1794 pages — **907 agrees, 66 contradicted, 786 ambiguous** |
| `render-quorra` | 957 pages — 932 agree, 23 differ, 2 refused |
| **`fixed_documents`** | **25 checked, 0 absent, 25 rows** |
| text extraction | 98.26%, 486 of 508 documents |
| selection, accessibility, dates, XMP, JPEG 2000 | clean |
| `cargo deny check` | advisories, bans, licenses, sources **all ok** |

Every verdict identical to what the five branches measured separately.

**624's check ran in the round it was built for**, which is the whole of its argument: a document no
gate covers is measured once, by the round that fixes it, in a tree that does not yet hold its
neighbours' work — so the merge is the only place the combination exists. Twenty-five documents from
sessions 603, 613, 615, 619 and 621, and this is the first time any of them has been asked in a tree
containing all five.

## The measurement 627 could not take

That round's launch check was defeated by this arrangement: three rounds' gates ran beside it at load
59–76 on 24 cores, and the 1023-page document measured 631–930 ms against the 120–170 ms band the
quiet rounds established. Its A/B pair still said the change did not move it; it could not say where
the level was.

Taken here, quiet, release binaries, `doc/ISO_32000-2_sponsored_EC3.pdf` under Xvfb:

```
first present   159.6 / 135.1 / 134.1 ms
```

**Inside the band.** And 628's new milestone is legible beside it: `surface configured` at
70.2–78.9 ms, costing 25.1–26.3, which is the grounding present that closed the owner's crash — paid
out of page one's own first acquire rather than added to it.

## What this round establishes about the arrangement

Three merge rounds in, the parallel model has cost three things and each has a cheap rule now:
`git add -A` destroying submodule gitlinks (618), a shell's working directory drifting into a
neighbour's worktree (618), and a loaded machine making both a gate and a measurement lie (626, and
627's launch run above). What it has bought is four to five rounds' work in the time one used to
take, and **two of the last ten findings came from one round's work meeting another's** — 623's
false attribution and this round's orphan. Neither is visible from inside a branch.

## Owed

- **The owner's own session.** 628's fix is verified here under Xvfb and lavapipe, 60 launches with
  no configure failure against 5 aborts in 150 before it — but the crash was on a real RADV
  swapchain under their compositor, and that is theirs to confirm.
- **CI's two remaining jobs**, both now diagnosed: `test` fails at *link* with
  `undefined reference to 'cxx_qt_init_crate_viewer_qt'` (the runner's `cxx-qt`, not ours — `check`
  passes because clippy does not link), and `nightly` exceeds the one-hour ceiling 614 added, which
  is the 42 unexplained minutes of Miri that round recorded rather than guessed at.
