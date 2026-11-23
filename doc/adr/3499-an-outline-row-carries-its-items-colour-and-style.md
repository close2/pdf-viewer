# 3499 — An outline row carries its item's colour and style, and every window draws them

Session 2199. Status: accepted. Context: review 2197 (finding 2197-B04); ISO 32000-2 §12.3.3,
Table 151 (`/C`), Table 152 (`/F`).

`viewer_host::PanelRow` gains `style`, an `ItemStyle` of `ink`, `italic` and `bold`, which
`row_of_item` sets from `pdf_model::outline::Item` — it already read `/C` (clamped `DeviceRGB`)
and Table 152's two bits — and every other panel leaves `None`. The colour is rounded to eight bits a component, which is
what all three toolkits draw text in, and it keeps `PanelRow` comparable.

Black is carried, not left to the theme. Table 151's `/C` defaults to `[0.0 0.0 0.0]`, and a
default the table states is a colour of the item, so an outline row carries its colour whatever the file
wrote. This is the choice the `viewer-ui` window already made by drawing `Item::colour`; its cost
is that on a dark theme an item with no `/C` is drawn black, as the table says.

How each window draws it: GTK sets Pango attributes on the row's label (foreground, italic
style, bold weight); Qt answers `Qt::ForegroundRole` with the colour and `Qt::FontRole` with a
font that is bold for Table 152's bit 2 or §12.3.5.1's mark and italic for bit 1; `viewer-ui`
draws `Item` directly in one of its four faces in `/C`'s colour. The C ABI answers
`quorra_outline_style` (`0xRRGGBB`, italic, bold), declared for .NET beside the outline's other
reads; the Windows host's drawing of it is that host's to take up.
