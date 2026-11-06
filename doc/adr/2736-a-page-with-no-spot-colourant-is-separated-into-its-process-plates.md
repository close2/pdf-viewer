# 2736 — A page with no spot colourant is separated into its process plates under the press view

Status: accepted. Session 1962. Context: ISO 32000-2 §10.8.3, §10.8.2, §8.6.7, §8.6.6.4, §11.7.3; ADRs 1311 and 1329
(the separation with spot planes); the Ghent PDF Output Suite (ADR 2728), deficit 5 of `doc/gwg-output-suite.md`.

## The reading

`content::separate` answered `NotSeparated::Nothing` for a page naming no spot colourant, so the press view of a
process-only page was its screen view. §10.8.3 step a) does not condition the simulation on a spot colourant:

> Process the PDF as if separations were to be created for a simulated device that supports subtractive process
> colourants and possibly spot colours.

The process colourants are the device's whatever the page names, and they are where §8.6.7 acts — "If the overprint
parameter is true and the output device supports overprinting, erasing actions shall not be performed" — and where
§10.8.2's own example puts the difference: a `Cyan` and a `Yellow` `Separation` overprinted "will appear green when
produced using separations". A process-only page under the simulation is therefore separated on the simulated press.

## The construction

- `pdf_colour::colour::DeviceSpots::new` with no names but a press is a simulated device rather than none, so a
  `Separation` or `DeviceN` naming `Cyan`, `Magenta`, `Yellow` or `Black` paints that plate directly (§8.6.6.4) and
  the overprint question is the one the spot planes already ask (`DeviceSpots::overprint`).
- `content::separate` makes the process pair for such a page and puts it together with `set_blending`, the pair with
  no `SpotSeparation` beside it; steps b) and c) have one separation to multiply, which is the pair converted out of
  the press. Its `colourants::Separation` record has no colourant and two planes.
- A page whose group composites in one or three components has no process plate to separate into (ADR 1329), so with
  no spot plane its simulation is still the page it drew.

## What moved, and what it costs

Every process-only patch of the suite now draws its press view in the press. 1.1, 17.2's press view and the line-art
halves of 1.0 and 19.0–19.2 pass; 23.0's `Separation Black` and `DeviceN Black` image join DeviceCMYK in the press
view; 13.0's press view now shows the `X` in all four squares, deficit 4's rendering intent into the press. The screen
view is unchanged on all 51 patches (`compare -metric AE`, zero).

The press view is a reader's choice and off the launch path. `render_at` at 180 dpi, `perf stat -r 7`, screen against
press with the fix: 1.0 226 M → 920 M instructions (219 → 424 ms task-clock), 9.0 385 M → 1 159 M (246 → 437 ms),
18.1 796 M → 1 895 M (290 → 581 ms); 16.0, already drawn in its `/DeviceCMYK` pair, 1 057 M → 1 159 M. The pair is two
interpretations and a press conversion, the price every spot page already paid under the simulation.
