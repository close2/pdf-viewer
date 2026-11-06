# 1957 — The Ghent PDF Output Suite is measured, and sixteen-bit TIFF prediction is undone

Session 1957, the first round of the GWG lane (the owner's words of 2026-11-06). ADRs 2728 and 2729. No `Q`.

**The suite.** The Ghent Workgroup's PDF Output Suite 5.0, fetched 2026-11-06: 51 patches and 19 test-page pages.
Its notice permits use for testing workflow setup and forbids sale, commercial use and copies without the notice,
so nothing of it is tracked (ADR 2728): `tools/fetch-gwg.sh` is the submodule's equivalent — two URLs, two SHA-256
pins, a refusal on a differing hash — and unpacks into the git-ignored `doc/corpora/gwg-output-suite/`, run once for
the main checkout and linked from this worktree; `tools/places.sh` names it `PDF_VIEWER_GWG`;
`doc/gwg-output-suite-provenance.md` is the provenance.

**The measurement.** Every page drawn on the CPU at 180 dpi in the screen view and §10.8.3's press view, and every
patch judged by eye against its README (`doc/gwg-output-suite.md`). §8.6.7 makes overprint ineffective where the
device does not support it, so fifteen patches whose `X` reveals ignored overprint are *n/a* in the screen view and
judged in the press view. After the fix: screen 28 pass, 8 fail, 15 n/a; press 31 pass, 20 fail. Nine deficits are
ranked with a first diagnosis each; the second (luminosity masks in a `DeviceCMYK` group read through the output
intent) and the fifth (no process-only overprint in the press view) are the widest.

**The premise that did not hold.** `filter::tiff_predictor` said depths other than eight "do not occur in the streams
this reader meets" and returned them unchanged; patches 18.0–18.4 state `/Predictor 2 /BitsPerComponent 16` and drew
as noise in both views, with no report (trap 5).

**Built (ADR 2729).** TIFF prediction adds components modulo `2^bits` at 1, 2, 4, 8 and 16 bits, §7.4.4.4's "taking
into account the number of bits per component"; a depth Table 8 does not list is refused. `render_at --simulate`
draws the press view. `crates/pdf-model/tests/gwg_output_suite.rs` is a ratchet over eighteen pixel checks on nine
patches, failures recorded, and skips with a sentence where the suite is absent; `tools/gwg-render.sh` renders every
page for the eye under the heavy-walk lock, building the sandbox worker inside its hold.

**Tests and plants.** Three `pdf-syntax` tests from the clause: a sixteen-bit carry and wrap, a two-bit wrap with the
row's padding untouched, an unlisted depth refused. **Plant**: the old fallback fails all three, and the harness's five
photograph checks (roughness 21.5 to 57.8 against 7.6 to 11.2); with the suite absent the harness prints its sentence.

**Gates.** fmt `--check` and clippy `-D warnings --all-targets` (`pdf-syntax`, `pdf-model`): exit 0. nextest:
`pdf-syntax` 309 of 309, `pdf-model` 2376 of 2376. `cargo test -p conformance` after `git add -N`: exit 0.
`state.sh --round 1957 docs`: exit 0 (101 first, on an unescaped `[a]` in the predictor's quotation). Tier 2: `on-disk`
exit 0; `corpus` and `golden` exit 101, 62 incomplete documents and 16 moved pages, the counts round 1955 recorded at HEAD.

**Unfinished.** Deficits 2 to 9 of `doc/gwg-output-suite.md`, for the next GWG rounds; deficit 2's hypothesis is not yet
tested by a plant, and its clause reading may need a `Q`.
