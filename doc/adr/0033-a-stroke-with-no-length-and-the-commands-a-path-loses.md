# ADR 0033 — A stroke with no length, and the commands a path loses

Status: accepted, 2026-07-30.

## Context

§8.4 and §8.5 — the graphics state parameters and path construction and painting — were **23
`unreviewed` ledger rows**, and they are the two clauses that decide every mark on a page.
Their operators had been executed since the interpreter's first commit: `w`, `J`, `j`, `M`,
`d`, `m`, `l`, `c`, `v`, `y`, `h`, `re`, and all ten painting operators. Nothing about the
corpus said anything was wrong, and nothing could: this is the shape the ledger exists to find,
a clause whose *obvious* half is implemented and whose remaining sentences nobody has read.

The survey before any code found four things reaching nothing at all, and they are of two
different kinds.

Three were **silent duplicates of an implemented route**. §8.4.1's NOTE 1 says a graphics state
parameter "can be specified either way" — by its own operator or by an entry in a `gs`
dictionary — and Table 57's `/LC`, `/LJ` and `/ML` had no code behind them, though `J`, `j` and
`M` had. Three corpus documents set them that way: `issue16287.pdf`, `issue7878.pdf` and
`extgstate.pdf`.

One was a **paragraph nobody had read**, and it is the substance of this session. §8.5.3.2's
last paragraph gives a stroke with no length a meaning, and inverts that meaning for a *dash*
with no length. Four corpus first pages state such marks: 64 dots and 31 dotted strokes. Beside
it, three sentences in Table 58 and §8.5.3.3.1 describe a command being **removed** from a path
rather than added to it, and six corpus documents write consecutive `m` operators —
`bug1743245.pdf` 205 of them on one page — while eleven end a path with a trailing `m`.

## Decision

### One function per parameter, because the clause says the two routes are the same

`line_cap`, `line_join` and `miter_limit` are each the one place a code becomes a parameter,
called by the operator and by the `/ExtGState` entry alike. §8.4.1 states this as a property of
the graphics state rather than a convenience: a parameter set either way is the same parameter,
so two `match` arms in two places is a way for the routes to disagree and nothing else.

The clamping rules come from the same clause. A value must "be of the correct type or have
values that fall within a certain range", which is why a cap code outside Table 53's three is
Table 51's initial value rather than a guess; numeric values "shall be clipped into valid
range", which puts the miter limit's floor at 1 — §8.4.3.5's own ratio is `1 / sin(φ/2)`, which
never goes below one.

Table 57's `/Font` is the one entry of that table this tree owes and cannot yet pay: it selects
a font by *indirect reference* where `Tf` and this crate's font cache are keyed by resource
name. It is **reported**, not passed over. One corpus document writes it, and that document
left the silent set as a result — trap 5's price, paid deliberately.

### §8.5.3.2's degenerate subpath is a mark, and neither rasteriser draws it

> If a subpath is degenerate (consists of a single-point closed path or of two or more points
> at the same coordinates), the S operator shall paint it only if round line caps have been
> specified, producing a filled circle centred at the single point. If butt or projecting
> square line caps have been specified, S shall produce no output, because the orientation of
> the caps would be indeterminate.

The two backends were measured against that sentence rather than assumed to implement it, by
stroking each shape at width 10 on a 100-unit page and summing the ink:

```text
subpath              cap       tiny-skia   Vello    §8.5.3.2
m h                  butt          0.0       0.0     nothing
m h                  round        77.5       0.0     a circle, area 78.5
m h                  square      100.0       0.0     nothing
m (alone)            any         refused     0.0     nothing
```

Three different wrong answers and one right one by accident. `tiny-skia` follows Skia, whose
stroker paints square and round caps "even if the segment length is zero"; `kurbo` drops a
contour that expanded to nothing before a cap is ever considered; and a path consisting only of
`m` was an **error** on one backend and silence on the other.

So the rule is `crates/pdf-render/src/degenerate.rs`, in the crate both backends consume, and
the circle is this crate's own four-cubic geometry rather than whatever each rasteriser's round
cap happens to be. This is trap 2 in its sharpest form — *a decision either backend can make
alone is a decision neither has made* — and it is the third device decision to move into
`pdf-render` for that reason, after `Image::area_averaged` and `Stroke::device_width`.

