# 801 — Two refusals that outlived their reasons

A page refused at one magnification stayed refused at every other, and a worker that died was read
as the end of the document; both are refusals recorded against something larger than what was
refused.

Date: 2026-08-28. ADR: [0734](../adr/0734-two-refusals-that-outlived-their-reasons.md).
Subject: `doc/todo/15`'s remainder after ADRs 0725 and 0729.

Touched: `crates/viewer-confined/src/{lib.rs,resume.rs}`,
`crates/viewer-ui/src/bin/pdf-viewer-confined.rs`,
`crates/viewer-ui/src/bin/pdf-viewer-confined/screen.rs`,
`doc/todo/15-ship-the-confinement.md`, and the two documents above.

## What was taken, and why these two together

`doc/todo/15` listed three things owed after ADR 0729 and one of them is the owner's session. Of
the two that are a round's, both turned out to be the same defect at two scales — a refusal
recorded against something larger than the question that earned it — so they were taken as one
change with one argument. The third, moving the three established windows onto the boundary, was
judged too large for a coherent first piece this round: it needs an abstraction over "a viewer
here" and "a viewer over there" across fourteen thousand lines of chrome whose `Answer`s borrow
from an in-process `Viewer`, and any half of that is a placeholder path.

## Part 1 — the page

`Content::Refused(String)` had no identity beside it, alone among the variants of its own enum, so
`Screen::take`'s arm fired for any later list payload of that page. A zoom, a resize or a
re-interpretation kept a refusal that was about none of them. The commonest way to earn one is
`render-cpu` refusing a target over `pdf_render::MAX_PIXELS`, which is exactly the refusal the
*next magnification down* does not earn — so one `+` too many cost the page permanently, at every
magnification, for as long as the document stayed open. Present since ADR 0713 and inherited by
ADR 0725's device arm; observed in ADR 0725's own round and deliberately not folded into 795.

The variant now carries the list and the target, and the arm is guarded by them in both arms.

## Part 2 — the worker

`doc/todo/15` recorded the breach an allocation budget cannot see as owed "as a refusal", with the
reason it had not been done: *making it a refusal needs a fallible allocation on a path this crate
does not own*. **That sentence is true and it answers a different question.** It is about a refusal
the *worker* makes. What the reader needs is a refusal of the *page* — and inside the confinement
that costs nothing, because a worker's death leaves nothing of the document behind it, the file is
on this side by rule 2, and the command that killed it need not be sent again.

`viewer_confined::Resuming` decides which errors are worth another worker (only `WorkerDied`, with
every other arm refused for a stated reason and a test that walks the enum), how many in a row
(`RESTARTS`, three, put back by every frame that reaches the screen) and what a resume goes back to
(the last page a frame arrived for, which is by construction not the page that killed it).
`pdf-viewer-confined` does the starting, at the loop's turn rather than where the death was seen,
so no restart runs inside another one.

**Observed, not changed.** `ConfinedError` is `#[non_exhaustive]` and carries no reason for it,
while `doc/ui-boundary.md` says of the vocabulary beside it: *"Nothing is `#[non_exhaustive]`,
deliberately: it forces a catch-all arm on every host, and a catch-all arm is where a message added
later goes to be ignored in silence."* That rule is about messages rather than errors, so this is a
tension rather than a defect — and it is answered in practice here without touching the attribute:
`Resuming::after` matches the enum **inside its own crate**, where `#[non_exhaustive]` does not
apply, so the wildcard-free match is possible there and no host needs a catch-all at all. A round
that wants the attribute justified or removed has the sentence to start from.

The honest limit is named rather than discovered: the magnification and the position on the page
are **not** restored, because nothing on this boundary asks the viewer what they are and the
commands that set them are relative and clamped. ADR 0734 has the argument and `doc/todo/15` now
carries the exact restore — a `Query::View` on the boundary — as the next piece.

## Proof, under Xvfb on the release programs (llvmpipe; illustration, not a gate)

`PDF20_AN001-BPC.pdf` on a 900×1100 virtual display, 800×1000 window. The instrument for a death
is `kill -9` on the worker's pid, which from the host's side is exactly what a ceiling breach is:
the worker's output closes and `read_exact` returns `UnexpectedEof`.

| | device path | `--cpu` |
|---|---|---|
| title before the kill | *page 2 (3 of 5)* | *page 2 (3 of 5)* |
| title after the restart | *page 2 (3 of 5)* | *page 2 (3 of 5)* |
| the window's pixels, before against after | **identical** (`magick compare -metric AE` → 0) | **identical** (0) |
| second worker started and confined in | 6.9 ms | 4.1 ms |
| and it still turns pages | *page 3 (4 of 5)* | *page 3 (4 of 5)* |

The sentence on standard error names all four things it has to: what happened, where it is going
back to, which attempt it is, and what is *not* restored. The window's title says it is restarting
while it does, and goes back to the page and label when the second worker answers.

**The documented cost, photographed rather than asserted.** The same run with two zoom steps before
the kill: the restart comes back on the right page at the *opening* magnification, 11.1% of the
window's pixels different from what was there. That is the limit above, and it is what the sentence
warns about.

