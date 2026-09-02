# 877 — The profile states the way in, and a mask's component has a luminance: `ICCBased` 'CMYK' blending spaces convert in through the profile's `B2A`, and a one-component CIE-based mask group takes §11.5.3's `Y`

Date: 2026-09-03.
ADR: [0796](../adr/0796-the-profile-states-the-way-in-and-a-masks-component-has-a-luminance.md).
Touched: `crates/pdf-model/src/icc.rs`, `crates/pdf-model/src/colour.rs`,
`crates/pdf-model/src/soft_mask.rs`, `crates/pdf-model/examples/press_census.rs`,
`crates/pdf-model/tests/transparency_groups.rs`, `crates/pdf-model/tests/oracle.rs`,
`clippy.toml`, `doc/conformance/ledger.toml`, `doc/todo/01-ledger-partial-rows.md`,
`doc/todo/23-transparency-departures.md`, `doc/verify.md`. A worktree round on `round-877`,
not merged.

## The spec track: §11.3.4's remainder

The row named `ICCBased` 'CMYK' as "reported by name" and it had not been since ADR 0272: a
four-component profile was already a press, composited as §11.4.7's pair with its `A2B` out and a
right inverse of that sampling in. So the debt was one direction of one conversion, and four
clauses decide it in an order — §8.6.5.5 says the `B2A` is "the destination for objects being
painted within the group", §10.3.1 hands a CIE-to-CIE conversion to the ICC specification,
§10.4.2.1 ranks that route above §10.4.2.4's classic one for an ICC-enabled processor (so the
classic conversion does not apply to a profile target), and §11.7.5.3 picks the table by intent,
`B2A1` over `B2A0` as `A2B1` over `A2B0`. `icc::Profile::to_device` reads the table in `mft1`,
`mft2` and v4's `mBA ` (matrix and M curves modelled, where the `mAB ` parser still refuses them),
undoes the black point compensation the way out applied, and `colour::Press` converts in through it
where the profile is bi-directional; a CIE-based source goes in from its own XYZ and a device one
through sRGB's. A profile without the table keeps ADR 0263's right inverse. Tests derive their
inks from the table's own affine rule and sRGB's decoding.

The second half of the remainder is a mask group in its own space: a `/Luminosity` group whose
`/CS` is `CalGray` or a one-component profile is painted in its component under
`Compositing::Calibrated` — ADR 0792's route — `/BC` is that component, and
`soft_mask::luminance_derivation` composes §11.5.3's "use the Y component as the luminosity" into
the mask's table. A component of 0.5 under `/Gamma 2` is a luminosity of 0.25 where the sRGB route
gave 0.54. Three-component CIE-based spaces stay the device's channels in both places, a choice
now named in §11.3.4's and §11.5.3's rows rather than a silence.

## Witnesses

`press_census` over the crawl's 145 archives, one archive at a time under `tools/bounded.sh`:
every profile press a file names — 187 page groups, 94 output intents, 6 `/DefaultCMYK` — carries
a `B2A` and every one is evaluated; `doc/pdf.js` names none. Page one of all 290 rendered before
and after: 148 move, by a mean of up to 9.4 of 255. `6942845.pdf` and `0300111.pdf` moved toward
`mupdf` (11.90 → 9.58 and 6.91 → 4.14 mean levels) and away from `poppler`, which draws the page
in device RGB; the ADR has the pixel. `bug1721218_reduced.pdf`'s eight one-component mask groups
move the page by at most 5 of 255 on 666 pixels at two pixels per unit and it stays ambiguous.

## Gates

The whole of `doc/todo/02` §2 in the worktree, in order, each corpus line under
`tools/bounded.sh --tree 12` and one at a time: fmt, clippy under `-D warnings`, nextest (2981
passed — a first run under `--tree 8` was killed by the ceiling at 8.18 GiB and rerun at 12),
doctests, both `fuzz/` lines, the sandbox build, corpus (974 documents, 64 incomplete), the
hayro build, oracle (1945 pages, 980 agree, 60 contradicted), text, both censuses, dates, xmp,
jpeg2000, quorra (958 pages, 932 agree, 22 differ, 4 refused), fixed documents, transform and
conformance — every one green. The conformance gate failed once, on `ISO 15076-1 §10.11` written
with a `§` after a standard that is not ISO 32000-2, and was rerun after the four citations were
rewritten as "section". Round 876's gates ran on `main` beside this round's builds, and this
round's own walks ran one at a time after checking for its.

## For the next round

- §11.3.4 and §11.5.3: the three-component CIE-based spaces — a sampled 3 → 3 grid on the display
  list for a page or group, a three-curve `Y` for a mask — and the route-into-grey choice of ADR
  0790.
- A four-component profile as a mask group's `/CS`: §11.4.7's pair inside a mask, no corpus member.
- `doc/todo/23`'s remaining rows and `doc/todo/49`'s `MAX_FORM_DEPTH` decision, unchanged.
