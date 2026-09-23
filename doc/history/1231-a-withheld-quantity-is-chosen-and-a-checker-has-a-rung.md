# 1231 — A withheld quantity is chosen, and a checker has a rung of its own

## What moved

§12.4.4, §12.4.4.1, §12.5.6.19 and §12.6.4.15 `departed` → `implemented` (A72, ADR 1299);
§8.6.6.5 `departed` → `implemented` (A100). §12.4, §12.11.2 and `requirements::Kind::Transitions`
now say what is drawn.

## Findings

**The brief's file was the neighbour.** `viewer-core/src/presentation.rs` is §12.4.4.2's sub-page
navigation; Table 164's frames are `transition.rs`, and every host reaches them through
`viewer_host::Clock`, so no window needed a call — `Clock::begin` builds `transition::Faces`.

**`Fly` needed pixels, not geometry.** "Changes" read as the pixels in which the two pages differ is
the one reading whose last frame is the arriving page exactly. It is computed once per transition,
lazily, into a `OnceLock`, so the GPU sees one more `Arc` rather than a diff per frame.

**§12.6.4.15 rested on the same four styles** and was not in the brief; A72's "the rows resting on
the argument" includes it.

**Table 192 is right**: `/TP` is in the appearance characteristics dictionary, not a slip.

**`requirements::Kind::Transitions` said "four … reported by name"** to a person reading a
document's `/Requirements`. It is met now.

## Instrument

`--bin cited` has `Rung::Checker`: `cited::CHECKERS` names `crates/pdf-archive` with its reason,
its 50 pairs print as one count, and a named checker the workspace lacks is printed.

## Handed over

- The frames were not driven under Xvfb; `transition_frames.rs` holds `Dissolve` and `Fly` equal
  on both backends at 0 differing pixels. A `Dissolve`'s clip cost at 4K is unmeasured.
- `conformance`'s quotation gate fails on `pdf-syntax/src/linearize.rs` (a sibling's file).
