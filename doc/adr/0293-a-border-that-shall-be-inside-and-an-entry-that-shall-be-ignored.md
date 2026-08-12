# 0293 — A border that shall be drawn inside, and an entry that shall be ignored

**Status.** Accepted.
**Context.** `doc/todo/01`'s standing job. §12.5.4 is one of the rows `git blame` says nothing has
re-read since commit 93 of 607 — one of the three left above that fold after the
four-hundred-and-forty-second read thirty-two — and reading it beside `appearance.rs`'s `Border`
found two departures from two sentences, both silent.

## The first sentence, and the style it was not applied to

§12.5.4, first paragraph:

> If present, the border shall be drawn completely inside the annotation rectangle.

`Border::inset` is that sentence: a stroke straddles its path, so the path is the rectangle inset
by half the width, and every rectangular style got it. **The `U` style did not.** Table 168:

> U (Underline) A single line along the bottom of the annotation rectangle.

and `Border::outline` put the path *on* that edge, under a comment saying "[t]he line's width is
centred on that edge, as any stroke is on its path" — which is a true statement about a stroke and
says nothing at all about where the path goes. Half a `/W`-wide line therefore fell below `/Rect`.

**The symptom was not ink outside the rectangle, and that is why nothing saw it.**
`Constructed::bounded` clips a link's construction to `/Rect` (the four subtypes stating geometry
in default user space are the exceptions, and a link is not one of them), so the outside half was
cut away and what a reader got was an underline half the width the document asked for. A departure
that loses half a mark *inside* a clip looks like a mark. The fix is one line — the path is the
bottom edge raised by half the width, the same arithmetic `inset` already does — and the ends stay
on the rectangle's own sides because a constructed appearance never sets a line cap, so the default
butt cap keeps the ink between `rect[0]` and `rect[2]`.

**One corpus document reaches it**: `annotation-border-styles.pdf`, whose object 29 is a
`/Subtype /Link` with `/BS << /S /U /W 1 >>`, `/Border [0 0 1]` and no `/AP`. It is the *only* `U`
in the population, which is the count the next section explains how to take.

## The second sentence, and the half of an entry that outlived it

Table 166, on `/Border`:

> If an annotation dictionary includes the BS entry, then the Border entry is ignored.

Errata Collection 3 Issue #287 (`/State` `Review` `Completed`) sharpens *is ignored* to *shall be
ignored*; the precedence is the same either way, and `border_width`'s own doc comment has cited it
for many sessions. `Border::read` obeyed it **for the width, the style and the dash, and not for
the corner radii** — `/Border`'s first two elements were read before the branch and used whatever
`/BS` said.

The reason that survived is worth naming, because it is a shape the sweeps in `doc/todo/01` do not
have: **the radii are the one thing Table 166's array states that Table 168 has no entry for.**
Reading them beside a `/BS` therefore looks like completeness — a value nothing else supplies,
taken from the only place that supplies it — where it is in fact a border the standard says is
square, drawn round without a word. A precedence rule is not a rule about which entry is *better
informed*; it is a rule about which entry is read.

## What the corpus can and cannot say, counted before it was believed

`crates/pdf-model/examples/border_precedence_census.rs` walks every annotation on every page and
restricts itself to the ones that state no `/AP`, because §12.5.2 hands a stored appearance the
whole job and a border this crate never constructs cannot be misplaced by it — trap 11's condition,
derived from the clause rather than from the code. Over the 964 openable documents of the 974:

```text
  34 835 annotations, 33 781 of them stating no /AP
     192 state both /Border and /BS
       6 of those state a non-zero /Border corner radius
  border styles among the constructed: D 1, S 201, S (or default) 33 578, U 1
```

So the first departure has exactly one witness and the second has none: all six radius-and-`/BS`
annotations are `/Subtype /Ink`, whose mark is `/InkList` and which never reaches `outline`. That
is trap 8 in its plainest form — a rule the corpus states and cannot rank — so
`a_border_style_dictionary_ignores_the_border_arrays_corner_radii` is a **pair** of fixtures
differing only in whether `/BS` is present, which is the shape `pdf-syntax/tests/cross_references.rs`
already uses for §7.5's rules.

## What the other readers do, as evidence and not as truth

pdf.js reads the two entries as `if (borderStyle.has("BS")) { … } else if (borderStyle.has("Border")) { … }`,
so a `/BS` takes the corner radii out of reach there too, and it draws an underline as a CSS
`border-bottom`, which is inside the element's box. Agreement raises confidence that Table 166 and
§12.5.4 were read right. It is not why either change was made: both sentences say what they say.

## Consequences

- §12.5.4 stays `partial`, and for the two reasons it already named: Table 168's `B` and `I` state
  no highlight or shadow colour, and Table 169's cloudy `/BE` states no curve. Both are reported.
- The oracle is unmoved. `annotation-border-styles.pdf` is not in the ambiguous bucket, so
  `doc/todo/00` step 7's ink sweep over all 786 ambiguous pages cannot see this round at all — run
  anyway, and the alarm holds: twenty names at or past −1 of 255, sixteen of them documents this
  tree already calls incomplete, and the four complete ones are the four diagnosed names
  (`issue16038.pdf` −5.734, `issue12295.pdf` −2.823, `issue14297.pdf` −1.145, `issue7821.pdf`
  −1.000).
- **The lesson is the first departure's, and it is about where a departure hides.** A clip that
  bounds a construction turns "half the mark is in the wrong place" into "the mark is thinner", and
  the second is not a report, a refusal or a distance from a reference. Ask of any construction
  clipped to its own box what the clip is *absorbing*.