The dot's diameter is [`Stroke::device_width`], not the width field, because §8.4.3.2's
one-device-pixel minimum decides a dot's size exactly as it decides a line's thickness: `0 w 1
J` at a point is a one-pixel dot.

### A zero-length *dash* is the opposite rule, and its direction must survive dashing

The same paragraph continues:

> This rule shall apply only to zero-length subpaths of the path being stroked, and not to
> zero-length dashes in a dash pattern of a non-degenerate subpath. In the latter case, the
> line caps shall always be painted, since their orientation is determined by the direction of
> the underlying path except in the case of a degenerate subpath.

So `[0 6] 0 d 1 J S` is a **dotted line** — Vello drew nothing at all — and under a projecting
square cap it is a line of squares *turned to face along the path*, where the subpath rule
above paints nothing. Two shapes that look identical in the file, opposite answers, and Skia
gets the second one upright because its dasher has thrown the direction away: "since the zero
length segment has no direction, set the orientation to upright as the default orientation".

The direction is recovered by not losing it. `dashes_showing_direction` dispenses a zero-length
dash at `ZERO_DASH` = 1/1000 of a user space unit — a hundredth of the thinnest line a 300 dpi
device draws — and takes that length back from the gap that follows, so the pattern's period is
unchanged and every dash lands where it would have. The dash then comes back from either
library's dasher as a segment pointing the way the path was going.

The alternative was searching the source path for the segment nearest each dash: quadratic in
the number of dashes, needing a nearest-point-on-cubic solver, and needing to exist twice
because the two backends hold their geometry in different libraries' types.

Where the direction is genuinely absent, §8.4.3.4 hands the answer back — "if the line caps are
non-round is rendered in an implementation-dependent manner" — and a square cap is given the
round cap's circle, the one shape that is the same under every orientation the square could
have had.

### Three sentences that remove a command, and why they became load-bearing this session

Table 58 on `m`: one `m` after another leaves "no vestige of the previous m operation … in the
path". Table 58 states `re` as `x y m` and three `l`s and an `h`, so an `re` overrides a
preceding `m` word for word — 60 paths on `issue12810.pdf`'s first page are that pair. Table 58
on `h`: "[i]f the current subpath is already closed, h shall do nothing". §8.5.3.3.1 on a
trailing `m`: "it shall be disregarded and not considered to be part of the path", which
§8.5.3.2 repeats for stroking and §8.5.4 inherits by defining a clip as the area `f` would
fill.

None of them changes a pixel on its own, which is exactly why they were missing: every metric
this project has looks at what was drawn. **They stopped being invisible the moment the
paragraph above made a single-point subpath a mark** — each stray `m` would otherwise have
become a dot the document never asked for, 205 of them on one page of `bug1743245.pdf`. A rule
that decides nothing can become a rule that decides everything when the clause beside it is
implemented, and the only way to have known that in advance was to read them together.

### An empty clipping path admits nothing, which the oracle found the same day

Dropping the trailing `m` made four pages drawable that had **never rendered at all** — they
were the whole of the "path is empty or contains non-finite coordinates" failure class, since a
path consisting only of `m` reached `tiny-skia`, which refuses it, and failed the page. Three
of the four then agreed with the reference consensus. The fourth, `issue9017_reduced.pdf`,
contradicted it, and the artefact said why in one look: we painted a shading across a rectangle
that all three references leave blank.

Its content stream is `568.938 673.022 m W n` — a clipping path that is *only* a trailing `m`.
§8.5.4 defines the region as "the same area that would be filled by the `f` operator", and
§8.5.3.3.1 has just removed the only subpath there was, so the area is none and everything
inside that `q`/`Q` marks nothing. This renderer had been dropping such a clip on the floor and
drawing unclipped, which is the loudest possible way to be wrong, and it had been invisible for
the project's whole life because the page never rasterised.

`Clip::admits_nothing` is where that is stated, for the same reason as the circle: `tiny-skia`
refuses an empty path and `kurbo` clips to an empty region, and it was *verified* that Vello
reaches the right page by its own convention — which is precisely a decision neither backend
has made. A `W` with no path in front of it at all is the other case, which §8.5.3.1 calls an
error; that one leaves the clip alone, because blanking the rest of a content stream is a worse
answer to a malformed file than ignoring one operator.

