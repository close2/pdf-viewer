# 3866 — Rich media's transparency, toolbar and context click are read for the players

Session 2291. Status: **accepted and built**. Code: `pdf-model`'s `media/rich_media.rs`
(`presentation`, `embedded`, `activated`). Clauses: ISO 32000-2 §13.7.2.2.5, Table 338.

## Context

Table 338's `/Transparent`: "If true, the rich media artwork shall be composited over the page
content using an alpha channel. If false, the rich media artwork shall be drawn over an opaque
background prior to composition over the page content." `/PassContextClick`: "If false, the
interactive PDF processor shall handle the context click. If true, the interactive PDF processor
shall allow the media runtime to provide any user interface it wishes." `/Toolbar` and
`/NavigationPane` are `should`s.

## Decision

1. **`/Transparent`** sets the opacity of the rectangle's background (Table 294's, ADR 3407): 1,
   opaque, for false; 0 for true. The pictures this program decodes are H.264's, which carry no
   alpha, so the page shows through exactly where no picture covers the rectangle. A window of its
   own has no page behind it, so `Windowed` with `/Transparent` true is drawn opaque, said.
2. **`/PassContextClick`**: this program's player is the media run time here, and it provides no
   context-click interface, so the click is handled by this program either way; true is said.
3. **`/Toolbar`** true is the player's controller bar over the pictures (Table 307's
   `/ShowControls`, ADR 3595), "in proximity to the annotation"; for a `Sound` configuration, which
   shows nothing, it is said as set aside. **`/NavigationPane`** true is said: a sound or a video has
   no "hierarchical relationship of entities within the artwork" to show.
4. A flag of another type, or a `/Style` neither table name, is said and read as its default.

## Tests

`rich_media.rs::embedded_is_the_rectangle_opaque_unless_transparent_and_the_toolbar_is_the_bar`;
`viewer-core/tests/rich_media_activation.rs::the_presentation_style_decides_the_rectangle_or_a_window_of_its_own`.
