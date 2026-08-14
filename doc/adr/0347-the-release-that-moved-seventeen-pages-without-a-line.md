# 0347 — The release that moved seventeen pages without a line, and the reuse that came back as a question

**Status.** Accepted.

## Context

The project owner said a new quorra release was available. The tree pinned `a7babab` (taken in
the four-hundred-and-seventy-eighth session, ADR-less because it cost no line); upstream's head
was `87898c69`, twenty-five commits on. `doc/QUORRA_UPGRADE.md` is the procedure and
`doc/todo/02-every-round.md` §2 makes all four corpus lanes — both coverage lanes at the page's
own scale and at four times it — a debt of the round that takes the release, for ADR 0283's
reason: a release can live entirely inside a lane the default gate does not exercise.

Two things were owed beyond the lanes. `QUORRA_FEEDBACK.md` §22.7 had left standing orders:
re-run §21's two readings (the round cap that deposits no ink, the small circle flattened to its
inscribed polygon) first thing after the next bump, because upstream's sibling checkout already
carried a commit named for the first of them. And `doc/todo/44` §3 had made the encode cache an
upstream ask — a retained/reusable encoded scene or scene-fragment composition, and a root
affine so zoom steps reuse the encode — so whether the release carried either had to be
answered, not assumed.

## Decision

**Take `87898c69`.** The bump is two hashes in `Cargo.lock` and not one line of source — the
second such release in a row (upstream moved `SceneError` and its reason enums into
`quorra-scene/src/error.rs` but kept their crate-root paths, so the move is invisible here).
What the release changes is what a frame *draws*, and the evidence and re-baselines follow.

### The four lanes, and the seventeen pages

| | agree | differ | refused | not comparable |
|---|---:|---:|---:|---:|
| scale 1, `cpu` | 934 | 20 | 2 | 18 |
| scale 1, `gpu` | 933 | 21 | 2 | 18 |
| scale 4, `cpu` | 936 | 10 | 5 | 23 |
| scale 4, `gpu` | 937 | 9 | 5 | 23 |

