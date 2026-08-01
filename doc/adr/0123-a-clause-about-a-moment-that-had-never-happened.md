# ADR 0123 — A clause about a moment that had never happened

Status: accepted, 2026-08-01.

## What this implements

Table 176's and Table 192's `/H`, the highlighting mode:

> The annotation's highlighting mode , the visual effect that shall be used when the mouse button
> is pressed or held down inside its active area: N (None) No highlighting. I (Invert) Invert the
> colours used to display the contents of the annotation rectangle. O (Outline) Stroke the colours
> used to display the annotation border.

Five modes for a widget, four for a link, and the same default: `I`. Until the
hundred-and-thirty-second session this program had no mouse button, so the clause described a
moment that never happened; the previous session's ledger sweep found it (ADR 0122) and this one
closes it.

## The arithmetic is the clause's own

The widget table states the effect as a function: "for each colour channel in the colour space
used for display of the annotation value, colour values shall be transformed by the function
f(x) = 1 - x for display."

§11.3.5.2's Difference mode is `B(cb, cs) = |cb - cs|`. With every source component at 1, that is
`1 - cb`. So both marks are **one white shape under one blend mode** — a filled rectangle for `I`,
a stroked one at §12.5.4's border width for `O` — and nothing new was needed in the display list,
in either backend, or in the GPU translation. A feature that costs no new primitive is a feature
whose two rasterisers cannot disagree about it.

The tests are pixels rather than commands, because nothing in a display list distinguishes "we
drew the mark" from "we drew it in the wrong colour": a red widget pressed with `/H /I` is
`(0, 255, 255)`, exactly, and one with `/H /N` stays red.

## The default, which is a reading rather than the sentence

Table 192 gives `/H` the default `I` and says "[a] highlighting mode other than P shall override
any down appearance". §12.5.5 says the down appearance "shall be used when the mouse button is
pressed or held down within the annotation's active area". For an annotation that states a `/D`
and **no** `/H`, those two disagree.

Taking the default flatly makes the file's own artwork unshowable. Measured over the corpus's
page-one annotations: **95 state a `/D` and no `/H`**, so 95 pieces of artwork would exist for a
moment that could never display them. And the corpus cannot settle it — of the **4** annotations
that state both, every one states `/H P`.

So: a **stated** mode is honoured exactly, including the override; an **unstated** one is `I`
where there is no `/D` and `P` where there is. Every entry a file writes then means something,
and the only thing lost is a default the file did not write. That is the same test this project
applied to §12.5.6.7's `/LL` and to ADR 0106's line endings — *which reading makes a file's own
words mean nothing* — and it is recorded as a reading, not as a derivation.

## The half that would have been invisible

`viewer-core` decided whether a press was worth re-interpreting the page for by looking for an
`/AP` `/D` and stopping there — which was right when `/D` was the only thing a press could
change. With `/H` implemented it would have left the whole feature unreachable from the one
program that has a mouse: 124 of the corpus's page-one annotations state an `/H` that is not `P`,
and **none of them states a `/D`**, so not one would have re-interpreted.

The question moved into `pdf-model` as `view::press_changes_appearance`, because it is two
clauses rather than one lookup and a host asking it has no business knowing that.

That is trap 5's archetype — *the `d` operator*: every layer of the feature existed and one line
decided it never ran. It was avoided here only because the session that wrote the clause also
owned the caller.

## Consequences

Tests 932 → 934, both pixels. `appearance::border_width` is new, factored out of `Border::read`
for the `O` mode, which strokes a border it has no colour of its own to read. The ledger's
§12.5.6.19 and §12.5.6.5 rows record it.

The four gates are unmoved, and cannot move: `pressed_mark` returns nothing unless the pointer is
down on the annotation, and no gate presses anything. That is the whole reason this clause could
sit unimplemented — and, one session earlier, unnoticed.
