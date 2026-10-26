# Q634 — Does "tagged PDF as far as accessibility needs it" take Table 380's inline attributes into the accessibility tree?

Asked by round 1768, which re-read the ledger's `inapplicable` rows for scope
(`doc/reviews/1768-the-inapplicable-and-out-of-scope-rows-re-read-for-scope.md`, finding 7).

## The row, and the reason it records

**§14.8.5.4.4** (layout attributes for inline-level elements, Table 380: `/BaselineShift`,
`/LineHeight`, `/TextDecorationColor`, `/TextDecorationThickness`, `/TextDecorationType`,
`/RubyAlign`, `/RubyPosition`, `/GlyphOrientationVertical`, `/TextPosition`) is `inapplicable`:
§14.8.5.4.1 defines layout attributes by the layout process they parameterise, and the note says this
program performs no such process "and exports to no format". Its four neighbours (§14.8.3.2,
§14.8.5.4.1, §14.8.5.4.6, §14.8.5.4.7) say the same, and each carries a `shall` addressed to whoever
lays the content out or defines the attribute, which keeps them.

## What has changed

`viewer-core`'s `accessibility.rs` is an export: it hands the structure tree to the platform's
assistive technology through `viewer-accessibility` and `accesskit`, and it already reads §14.8.5.4.2's
writing mode, §14.8.5.4.3's `/BBox` and §14.8.5.4.5's allocation rectangle, all `implemented`. So
"exports to no format" no longer holds. What the export lacks is what Table 380 states and a sighted
reader sees drawn: a rule through the text that is a deletion, a raised run that is a superscript,
ruby beside a base text. `accesskit` 0.24.1, already in the tree, carries `TextDecoration`
(underline, overline, strikethrough) and `VerticalOffset` (subscript, superscript).

## What moving it would take

`pdf_model::structure` already resolves an element's attributes through §14.8.5.3's owners and
inheritance; the accessibility node gains the decoration and the vertical offset, and the bridge sets
them on the text run. One round, no dependency, nothing on the launch path (the tree is built after
the first frame). `/LineHeight`, `/GlyphOrientationVertical` and `/TextPosition` have no platform
counterpart and would stay unread, which the note would say.

## The question

Is carrying Table 380's decoration, baseline shift and ruby attributes to the accessibility tree within
`CLAUDE.md`'s "tagged PDF as far as accessibility needs it"?

## Recommendation

**Yes**, for `/TextDecorationType`, `/BaselineShift` read as sub- or superscript by its sign (a mapping this program would choose and
document, since the clause states a distance and not a meaning), and the
ruby attributes where the platform role exists; the row would go to `partial` naming the three it
leaves. **And no reflowing view**, which is what the four neighbours' attributes serve: that is a
layout engine, on the authoring exclusion's side of the line.

**What the tree does meanwhile**: the accessibility tree carries roles, names, bounds, tables and
lists; the row stays `inapplicable`.
