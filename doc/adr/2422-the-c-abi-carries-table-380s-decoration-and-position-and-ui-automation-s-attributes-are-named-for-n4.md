# 2422 — The C ABI carries Table 380's decoration and position, and UI Automation's attributes are named for N4

Status: accepted — the ABI half is built; the WinUI half is a mapping decided here and built by RFC
0010's step N4, because the window has no UI Automation tree to put it on.
Session 1794, 2026-10-26, slot 7 of batch 119. Context: ADR 2384 (Table 380's entries in the
accessibility tree), ADRs 0576 and 0737 (an entry point added moves no version), ADR 2054 (the C#
declarations held to the header), RFC 0010 section 6 (N4), ISO 32000-2 §14.8.5.4.4. ADR 2423 is
unused.

## Decision

`viewer_core::AccessibilityNode` has carried `decoration`, `decoration_colour` and `position` since
ADR 2384, and `viewer-ffi` handed a C caller none of them, so a host on the C ABI could not give a
platform's text interface the underline a sighted reader sees.

- **One new entry point, `quorra_structure_style`**, answering a `QUORRA_DECORATION_*`, a
  `QUORRA_POSITION_*` and the colour with a `has_colour` flag, every out-parameter optional. A new
  function rather than two more out-parameters on `quorra_structure_node`, because a C entry point
  cannot change shape once compiled against; a function is a symbol an old caller never looks up,
  so `QUORRA_ABI_VERSION` does not move — the rule its own comment and ADR 0576 state.
- **Two answered enumerations**, `DecorationKind` (none, underline, overline, line-through, in
  Table 380's order) and `PositionKind` (normal first, as the entry's default, then superscript and
  subscript), each produced by a `match` exhaustive over `pdf_model::structure`'s type and each with
  a `from_code`, as `kinds.rs` requires of an enumeration this ABI answers with.
- **An unstated colour crosses as an absence.** The entry's default is "the current fill colour in
  effect at the start of its associated content", which the readback does not carry. ADR 2384's
  opaque black is the `accesskit` bridge's alone, forced by a type that cannot be built without a
  colour; a C caller can be told "not known", so it is, and `colour` is not written.
- **On the element's own lines and nowhere else**, said in the header beside the declaration: the
  walk resolved each entry by its cell's reach, and a platform that inherits text attributes down
  its own tree would carry a paragraph's underline into the block nested inside it.

## The WinUI window's half, named for N4

The brief's premise was that the WinUI window's accessibility peer builds a tree these fields could
be set on. It builds none: `hosts/windows/Quorra.App/` sets an automation name on each control over
a widget and nothing else, and "UI Automation peers from `Query::Accessibility`" is RFC 0010's N4,
unbuilt. Nothing is written in the window, because a mapping no peer calls is a placeholder. What
N4's text-range provider answers is decided here so that it is not re-argued:

| from the ABI | UI Automation text attribute | value |
|---|---|---|
| `QUORRA_DECORATION_UNDERLINE` | `UIA_UnderlineStyleAttributeId` | `TextDecorationLineStyle_Single`; `_None` for every other decoration |
| `QUORRA_DECORATION_OVERLINE` | `UIA_OverlineStyleAttributeId` | `TextDecorationLineStyle_Single`; `_None` otherwise |
| `QUORRA_DECORATION_LINE_THROUGH` | `UIA_StrikethroughStyleAttributeId` | `TextDecorationLineStyle_Single`; `_None` otherwise |
| the colour, `has_colour` true | the matching `…ColorAttributeId` | the three floats as the attribute's colour |
| the colour, `has_colour` false | the matching `…ColorAttributeId` | UI Automation's not-supported value, never black |
| `QUORRA_POSITION_SUPERSCRIPT` | `UIA_IsSuperscriptAttributeId` | true; false otherwise |
| `QUORRA_POSITION_SUBSCRIPT` | `UIA_IsSubscriptAttributeId` | true; false otherwise |

`Single` is a choice: Table 380 names a line and not its style, and single is what a platform shows
for a line it was given no style for. A range spanning nodes that answer differently answers UI
Automation's mixed value, which is what the attribute's contract asks of a range.

## On Linux

Nothing is built. `accesskit_atspi_common` 0.19.1 publishes `underline` and `strikethrough` among
its text attributes and has none for an overline or a vertical offset (ADR 2384), so those two wait
on the bridge, not on this tree.

## Cost

One symbol and seven constants; the header's two checks and the C# declarations' check name them.
The tree is asked after page one's frame, so the launch path does not reach the new function.
