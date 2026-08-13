# 478 — A release that cost no line, and a survey that became a gate

**Finding:** taking quorra's `a7babab` required no source change at all, and it emptied the one
population of refusals that had kept the 4× corpus run from ever being a gate — so the magnified
run `doc/todo/02-every-round.md` §2 demands holds a refusal list to equality now instead of
printing a survey nobody could fail.

Date: 2026-08-13
ADR: [0313](../adr/0313-a-release-that-changed-no-line-here-and-emptied-a-refusal-list.md)

Files: `Cargo.lock`, `crates/render-quorra/tests/corpus.rs`, `doc/QUORRA_FEEDBACK.md`,
`doc/QUORRA_UPGRADE.md`, `doc/QUORRA_NON_ISOLATED_GROUPS.md`, `doc/performance.md`,
`doc/todo/40-mask-chain-crop.md`, `doc/todo/42-the-launch-path.md`,
`doc/todo/45-where-a-frame-goes.md`

## What the round did

Fourteen commits of quorra, six of them with ADRs. The one that matters is 0039: the root texture
is sized to what the page marks rather than to the target, and with 0036 to 0038 in front of it
every buffer in a frame is the size of its own plan. Upstream prices `issue16287.pdf` at 4× from
291 199 104 frame bytes to 6 158 496, and the corpus's layered frames at 4× from 2 259.2 MB to
1 325.5 MB.

All four lanes were run here — the two coverage lanes at the page's own scale and at four times it
— because §2 says a round taking a release owes the ones the default run does not exercise. The
verdicts, and only the verdicts, are in `QUORRA_FEEDBACK.md` §22: five other sessions were building
on this machine throughout and the CPU oracle, doing identical work in three scale-1 runs, reported
3.10 s, 12.51 s and 13.00 s for it. There is no clock in this round worth quoting and the document
says so instead of averaging it.

## The three things worth remembering

**A bump that costs nothing is a fact about the interface, not an absence of work.** The two
releases before it each cost a line or a deleted test. This one changed what a frame costs without
touching what a caller says, which is what a dependency ought to be able to do, and it is why the
diff here is a measurement.

**A list stops being unholdable when its population changes, not when somebody tries harder.** The
4× refusals were twelve, then seven, and each release moved them because most of them were a byte
count 4 % to 20 % over a budget upstream kept improving. With that population at zero, what is left
refuses for a device's texture limit or for a cache budget this tree sets — neither of which a
better allocator touches — so `REFUSED_AT_FOUR` can be held to equality by name. Session 462 had
already noticed the cost of not holding it: a "zero resource refusals at 4×" in `doc/performance.md`
had decayed to one, and nobody knows for how long, because nothing was counting.

**A counter can change its meaning without changing its name, and this one did.**
`Counters::layer_textures` reports fewer textures and different ones. Nothing in this tree reads
it — the whole cost was two sentences of prose asserting what it meant — but the reason that was
cheap is that nobody had put it on a dashboard, which is luck. It is corrected in place and dated,
and the shape is named in ADR 0313 so the next one is recognised sooner.

## One gate earned its keep on this round's own prose

`conformance::every_citation_names_a_clause_that_exists` failed on a doc comment written *in this
round*: the new ratchet's argument said "`QUORRA_UPGRADE.md` §6", and a `§` is checked against
ISO 32000-2's clauses, so a reference to somebody else's document would have passed by landing on
a real clause number. The gate's own message says to write "section N" instead. Two characters,
and the reason to record it is that the failure was in the *explanation* of a ratchet rather than
in code — the file where a citation is least likely to be checked by a reader.

## What was declined, and measured before declining

`Device::warm_for` arrived (quorra's ADR 0035) and is still not called here. §9.1's argument holds:
`viewer-ui` learns its viewport from `Resized`, after the point where the saving could be banked.
The round re-measured the first frame across both revisions rather than re-reading the paragraph —
`examples/first_frame.rs`, eight runs an arm, read at the minimum — and neither column moves. The
release adds a second reason and it is upstream's own: ADR 0039 records that `warm_for` warms a
target-sized layer, which after 0036 to 0039 is the right size for about a quarter of layered
frames, so its 24.7 → 10.3 ms headline is not a general number and this tree does not quote it.

The sheet packer (ADR 0034) needed no decision: it is not an API, and taking the revision took it.
It moved none of the three sheet-capacity refusals, which is exactly what that ADR predicts about
itself — a shelf-height limit is an occupancy and 16 384 texels is a ceiling.

## And two claims about somebody else's code were re-run rather than re-read

`QUORRA_FEEDBACK.md` §21 reports two readings of quorra's rasteriser — a round cap that deposits
exactly what a butt cap does, and a small circle flattened to its inscribed polygon. `git diff`
across the fourteen commits does not name `raster.rs`, so the prediction was that both survive.
Both do, to the digit: 200.1571 against 219.6349 of the mark's own area, and 0.5020 against 0.7854.
One command, and it is ADR 0283's lesson in the direction that is easy to skip — a claim can
*survive* a release, and only running it says which.

**And by the time the round ended, the sibling checkout had three commits past the pin** — one of
them named for the round-cap defect §21.1 reports, one an ADR that re-measures what `warm_for`
saves. `QUORRA_FEEDBACK.md` §22.7 says what the next bump should re-run first. Nothing here was
built on them: a round that measured an unpushed revision would be reporting a number no document
states.
