# 1973 — A colour enters the press by its own intent, and a grey is the press's black ink

Session 1973, the fourth round of the GWG lane. ADRs 2757 and 2758. No `Q`.

**Deficit 4, diagnosed.** Both of the brief's readings were planted on 13.0's own profiles with a throw-away probe:
the source's `A2B0` was already honoured, and the press took `B2A1` under the group's rendering whatever the object
stated. HEAD gave the RGB `X` 208/0/255/1 and the CMYK `X` 205/2/255/8 against the square's 217/8/255/38; `B2A0` gives
217/8/255/39 and 214/8/253/38. The CMYK pair's `X` had the RGB pair's cause, so the second reading is the first.

**Built (ADR 2757).** §11.7.5.3: the intent "influences the conversion from a CIE-based colour space to a target colour
space, taking into account the target space's colour gamut", and is "the rendering intent parameter associated with
the object". `Profile::inverse_for` selects `B2A0`, `B2A1` or `B2A2` (parsed on first use, `IntentInverse`), and
`xyz_to_ink` takes the object's rendering; a device colour still enters under the press's.

**Deficit 7, press half (ADR 2758).** §10.3.2: on a device whose native colour space is CMYK a grey "should follow
the method described in 10.4.2.3"; §10.8.3's simulated press is such a device, which amends ADR 1437 there.
`grey_reading` answers `IntentBlack` under the simulation with a CMYK intent. The first render showed 3.0 k's `X`:
the construction carried four process tints into §8.6.7's nonzero mode, which applies only to `DeviceCMYK` and
§8.6.5.7's CIE-based implicit conversion, so `cmyk_tints` gives it none (PDF/A's construction included). The
screen half stays open: there the native space is RGB.

**Tests and plants.** `icc.rs`: each intent comes back in its own table's ink, an absent table falls back to `B2A1`.
`colour.rs`: a `Lab` colour enters a press sampled colorimetric by `B2A0` under `Perceptual`, `B2A2` under
`Saturation`; `DeviceRGB` by the press's. `colour_paths.rs`: `0.5 g` on the simulated press is `0 0 0 0.5 k`, 128 grey
on screen, and under `OPM 1` knocks out cyan. Plants: `to_device` on `inverse` alone fails both colour tests;
`xyz_to_ink` on `press.rendering` fails the press one; `grey_reading` ignoring the simulation and `cmyk_tints` keeping
the grey's tints each fail `colour_paths`. The harness gains 17 checks (13.0, 16.1, 22.1, 3.0 b and h, 23.0's grey
image) and flips one (23.0's DeviceGray, press).

**Verdicts.** 13.0 press, 16.1 ICC both, 22.1 both, 3.0 press, 23.0 press pass: screen 33 pass, 2 fail, 16 n/a; press
50 pass, 1 fail (16.4's Luminosity, deficit 10). Moved renders against HEAD: the intent-stating patches and test pages
(4), and in the press view only the grey ones (9.0, 9.1, 18.3, 3.0, 23.0; 4.0.1 by 28 edge pixels) (7).

**Gates.** fmt `--check`, clippy `-D warnings --all-targets` (`pdf-colour`, `pdf-model`): exit 0 (101 first, fixed).
nextest: 2547 of 2547. `cargo test -p conformance` after `git add -N`: exit 0. GWG harness: exit 0. Tier 2:
`raster_golden` and `corpus` exit 101 exactly as at HEAD (the same 16 moved pages, 62 incomplete); `drawn_while_read`,
`jpeg2000`, `archive_corpus`, nextest of pdf-transform and pdf-archive, and `state.sh docs` (two private links fixed): 0.

**Unfinished.** Deficit 7's screen half (whether a CMYK-intent page's screen view owes a grey the intent's black);
16.4's Luminosity (§11.3.5.3); deficit 8 still has no check of its own.
