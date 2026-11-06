# 1962 — A process-only page is separated, and an image overprints by its own space

Session 1962, the third round of the GWG lane. ADRs 2736 and 2737. No `Q`.

**Deficit 5, diagnosed.** The premise held: `content::separate` answered `NotSeparated::Nothing` without a spot
colourant. §10.8.3 step a)'s device "supports subtractive process colourants and possibly spot colours", so the
reading was wrong rather than incomplete. `compare -metric AE` of screen against press at HEAD was zero on every
process-only patch, and on all of them after the fix; the screen view moved on none of the 51.

**Built (ADR 2736).** `DeviceSpots::new` with no names on a press is a simulated device, so §8.6.6.4's process names
paint their plate and `DeviceSpots::overprint` answers §8.6.7. `separate` makes the process pair alone
(`set_blending`); a page compositing in one or three components with no spot colourant is unchanged.
`colourants::plane` reads such a pair. Cost, `render_at` at 180 dpi, screen → press: 1.0 226 M → 920 M instructions,
9.0 385 M → 1 159 M, 18.1 796 M → 1 895 M, 16.0 (already a pair) 1 057 M → 1 159 M.

**Deficit 6, built (ADR 2737).** The re-judging showed 19.x b, 8.2 and 1.0 e still wrong: only fills, strokes and
glyphs asked §8.6.7. `overprint_blend_of` asks it of an image's and an `sh` shading's own `/ColorSpace`, and
`stencil_overprint_blend` of a stencil's fill space, with the nonzero mode left out ("shall not, however, apply to
the painting of images or shadings") and §11.7.4.3's implicit group under a non-Normal mode. Shading pattern fills
are not asked; no patch exercises them under `op`.

**Tests and plants.** `spot_planes.rs`: a process page separated into its pair (replacing the test that pinned the
old reading); §10.8.2's cyan and yellow `Separation`s overprinting; OPM 1 on a process-only press; an image, an `sh`
and a stencil marking their own colourants; a process image keeping LogoGreen. Plants: the old early return fails
the three deficit-5 tests; `DeviceSpots::new` without the press fails the overprint one; `object_overprint`
returning `state.blend` fails all four deficit-6 tests. The harness gains eleven press checks (1.0 a, b, e; 2.0 c,
h; 19.0–19.2 a, b) and flips four (1.1 OPM 1, 17.2, 23.0's Separation Black and DeviceN image).

**Verdicts.** Press: 1.0, 1.1, 2.0, 3.1, 8.01, 8.1, 8.2, 17.2 and 19.0–19.2 pass. 13.0 still fails, now with a green
`X` in all four squares (deficit 4). Deficit 8 does not reproduce on the CMYK test page. Screen: 31 pass, 4 fail,
16 n/a. Press: 45 pass, 6 fail.

**Gates.** fmt `--check` and clippy `-D warnings --all-targets` (`pdf-colour`, `pdf-model`): exit 0 (101 first,
`too_many_lines` on `draw_image`, fixed by `image_blend`). nextest `pdf-colour` and `pdf-model`: 2544 of 2544.
`cargo test -p conformance` after `git add -N`: exit 0. Tier 2: `raster_golden` and `corpus` exit 101 exactly as at HEAD (16 moved pages, 62 incomplete,
the same documents); `drawn_while_read` and `jpeg2000` exit 0. `state.sh docs`: not run.

**Unfinished.** Deficit 4 (13.0's CMYK squares now point at the source profile's intent into the press); shading
pattern fills under `op`; deficit 8 has no check of its own yet.
