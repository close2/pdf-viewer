# 759 — The frame a toolkit was inside when page one finished

The quiet-machine launch A/B `doc/todo/30` said session 754 owed, taken first and on a machine at
load 1.5 to 2.8 — and it found a real regression rather than the 2 ms ADR 0668 §7 predicted.
`viewer_host::Drawing::settle` is the answer. ADR 0678. Date 2026-08-25.

## The measurement, and what it said

`opened` → `first frame on the screen`, two stamps inside one process, twenty alternating pairs a
host, `doc/PDF20_AN001-BPC.pdf` under `Xvfb`, two release builds differing only in `d1ecef4d` —
754's commit against its parent, so the column is that change and nothing else.

- **`viewer-gtk`: 9.5 ms before, 53.4 after.** Twenty runs an arm and the ranges do not overlap.
  About 40% of a 110 ms launch, which `CLAUDE.md` section 2 names a regression in its own words.
- **`viewer-qt`: 10.6 before, 7.9 after** — no regression, and the *faster* arm.
- Repeated on ISO 32000-2's 1023 pages: **18.0 before, 56.9 after**, so it is not the document's.

## Where it was

One trace line: `page 1 rasterised … in 3.252476ms, waited 61.528955ms`. The answer was ready in
three milliseconds and could not be collected for sixty, because `Drawing::POLL`'s one-shot is
dispatched when GTK's main loop comes back round and at launch the loop is inside its own first
frame — GSK's renderer bring-up, which under `Xvfb`'s software Vulkan holds it for most of a
sixteenth of a second. `GSK_RENDERER=cairo` is the control: the same wait falls to 11.3–12.5 ms.

**Qt's faster number is the worse picture, and that is how the fault was located.** Its
`/OneColumn` first frame in the 754 arm carries `2168976` bytes where every other arm carries
`4337952` — exactly half, one of the two pages Table 29 shows. Two hosts sharing one arrangement
and disagreeing is what says the disagreement is the toolkit's.

## What was built

`viewer_host::Drawing::settle(budget)` — `collect` with a `recv_timeout` in front of it, called by
a host **only while it has put no frame on the screen** and by nothing else. Three lines in each
host, identical, because the rule is `viewer-host`'s.

- It is **not a deadline**: nothing is interrupted, a page that outlasts the budget stays in flight
  and arrives through the poll as before, and ADR 0657's refusal of an automatic deadline stands.
- The budget is the **launch's** rather than the call's, accounted inside `Drawing` as time
  actually blocked — a column asks for two or three pages before its first frame, and a thousand-page
  document's thirty-millisecond open must not eat it.
- The bound is **one 60 Hz refresh**, on ADR 0657's census: 93.9% of `doc/pdf.js`'s first pages draw
  inside one at twice device scale.

## What it bought

`viewer-gtk` 9.9 ms against 9.8 before 754 and 52.9 with the regression; `viewer-qt` 11.7, its first
frame carrying `4337952` bytes again. The frame line reads `waited 3.935297ms` for a `3.823736ms`
draw — 0.11 ms of overhead, which is what "a thread and a channel" predicted once the loop was asked
at a moment it could answer. On the 1023-page document, page one waits 6.78 ms for a 6.70 ms draw.

754's own behaviour re-checked rather than assumed: the amplification fixture with three XTEST
zoom-ins during the first draw still abandons all three and still takes every key.

## Instruments left behind

In the scratchpad, named for the round: `launch-759.sh` (the alternating A/B, which waits for the
line rather than for a timeout, so forty runs cost under a second) and two `git worktree`s of
`3f4ee908` and `d1ecef4d` with `.cargo/config.toml` target directories of their own.

**A trap worth the words**: the worktrees inherited `/home/AI/.cargo/config.toml`'s `target-dir`, so
both arms and the main tree were building into **one** directory until the second build printed
`Blocking waiting for file lock`. An A/B whose two arms share a build directory measures whichever
linked last. `cargo metadata --no-deps | jq -r .target_directory` is the check, and it is §5's own
rule one directory over.

## Gates

The core — `fmt`, `clippy --workspace --all-targets` under `RUSTFLAGS="-D warnings"`, `nextest`,
doctests, the `fuzz/` check — plus `cargo test -p conformance`, by §2's map: the change is confined
to `viewer-host`, `viewer-gtk` and `viewer-qt`, none of which any gate rasterises with. §5's
binaries were rebuilt and installed, before the measurements and again after the fix.

## Ledger

Untouched, and for ADR 0668's reason: `CLAUDE.md` principle 2 against principle 3, citing no clause.
The clauses the change's comments name — §7.7.2's `/PageLayout` and Table 29 — decide how many pages
a launch asks for and their behaviour did not move.
