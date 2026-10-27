# 2440 — The page boundaries are shown as guidelines on request, in the style the page states

Session 1803, a "clause" round of batch one hundred and twenty, 2026-10-27.
Status: implemented — §14.11.2.2 moves from `inapplicable` to `implemented`. Context: review 1768
section 4's first candidate, RFC 0009 section 2.10, `doc/questions/A254` (section 12 item 2), ADR 1228
(the separation simulation, whose shape this copies), ADR 1227 (the boundaries `Page` carries).

## 1. The clause, and why the row was `inapplicable`

ISO 32000-2 §14.11.2.2 is a permission: "Interactive PDF processors may offer the ability to display
guidelines on the screen for the various page boundaries." The row said the program declined it and
would fill the row "the day a viewer grows a prepress view". Review 1768 ranked it first of its
candidates — one round, no dependency, no question — and A254 had already adopted RFC 0009's
"§14.11.2.2's guidelines in the panel". This round takes the permission without waiting for a print
panel: what the clause asks for is a display on the screen, and the screen is the window.

## 2. What is built

- **The asking.** `b` in every window (`viewer_host::keys`, `WindowAct::Guidelines`), toggled by the
  window and sent as `viewer_core::Command::Guidelines(bool)`, applied to every open document and every
  one opened afterwards — `Command::Separations`' shape and its reason (ADR 1228): the state is the
  reader's preference, `viewer-core` holds the answer it was last given, and the confined worker gets it
  over the wire (`k::GUIDELINES`). The C ABI numbers the key and the act after the existing ones
  (`QUORRA_KEY_B` 33, `QUORRA_ACT_GUIDELINES` 22) so that no number a caller passes changes meaning,
  and the Windows host's constants follow. Off until asked; nothing on the launch path.
- **The reading.** `pdf_model::guidelines::of` reads Table 396 off the page object (`/BoxColorInfo` is
  not inheritable), and draws a guideline for each of the crop, bleed, trim and art boxes the page
  **defines** — Table 396 says each entry "shall be ignored if no crop box is defined in the page
  object", and the same of the other three, and a box the page does not define is the crop box by
  §14.11.2.1's default, so a second rectangle over it would claim a boundary the page never stated. An
  inherited `/CropBox` counts: §7.7.3.4 makes it the page object's value (`Page::crop_box_defined`,
  `Page::defines`). The rectangle is §14.11.2.1's intersection with the media box, which `Page::boundary`
  already holds.
- **Table 397**, key by key with its stated defaults; `/D` through §8.4.3.6's one dash function at
  phase 0, because "[t]he dash phase shall not be specified and shall be assumed to be 0". A value the
  table does not admit — a colour outside 0.0 to 1.0, a negative width, a style other than `S` or `D`
  ("[o]ther guideline styles may be defined in the future"), a dash array §8.4.3.6 rejects — is drawn at
  that key's default and said as `Unsupported::Guidelines`, only while the guidelines are shown.
- **The drawing.** The guidelines are strokes appended to the page's display list after its
  annotations, under the page's base transform, so `/Rotate`, the crop box's origin and the
  magnification place them where the content they bound is placed, and every window and both backends
  draw them with no host drawing code. Only under `Purpose::View`: the clause's guidelines are "on the
  screen", so a print carries none.

## 3. The choices, written down as choices

1. **This program's own default settings.** "If a given entry is absent, the interactive PDF processor
   shall use its own current default settings instead." The clause hands the value over; this program's
   is Table 397's width and style, and a colour per boundary so that four rectangles drawn together can
   be told apart: the crop box black (Table 397's own default), the bleed box blue, the trim box green,
   the art box red. No clause states these colours and no other program was consulted for them.
2. **A guideline is centred on its boundary**, as a stroke of the rectangle is. Table 397 states a
   width and no placement; centring is what a stroke of the boundary's own path does, and a guideline
   on the displayed region's edge (the crop box, by default) therefore shows its inner half.
3. **A guideline lies inside what the page displays.** The marks are clipped to §12.2's `/ViewClip`
   like everything else on the page, so a bleed box larger than the crop box shows only its part inside.
   Widening the displayed region while guidelines are shown would move the page under the reader; it is
   not done, and the limit is stated here rather than discovered.
4. **Not over a page composited in its own space.** Table 397's `/C` is a DeviceRGB colour, and a page
   whose group puts compositing in a blending space of its own (§11.4.7: grey, a calibrated space, or the
   subtractive pair), or one drawn as §10.8.3's separations, carries every mark through that space's
   conversion on the way out. Such a page gets no guideline and one sentence saying so, rather than a
   guideline in a colour the page never asked for (trap 5). A post-composite overlay layer in
   `pdf-render` and three backends would lift this, and nobody has asked for it.

## 4. Why marks in the display list and not chrome geometry

`doc/ui-boundary.md` sends interactive chrome across as geometry for each host to draw in its own
idiom — a focus ring, a selection. A guideline is not that: its colour, width and dash are the
**page's** statement (Table 397), not a platform's, and drawing it from the display list is the one
route that puts the page's own style in every window identically, the C ABI and the confined worker
included, with the oracle's comparison untouched because no default run carries one. The cost is the
re-interpretation a toggle asks for, which is the separation simulation's cost and is paid on a
person's keystroke.

## 5. Evidence

`crates/pdf-model/tests/guidelines.rs`: each defined boundary and none other; Table 397's defaults key by
key; nothing unasked and nothing on paper; the dashed trim box rendered and read at a dash and at a gap;
an inherited crop box; a box past the media box drawn as its intersection; three malformed values
drawn at their defaults and said. The rendered fixture was looked at (trap 1). The corpus arms were
not run: no corpus walk asks for guidelines, and the one new call on the default path returns before it
reads anything, which `guidelines_are_drawn_only_on_a_screen_that_asked_for_them` pins.
