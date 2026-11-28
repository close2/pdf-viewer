# 3865 — Rich media is embedded in its rectangle, or windowed within the document window

Session 2291. Status: **accepted and built**. Code: `pdf-model`'s `media/rich_media.rs`
(`presentation`, `Style`, `RichMediaWindow::floating`, `activated`) and `media/screen.rs`
(`Floating::held_within`); `viewer-host`'s `video/screens.rs` (`place`,
`held_within`); `viewer-confined`'s protocol (`write_window`, `read_window`). Clauses: ISO 32000-2
§13.7.2.2.5, §13.7.2.2.6, Tables 338 to 340. Plan: `doc/todo/80` section 1 step 2; ADR 3820.

## Context

Table 338's `/Style`: "A interactive PDF processor shall support the state Embedded", the default;
`Windowed` is a floating window "an interactive PDF processor may present", and one "that supports
Windowed styles shall constrain the window extent to be within the presentation area". Tables 339
and 340 give its size in default user space units, "similar to how it presents an annotation when
the flags NoZoom and NoRotate are set", and its position from an edge with an offset that "shall
remain fixed, regardless of the page translation". ADR 3855 played every `Video` configuration in
the annotation's rectangle. The players already open Table 295's floating windows (ADRs 3585 to 3587).

## Decision

1. **`Embedded`** is the annotation's rectangle, as ADR 3855 drew it.
2. **`Windowed`** is a Table 295 floating window, relative to the document window, at the corner
   Table 340's alignment names (`HAlign` reversed for a right-to-left catalog `/Lang`, by its BCP 47
   script subtag or else its primary language — a listed choice), moved by `/HOffset` and
   `/VOffset` and held within the document window, moved and if need be shrunk
   onto it (`Floating::held_within`, `screens::held_within`). The document window is this program's
   presentation area.
3. **One default user space unit is one pixel**, the scale of a page at magnification 1, which
   NoZoom leaves (a choice: the clause fixes the unit, not the pixels). The window keeps `/Default`'s
   size and is not resizable, so `/Min` and `/Max` bound nothing a person can do; a `/Min` past
   `/Max` is read swapped and a `/Default` outside them held, each said.
4. **A host that cannot hold a window within the presentation area does not support `Windowed`**:
   one that opens no media window (Qt, `viewer-ffi`) or places none (GTK 4, whose window manager
   places every window) presents the clip `Embedded`, said by name. The winit window places one.
5. A `Sound` configuration shows nothing in either style; the annotation keeps its appearance.

## Tests

`rich_media.rs::windowed_opens_a_window_held_within_the_document_window_where_the_host_places_one`;
`screens.rs::a_rich_media_window_is_offset_from_its_corner_and_held_within_the_document_window`;
`viewer-core/tests/rich_media_activation.rs::the_presentation_style_decides_the_rectangle_or_a_window_of_its_own`.
