# 1478 — The drive reads its steps off the window, keeps one golden, and drives the confined refusal

Session 1321. Status: **accepted**.
Context: `tools/drive-windows.sh`, `doc/verify.md`'s drive entry. Amends ADR 1453 (which left ten
steps `manual`, "look at the picture") and ADR 1466 section 5 (whose `quorra-confined` title was
undriven).

## 1. The ten steps, and what now witnesses each

| step | windows | what a person looked at | what the drive now reads |
|---|---|---|---|
| `10-popup` | three | the popup open, then gone | pixels of the annotation's `/C` colour `#FFE633` — its title bar and icon — at least 1000 more open than closed, and under 1000 closed |
| `15-restrictions` | `quorra` | the card of four levels | the card *driven*: Down, Enter sets copy to `on`, which the core's trace states (`restrictions in this window copy:On`); Up, Enter sets it back |
| `16-print` | three | a dialogue or a preview | GTK: a top-level window titled `Print` of the process within ten seconds; `quorra` and Qt, which have no printer (ADR 1180): their "over 3 page(s)" note |
| `24-reopened` | GTK, Qt | 1, 2, 3, the box ticked, Blue | AT-SPI: the toolkit widgets' text, `CHECKED` and the combo box's name, `1 2 3 True Blue` |
| `24-reopened` | `quorra` | the same | a golden (section 2) |

The two find steps now look for a positive answer (`quorra`'s `searched: page` trace, the toolkits'
`found` note) instead of the absence of "not in this document".

## 2. One golden, and what a golden is

`quorra` draws its form fields and publishes no value for them (`doc/todo/31`), so its reopened form
has no witness but its picture. The drive crops the main window to the fields and compares that crop
(ImageMagick `compare -metric AE -fuzz 10%`, at most 40 pixels off) with the one it kept. **A golden
is a picture a person looked at once**: the first run writes it and reports the step `manual`; later
runs compare. It lives under `--goldens` (default `scratchpad/drive-goldens`), per machine and never
committed, because two toolkits' fonts and a driver's antialiasing are the machine's. `--regolden
REASON` writes it again and appends the date, the window, the step and the reason to `reasons.tsv`,
as `raster_golden` is regenerated with a reason. A second golden is added only where nothing the
window states can witness the step.

## 3. The confined refusal

ADR 0607 sends a page across the confined pipe as marks only where the encoded list is smaller than
the raster; the Type 3 cycle's list is not, so its worker sent pixels and the device had nothing to
refuse. `drive-coverage.pdf` is a thousand fifteen-point stars over the page: a few kilobytes of
marks, and more coverage than the scratch sheet holds, so on lavapipe the device refuses it
(`the frame's rasterised coverage outgrew the 16384x16384 scratch image`) after 336 tiles. Step
`28-confined-refusal` reads the title (`drawn on the processor … — confined`), the terminal line,
and that the picture is not blank. An adapter with a larger scratch limit would draw it, and the
step would then say `wrong` with what it saw, which is the honest answer on that machine.

## 4. What is left

`01-center-window` on GTK and `27-processor-fallback` on GTK and Qt stay `not offered`, for ADR
1429's and ADR 1466's reasons. The golden is the one step left to a picture.

## 5. `MachineFaces::settle` waits for the wake

Amends ADR 1406. `viewer_host::machine_faces`'s searching thread records an answer and then calls
the window's wake; `settle` returned once the answer was recorded, so a test counting wakes could
read the count between the two, and under a loaded workspace run it did. `State::unwoken` counts
answers whose wake has not yet been called, the thread decrements it after the call, and `settle`
waits for it to reach zero as well: a set of faces is settled when its window has been woken for
every answer it holds. `settle` is for tests and measurement only, so the wait costs no window.