**Seventeen pages left the differing list at scale 1 and none arrived, at any scale, on either
lane.** Fifteen — the six `tracemonkey*` pages, `bug1885505.pdf`, `bug1992868.pdf`,
`chrome-text-selection-markedContent.pdf`, `issue14438.pdf`, `issue15012.pdf`,
`issue18911.pdf`, `issue19239.pdf`, `issue7014.pdf`, `issue7492.pdf` — are quorra's ADR 0044:
a cubic's flattening bound is now the tighter of the fixed quarter-pixel tolerance and **1/32 of
the cubic's own device extent**, flooring a full turn at sixteen chords, argued from §10.7.2's
own NOTE 2 ("the purpose of the flatness tolerance is to control the precision of curve
rendering, not to draw inscribed polygons"). The population such a floor reaches is glyph
outlines — bowls at body size are cubics two to five device pixels across — which is why what
moved is prose. The other two, `extgstate.pdf` and `inks_basic.pdf`, are the round-cap fix
(`d594566`): the near cap of an open subpath was the *inward* half-disc wound against the body
it lies inside, so the two caps cancelled under the non-zero rule and a hole was punched where a
cap belongs — invisible to summed ink, which is why §21.1's instrument read a round cap as
exactly a butt cap. `DIFFERS_AT_THE_EDGES` is re-baselined 24 → 7 names with the argument in its
doc comment. Every page moved *toward the shape's own area*, not toward the other rasteriser —
principle 5's direction, stated by upstream's commits themselves.

### The refusal ratchets, and a hole that was not the release's

`REFUSED` is unchanged at its two names. `REFUSED_AT_FOUR` gains `issue18032.pdf` (4 → 5 names)
— **a stale baseline caught, not a regression**: session 492's §11.4.6 refusal is stated before
any scene is built and therefore holds at every scale, but that round re-ran the default lane
only, and no round between it and this one ran the 4× lane the list is measured at. The list's
doc comment now carries all five with their three kinds of reason, including upstream's `5483996`
measuring the multi-sheet fix for the two sheet-capacity pages and **declining it with numbers**
(a second sheet re-refuses one page on bytes and prices the others at a quarter-gigabyte of
per-frame upload). The sheet-capacity refusals are unmoved, as `QUORRA_UPGRADE.md` §6 predicted;
the two §11 construction refusals from session 492 are unmoved, and nothing in the range touches
the scene vocabulary they would need.

### The two §21 gates are written

Re-measured at this pin with `sub_pixel_marks`: the 40 × 5 round-capped rule reads −0.1% from
Table 53's own area (was −8.9%), the short rule −1.2% (was −60.9%), the one-pixel dot −2.1%
(was the inscribed square, −36.1% exactly). So `sub_pixel_coverage.rs`'s round-cap and dot rows
now hold **both** backends to the shape's own area — the rows held against the processor only
since the four-hundred-and-fifty-fifth session, flipped on the condition their own comment
stated: "[b]oth come back the moment either row draws its area." `QUORRA_FEEDBACK.md` §21 is
marked answered (§21.4), with two corrections kept: the hole-not-absence mechanism, and that
the governing clause is §10.7.2 (flatness), not §10.7.3 (smoothness) as §21.2 had cited.

### `doc/todo/44` §3: neither ask shipped, both answered — and this tree's answer back

Quorra's ADR 0045 **priced** the retained encode (an identical frame replayed: 0.154 ms against
1.538 re-encoded, tenfold) and built neither ask. Scene-fragment composition is deliberately
unbuilt pending one question: *can the host draw the page and the overlays as two `render` calls
into the same target?* — if yes, replay needs no new vocabulary; if no, the reason is the
specification for fragment composition. **This tree's answer is no, and `present.rs` names the
reason**: the frame is one scene because the selection overlay is `Multiply` fills (ADR 0176)
that must composite against the page beneath them, so a second call's root pass would need to
begin over the target's existing pixels rather than a cleared backdrop. That reason is recorded
in `doc/todo/44` §3.1 as the specification upstream asked to design from. And the root affine
does **not** buy zoom reuse at any price — the transform is inside every atlas key, the
flattening and the lane choice — which corrects §3's hope: page-space scene building buys the
`scene` phase only (median 50.2 ms on the owner's document), and that was already the contract.
todo/44's "same ~60 ms after a zoom step" sentence is withdrawn.

### The frame path on the owner's document

`tmp/Entwurf.pdf` under `Xvfb`/llvmpipe, both pins alternated A/B/A/B, structure only (the
machine carried parallel rounds; no wall clock is quoted): the shares are unchanged — `encode`
~90% of `device` at the median on both pins, `transfer` ~0.1 ms, `execute` single digits,
uploads and cull counts identical. One discordant run disagreed with its own arm's twin by 3×,
which is what a load spike looks like and why the arms were alternated.

## Consequences

- The corpus differing floor at scale 1 is 20 pages, and the `tracemonkey` family — this gate's
  textbook example of the antialiasing floor since its first run — is gone from it. The doc
  comments that used the family as the floor's description are rewritten.
- `REFUSED_AT_FOUR` is five names and its doc comment now states which refusals are scale-free
  constructions of this tree's own. A future round that lifts ADR 0327's refusals re-baselines
  *both* lists.
- `warm_for`'s declined table may no longer be quoted: quorra's ADR 0040 retracted 24.7 → 10.3 ms
  at the source (the cost was two in-frame pipeline compiles, now warmed — with the presenting
  format's pair covered by their ADR 0043). §9.2 carries the dated note.
- The encode-reuse conversation has a concrete next step that is a *design input*, not code:
  carry todo/44 §3.1's reason upstream so fragment composition (or a root-over-stated-content
  pass) is designed from it. Nothing in this tree should build a scene cache before that exists —
  it would save the 50 ms `scene` phase and leave the 234 ms `encode` untouched.
- `Options::instrument_encode` stays unused: upstream's own callgrind attribution (recording
  78.3% of a steady encode) is already finer than the instrument would report here.