### What is recorded rather than implemented

§8.5.3.3.1's sentence after the trailing-`m` rule: a degenerate subpath "shall be considered to
enclose the single device pixel lying under that point", and neither backend paints it. The
clause calls that result "device-dependent and not generally useful" in the same breath, so
what is owed is a pixel nothing asks to be able to see; it is written in the ledger rather than
reported, because a report would name pages on which no reader could tell.

The two "shall generate an error" cases — a segment with no current point, a painting operator
with no path — recover rather than refuse, which is what a viewer owes a malformed file. Both
are recorded in their rows.

## Consequences

**Four pages that had never rendered now render, and the failure class is empty.** 29 no-render
pages to 25, and the four are `annotation-text-without-popup.pdf`, `issue12810.pdf`,
`issue6342.pdf` and `issue9017_reduced.pdf`. Agreement went 810 to 814 and the contradicted
count did not move: 116 total, 102 of them on pages we call complete.

**The judged set is the same size for two opposite reasons.** 1620 pages before and after:
`annotation-text-without-popup.pdf` page 1 joined it by becoming drawable, and `extgstate.pdf`
page 1 left it by starting to report Table 57's `/Font`. The corpus's silent count went 824 to
823 and its reporting count 129 to 130 — a rise that is a silence ending, not a regression.

**23 ledger rows left `unreviewed`**, 498 to 475: the whole of §8.4 and §8.5, twenty of them
`implemented`, two `partial` and one `inapplicable`. The gate's ceiling had drifted 63 rows
above the actual count over the two previous sessions and is exact again.

**Interpretation got 0.21% cheaper and rasterisation 0.15% dearer, both measured.** 1.9319 G
instructions to 1.9278 G by callgrind on `examples/callgrind_interpret`, because collapsing
consecutive `m` operators and dropping a trailing one leaves fewer commands to build than the
rules cost to apply. `callgrind_rasterise` on the corpus's most stroke-heavy page,
`22060_A1_01_Plans.pdf`, goes 35.64 G to 35.69 G, and on the specification page by 0.001%:
`split_degenerate` returns without allocating for a path with no degenerate subpath, and
`dashes_showing_direction` returns immediately for a butt cap or a pattern with no zero-length
dash. Both baselines measured on this machine at the previous commit.

**Five new test files.** `line_parameters.rs` and `path_construction.rs` assert
against the display list, because what these clauses describe is a path's own contents;
`degenerate_subpath.rs`, `fill_rules.rs` and `empty_clip.rs` assert ink, because what the
others describe is a mark. The even-odd rule reached pixels through four crates with no
assertion anywhere that it produces a different picture from the non-zero rule until this
session; the ledger asked that question, not the corpus.

**Two cross-backend scenes.** A stroke with no length and a clip that admits nothing are both
shapes where the two rasterisers had different conventions, so both are in `headless_gpu.rs`.
The first needed three times the usual differing-channel bound and nothing else loosened: six
discs 20 units across on a 200-unit page are almost entirely antialiased edge. Deleting either
backend's handling of §8.5.3.2 fails it on the *mean* — 17.91 against 0.5 for the dotted line —
which is the check that the scene can fail at the defect's magnitude.

## What it taught

**A clause whose operators are implemented can still be unread.** `J`, `j` and `M` have set the
line parameters since the interpreter's first commit; `/LC`, `/LJ` and `/ML` set nothing for
twenty-three sessions, on three corpus documents, reporting nothing. Nothing that renders a
page can find that — the operators exist, the pages draw, the metrics are silent. Only reading
the clause as a family can.

**A rule that changes nothing can become load-bearing overnight.** Table 58's `m` override was
correct to skip while a single-point subpath painted nothing, and became mandatory the moment
§8.5.3.2 made one a dot. Clauses that share a page are read together or not at all.

**Making a page drawable is how you find out what you do with it.** The empty-clip defect is as
old as the clip code, sits on a document that has been in the corpus the whole time, and could
not be seen because the page failed to rasterise for an unrelated reason. Trap 1's habit — look
at every page a feature makes drawable — found it within minutes of the four pages appearing,
and the label on the group ("no render") said nothing about what was wrong.

[`Stroke::device_width`]: ../../crates/pdf-render/src/paint.rs
