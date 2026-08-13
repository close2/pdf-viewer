# ADR 0313 — A release that changed no line here, and emptied a refusal list

Date: 2026-08-13 (session 478)
Status: accepted

## Context

quorra's `a7babab` is fourteen commits past the `2c9bdd0` this tree pinned, and six of them carry
ADRs: 0034 packs the coverage sheet nearer square, 0035 adds `Device::warm_for`, and 0036 to 0039
size a layer, then a soft mask, then a plan's accumulation, then the **root** to what the plan
marks rather than to the target. The last of those is the one with the number on it: on the corpus
this tree hands them, `issue16287.pdf` at 4× falls from 291 199 104 frame bytes to 6 158 496.

`doc/todo/02-every-round.md` §2 says a round taking a quorra release owes the **second coverage
lane** — `PDFVIEWER_QUORRA_COVERAGE=gpu`, and again with `PDFVIEWER_QUORRA_SCALE=4` — because a
release can be entirely inside a lane the default run does not exercise, which `74c4994d` was
(ADR 0283). All four combinations were run here.

## What the bump required

**Nothing.** `Cargo.lock`'s two hashes and no source change: the workspace builds, `clippy
--workspace --all-targets` is silent, and every test compiles. That is worth stating rather than
passing over, because the two bumps before it each cost a line or a deleted test — `GroupSpec`
gained `isolated` at `89d7dd77` and `compose` at `2c9bdd0`, and `StagedComposeReason` lost a
variant that a test existed to catch. A release that changes what a frame *costs* without touching
what a caller *says* is the shape a dependency should have, and it is the reason this round's
diff is a measurement rather than a port.

One incidental: `cargo update -p quorra-gpu -p quorra-scene` also re-pointed five Windows-only
dependency edges from `windows-sys 0.52.0` to `0.61.2`, both of which were already in the lock.
They are reverted and `cargo build --locked` passes without them, so the commit is two lines.

## What it did, measured here

The four lanes, and the counts rather than the clocks — the machine was running six sessions at a
load average near 40, so every second below is contended and none of them is quoted as a
comparison. `doc/QUORRA_FEEDBACK.md` §22 carries the table and the per-lane detail.

**No page of this corpus is refused for frame bytes at any scale any more.** At 4× on the default
lane the refusals fell from seven to four and on the device lane from eight to four, and what went
in both cases is exactly what was arithmetic against the 256 MiB frame budget. The four that stay
are two kinds, and neither is a budget an allocator can win back: one page holds more *resident
resource* bytes than the 512 MiB default allows, and three exceed the adapter's 16 384 × 16 384
texture for the rasterised-coverage sheet. **The two lanes refuse the same four pages now**, which
they did not at `2c9bdd0`.

## The decision this ADR is for: the refusals at 4× become a ratchet

`crates/render-quorra/tests/corpus.rs` has held a refusal list to equality since ADR 0156 — but
only at the page's own scale, on the default lane. At 4× the same run has always printed
*"the ratchets below are NOT checked"*, and that was right while it was right: twelve refusals at
one revision, seven at the next, most of them a byte count 4 % to 20 % over a budget that upstream
kept improving. A list like that cannot be held to equality, because every release moves it and
every round would re-baseline it — which is a count living in a document, and `CLAUDE.md` says
what that is worth.

**What changed is not the discipline but the population.** With byte-budget refusals at zero, what
is left refuses for a *stated device limit* or for a *cache budget this tree sets*, and neither
moves when a library allocates better. So `REFUSED_AT_FOUR` is held to equality on the default
lane at exactly 4×, and `Ratchets` names the three cases a run can be in rather than answering
yes or no.

Two things it deliberately does **not** do:

- **It does not hold the differing list at 4×.** That list is a property of the coverage quantum
  and shrinks as a page grows; it is a different measurement at every scale and nobody has taken
  one to hold.
- **It does not hold either lane's list on the *other* lane.** Three of these four pages refuse for
  the coverage sheet, and which tiles go into that sheet is decided per command by the lane
  (quorra's ADR 0029) — so the sheet a frame commits is a property of the lane, and a lane's
  refusals are its own. The gpu lane at 4× is still a survey, and it prints that it is.

## `Device::warm_for` is declined again, and now for a second reason

§9.1 answered quorra's first ask with *state the 6 ms, build nothing*: `viewer-ui` learns its
viewport from `Resized`, after the window exists, which is after the point where the saving could
be banked. **That argument is unchanged and this round re-took the measurement rather than
re-reading the paragraph** — the first frame here is `crates/render-quorra/examples/first_frame.rs`,
A/B across the two revisions, eight runs an arm, read at the minimum because the spread is larger
than the effect. Neither the first frame nor the steady frame moves, at either scale, and that is
the expected result rather than a disappointment: a frame with no transparency group allocates no
layer at all, and `doc/todo/42` item 5 has the table.

The release adds a second reason, and it is upstream's own: ADR 0035's headline of 24.7 ms → 10.3
was measured on a page whose root filled its target, and ADR 0039 says in its own *what it cost*
section that this is now about a quarter of layered frames — and only about 8 % of frames are
layered at all. So the hint would warm a texture of the wrong size for most frames that want one
and of no size at all for the rest. It stays free to call and correct to call; there is still no
host here that can.

**The sheet packer is the other half of the same question and the answer is the opposite**, which
is worth the sentence: ADR 0034 is not an API and taking the revision takes it. It is measured on
their side as halving a page's sheet and moving no verdict, and that is what it did here — the
three sheet refusals at 4× are unmoved, because a 16 384-texel limit is a ceiling rather than an
occupancy.

## What it cost

**`Counters::layer_textures` reports a smaller number and means something slightly different.**
Nothing in this tree reads it — `present.rs` takes `commands`, `commands_culled` and
`bytes_uploaded` and nothing else, which is the whole of the code cost. What it cost was two
sentences of prose: `doc/QUORRA_NON_ISOLATED_GROUPS.md` calls the field "how many full-target
internal textures the frame actually allocated" and "the number `max_frame_bytes` is spent on",
and after `a7babab` neither is true — the textures are not full-target, they are one per plan
rather than a pair, one of the ones counted is a transient copy of the backdrop, and a budget is
spent on bytes rather than on a count of things of differing sizes. Both are corrected in place
with the correction dated, rather than rewritten as though they had always said so.

That is the shape this project keeps being caught by and is worth naming once more: **a counter
whose meaning changed while its name did not**. It was harmless here only because nobody had put
it in a dashboard, and the reason nobody had is luck rather than judgement.

## Revisit when

- A page arrives in `REFUSED_AT_FOUR` for a *byte budget*. That is the population this ratchet was
  built on being empty; a refusal of that kind returning means either a document unlike the corpus's
  or an allocation regression, and the two need different answers.
- A host in this tree knows its viewport before its first frame — a print path, a fixed-size
  viewer, `viewer-confined`'s worker — at which point `warm_for` costs one line and §9.1's argument
  stops applying to it.
- Somebody measures a launch on the owner's own adapter, which is the number §9 has been short of
  since it was written.
- The next revision is taken. Upstream moved three commits past this pin while the round ran, one
  of them named for the round-cap defect `QUORRA_FEEDBACK.md` §21.1 reports and one an ADR that
  re-measures `warm_for`'s saving; §22.7 says what to re-run first and why nothing here was built
  on them.