No zombie beside the window after either run, and `q` exits. The two start times are wall clock on
a machine carrying three sibling rounds' builds, so they are an upper bound rather than a
measurement; an earlier run of the same script on the same tree with the machine quieter gave
1.3 ms and 1.5 ms. What the row is for is the order of magnitude — a restart is milliseconds, and
that is what makes it the right answer to a death at all.

## Trap-13 calibration

Every new test was run against an injected defect before being believed; all ten failed, and the
suite is green as committed.

| injected defect | failed |
|---|---|
| the processor arm's refusal guard removed (ADR 0713's arm restored) | `a_refusal_is_kept_for_its_own_drawing_and_not_for_the_page` |
| the device arm's refusal guard removed | `a_device_screens_refusal_is_kept_for_its_own_drawing_too` |
| the budget not consulted (`spent < RESTARTS` dropped) | `a_dead_worker_is_started_again_until_the_budget_is_gone` |
| `showing` does not put the budget back | `a_frame_that_reached_the_screen_gives_the_budget_back` |
| `showing` does not record the page | `a_resume_returns_to_the_last_page_that_answered` |
| `Connection` resumed as though it were a death | `only_a_dead_worker_is_worth_another_one` |
| `Refused` resumed as though it were a death | `a_refusal_the_worker_survived_is_not_restarted_from` |
| `Cancelled` resumed as though it were a death | `the_readers_abort_is_not_undone_by_a_restart` |
| `died` always stops (the behaviour this replaces) | `a_dead_worker_leaves_a_restart_owed_at_the_readers_page` |
| `reopen`'s stopped-window guard removed | `an_abort_between_the_death_and_the_restart_wins` |

## Gates

Run last, after the final edit, as `doc/todo/02-every-round.md` §2's change→gate map assigns for a
change in `viewer-confined` and `viewer-ui` plus documents: the four core lines, the conformance
gate, and the quotation and pointer binaries. Not a fifth round (`tools/round.sh`), and nothing
here can move a pixel of a rasteriser — the two crates are hosts, and no gate rasterises with
them.

| | |
|---|---|
| `cargo fmt --all --check` | silent |
| `RUSTFLAGS="-D warnings" cargo clippy --workspace --all-targets` | silent |
| `cargo nextest run --workspace` | 2765 tests run, 2765 passed, 18 skipped, in 59.6 s |
| `cargo test --workspace --doc` | 1 passed, 0 failed |
| `RUSTFLAGS="-D warnings" cargo check` over `fuzz/` | silent |
| `cargo test -p conformance` | 200 passed, 0 failed |
| `--bin quotations` | 6707 quotations over 1082 documents; 2801 verbatim, **38 diverging — the same 38** |
| `--bin pointers` | 9030 path pointers, 5127 live; **98 absent and 13 undefined symbols, both unchanged** |

The reference-spawning gates (oracle, corpora, text extraction, the censuses, quorra's corpus)
were **not** run, and that is the map's answer rather than an omission: this change cannot reach
what they measure, and §2's own note is that a gate spawning a reference on a loaded machine
measures the load — three sibling rounds were building beside this one throughout.

## §4 sweeps

Fourteen sweep binaries run over the pristine `main` checkout at the branch point and again over
this worktree, and the two outputs diffed. Every delta accounted, and none of them is a finding:

| delta | what it is |
|---|---|
| `pdf-viewer-confined.rs:716` → `:865`, `viewer-confined/src/lib.rs:294` → `:307` | the same two standing hits, moved down the file by lines this round added |
| `overtaken`: 621 → 622 decision records; 48 overtaken unchanged | ADR 0734 exists |
| `pointers`: 9020 → 9030, live 5118 → 5127, a form 196 → 197; **absent 98 and undefined 13 unchanged** | the two new documents' own pointers resolve |
| `quotations`: 6703 → 6707 over 1080 → 1082 documents; **verbatim 2801 and diverging 38 both unchanged** | four phrases this round quotes are this project's own sentences, not the standard's |
| `counts`: 8792 → 8801 governing sentences; 441 attributed counts unchanged | new prose |
| every `named by N file(s)` rung up by exactly one | `crates/viewer-confined/src/resume.rs` |

One delta is the **instrument** rather than the tree and is worth writing down: `pointers` resolves
`tmp/hayro/hayro-jbig2/src/file.rs` in the main checkout and answers *no file of that name* in the
worktree, because `tmp/` is gitignored and `tools/worktree.sh` does not link it. A worktree round
reading that sweep sees three absences a merge does not.

## Contradictions with the briefing

- The briefing named `Content::Refused` outliving a zoom as "pre-existing since ADR 0713, inherited
  by the fallback path", which the tree confirms exactly.
- The briefing recorded breach-as-refusal as owed and `doc/todo/15` said it needs a fallible
  allocation this crate does not own. **The tree wins and the entry was answering the worker's
  question rather than the reader's**; ADR 0734 §2 has the argument, and the fallible-allocation
  sentence is still true of the worker-side refusal, which is not what was built.
- `tools/round.sh` reports the next session as 799 because `doc/history/` ends at 798; three
  sibling rounds are in flight and none of them has written its file yet. Nothing was read from it
  but the fifth-round question, which is answered the same way for 799 and 801.
