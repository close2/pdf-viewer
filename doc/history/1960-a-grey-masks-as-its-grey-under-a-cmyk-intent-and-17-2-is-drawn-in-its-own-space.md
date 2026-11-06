# 1960 — A grey masks as its grey under a CMYK intent, and 17.2 is drawn in its own space

Session 1960, the second round of the GWG lane. ADRs 2734 and 2735. No `Q`.

**Deficit 2, diagnosed.** 16.8, 16.9 and 16.11's luminosity groups state `/CS /DeviceCMYK` and paint `DeviceGray`
images under a PDF/X-4 intent. The premise that did not hold: the luminosity is not the press's colorimetric `Y`.
It is already §11.5.3's device branch (A23). What moved was the grey's way into the intent's press, a `B2A`
separation into four inks that EXAMPLE 2 weighs darker than black alone. The plant (the grey read as the press's
black inside such a group) drew all three patches like their references in both views.

**Built (ADR 2734).** `soft_mask::content_grey` puts `GreyReading::IntentBlack` in force for a `/Luminosity` group
stating `/CS /DeviceCMYK` that composites in a CMYK intent's press. The interpreter swaps it in and out with the
mask's compositing. The grounds are §11.5.3's "no compensation for gamma or other colour calibration" and §10.4.2.3,
inside §11.7.2's implementation-dependent conversion. A788's rule and round 1949's tests are unchanged, so no `Q`.
`DeviceRGB` in such a group is not taken.

**Deficit 3, refuted.** 17.2's `X` draws (0, 247, 0), the dictionary's Adobe RGB entry on an sRGB screen. The
codestream's sRGB palette would draw (114, 247, 13). §7.4.9's precedence already held. The screen shows a green the
press cannot print (n/a), and the press view draws no simulation on a process-only page (deficit 5). ADR 2735.
Deficit 4 got a reading only: 13.0's press view also equals its screen view (deficit 5 first), and 16.1's objects
enter the page press through a `B2A` sampled under the group's rendering.

**Tests and plants.** `pdfa_grey.rs`: a grey fill and a grey image in a `/DeviceCMYK` mask group under a PDF/X
intent mask as their black ink. `jpx_enumerated_spaces.rs`: a stated `/Indexed` space sets a `pclr` palette aside.
The harness gains eleven mask checks and a press check on 17.2. **Plants**: `content_grey` returning the page's
reading fails the fixture test, (247, 247, 247) against (200, 199, 199), and all eleven mask checks, gaps 25 to 84.
The palette asked of the decoder fails the JPX test ("takes 1 components but the codestream has 3").

**Verdicts.** 16.8, 16.9 and 16.11 pass in both views; 17.2 is n/a on screen and fails in the press view. Screen:
31 pass, 4 fail, 16 n/a. Press: 34 pass, 17 fail.

**Gates.** fmt `--check` and clippy `-D warnings --all-targets` (`pdf-model`): exit 0 (101 first, on a
`doc_markdown` word in the new test). nextest `pdf-model`: 2378 of 2378. `cargo test -p conformance` after
`git add -N`: exit 0. `state.sh --round 1960 docs`: exit 0. Tier 2: `golden` and `corpus` exit 101, as at HEAD: the
same sixteen moved pages round 1955 named and 62 incomplete documents, so this change moves no corpus page and
nothing goes to lavapipe. `ahead` exit 0. `jpeg2000` exit 1: its test skips because `opj_decompress` is not installed.

**Unfinished.** Deficit 4 needs a plant of its `B2A` reading. Deficit 5 now carries 17.2 and 13.0 as well.
