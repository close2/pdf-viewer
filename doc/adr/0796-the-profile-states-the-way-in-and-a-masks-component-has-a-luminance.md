# ADR 0796 — The profile states the way in, and a mask's component has a luminance: `ICCBased` 'CMYK' blending spaces convert in by `B2A`, and a one-component CIE-based mask group takes §11.5.3's `Y`

Status: accepted. Session 877.
Clauses: ISO 32000-2 §11.3.4, §8.6.5.5, §10.3.1, §10.3.2, §10.4.2.1, §10.4.2.4, §11.6.6,
§11.7.2, §11.7.5.3, §11.5.3, §11.6.5.1 (Table 142), §8.6.5.2, §8.6.5.9.
Code: `crates/pdf-model/src/icc.rs` (`Profile::to_device`, `Profile::to_xyz_with`,
`Profile::is_bidirectional`, `parse_mba`, `parse_clut`, `Encoding::encode`, `MAX_OUTPUTS`),
`crates/pdf-model/src/colour.rs` (`Press::profile`, `Press::converts_in_by_profile`,
`xyz_to_ink`, `ColourSpace::cie_xyz_at`, `ColourSpace::cie_luminance`, `srgb_to_xyz_d50`,
`xyz_to_lab`), `crates/pdf-model/src/soft_mask.rs` (`luminance_derivation`),
`crates/pdf-model/examples/press_census.rs`, `clippy.toml`.
Tests: `crates/pdf-model/src/icc.rs::a_from_cie_table_states_the_device_colour_of_a_connection_space_colour`,
`::a_legacy_lab_from_cie_table_reads_its_input_as_the_legacy_encoding`,
`::a_v4_from_cie_table_runs_its_matrix_before_its_table`,
`::the_conversion_in_undoes_the_compensation_the_conversion_out_applied`,
`::a_profile_without_a_from_cie_table_is_not_bidirectional`,
`::an_unreadable_from_cie_table_costs_the_profile_nothing_it_had`,
`::an_encoding_round_trips_through_its_inverse`,
`crates/pdf-model/src/colour.rs::a_bidirectional_profiles_press_converts_in_through_its_own_table`,
`::a_lab_round_trips_through_xyz`, `::srgb_to_xyz_is_the_inverse_of_xyz_to_srgb`,
`crates/pdf-model/tests/transparency_groups.rs::a_colour_painted_into_a_bidirectional_press_goes_in_through_its_from_cie_table`,
`::a_calibrated_mask_group_takes_the_luminance_of_its_composited_component`.
Continues ADRs 0263, 0272, 0790, 0792.

## The question

§11.3.4's ledger row has been `doc/todo/01`'s oldest `partial` since ADR 0788, and after ADRs
0790 and 0792 took the three one-component spaces it named two debts: `ICCBased` 'CMYK', "which
is `doc/todo/23`'s ICC `B2A` row", and a group's colour space reaching soft masks — §11.6.5.1's
`/G` composited in its own `/CS` with Table 142's `/BC` in that space. This decision takes both,
and begins by finding that the first was mis-described.

## What the row said, and what the tree did

The row said `ICCBased` 'CMYK' was "reported by name". It had not been since ADR 0272: a
four-component profile named by a page group, a `/DefaultCMYK` or an output intent has been
sampled into a `Press` and composited as §11.4.7's pair since the four-hundred-and-thirty-sixth
session, with its `A2B` as the conversion out. What ADR 0272 declined was the conversion *in*:
`press_for_profile`'s comment argued that a `B2A` table "is not read even where §8.6.5.5
requires the file to carry one", because a search over the sampled `A2B` gives a right inverse
— an opaque mark comes back exactly — and "[r]eading both would put two separately-built maps
on one page". So the debt was one direction of one conversion, and the question is what the
standard says that direction is.

## The reading

Four clauses answer it, in an order.

**§8.6.5.5 names the table and says why it is there.** Of an `ICCBased` space used as a
blending colour space:

> it shall have both "to CIE" ( AToB ) and "from CIE" ( BToA ) information. This is because
> the group colour space shall be used as both the destination for objects being painted
> within the group and the source for the group's results.

The sentence is addressed to the file — a `shall` on what the profile carries — but its second
half states the processor's use of each half: the `A2B` is for "the source for the group's
results", which is the conversion out this tree already takes, and the `B2A` is for "the
destination for objects being painted within the group", which is the conversion in.

**§10.3.1 says whose conversion a CIE-to-CIE one is.** "Conversion from a CIE-based source
colour to a CIE-based destination colour shall be performed based on" the ICC specification —
and a destination that is a profile is converted to, in that specification, by its `B2A`.

**§10.4.2.1 ranks that route above the classic one**, which is the ranking the task asked to be
read and stated. "Although ICC enabled PDF processors should always follow the provisions and
recommendations provided in 10.3 … a less-capable PDF processor may choose to use the algorithms
specified in the following subclauses 10.4.2.2 through 10.4.2.5." §10.4.2.4's RGB-to-CMYK
conversion is the `may` branch, and it is stated for a `DeviceCMYK` target; an `ICCBased` 'CMYK'
target is a profile, this tree is ICC-enabled (ADR 0009), and the profile's own table is the
`should`. So **§10.4.2.4 does not apply here**, and a stated black generation (`/BG`, `/UCR`)
has nothing to act on for a page compositing in a bi-directional profile's press — that debt
stays where it was, on the `DeviceCMYK` target it is written for.

**§11.7.5.3 decides which table.** The rendering intent "influences the conversion from a
CIE-based colour space to a target colour space, taking into account the target space's colour
gamut", and in the transparent model the target "may instead be the group colour space". So
the `B2A` is chosen as the `A2B` already is — `B2A1`, relative colorimetric, PDF's default
intent, and `B2A0` where a profile has only that — and the gamut question ADR 0263 answered
with the nearest reachable colour by squared distance is answered here by the profile writer's
own mapping, which is what a `B2A` table holds over the whole connection space.

What ADR 0272's objection got right is that the two tables are two maps and an opaque mark
painted into the group and out again moves by the profile's own round-trip residue. What it got
wrong is whose residue that is: the clause makes both maps the file's, so the residue is the
picture the producer specified, and a right inverse tuned so that nothing moves is this tree's
substitute for a table it had not read. ADR 0263's construction stays for the press it was built
for — the assumed inks, which have no profile — and for a profile a file states *without* a
`B2A`, which breaches §11.3.4's "bi-directional" and is drawn as well as it can be rather than
refused.

## What was built

`icc::Profile` reads `B2A1` or `B2A0` beside its `A2B`, in the three encodings the `A2B` parser
already knew and one it did not: `mft1` and `mft2` with the connection space on the *input*
side — XYZ at `0x8000`, v4 Lab, or v2's legacy sixteen-bit Lab with `L* = 100` at `0xFF00`, the
inverse of `Encoding::decode` written beside it — and v4's `lutBToAType`, whose B curves, 3×4
matrix, M curves, table and A curves run in ISO 15076-1 section 10.11's order and are all modelled,
where the `mAB ` parser still refuses a matrix it never meets. `Profile::to_device` takes a D50
XYZ, undoes the black point compensation `to_rgb_with` applied so that the two directions are
inverses in the same sense, encodes it as the table's input and returns the device's channels.
A table the parser cannot read costs the profile nothing it had: the `A2B` half parses on its
own and the profile is simply not bi-directional.

`colour::Press` holds the profile where it is bi-directional, and `to_cmyk` routes a colour into
such a press as an XYZ: a CIE-based source's own (`ColourSpace::cie_xyz_at`, the same arithmetic
`to_rgb_at` runs up to the one matrix that makes a pixel, so that a `Lab` or `CalRGB` colour
meets no screen on the way to a press), and a device colour's through the sRGB this processor
takes device colours to be (§10.3.2; `srgb_to_xyz_d50`, the inverse of the folded matrix and
transfer function, held to a tenth of a level). A `DeviceCMYK` colour inside the group keeps its
components, as §11.7.2 redefines it, and a colour in the press's own profile keeps them by
§8.6.5.7, both unchanged. The display list, the backends and the oracle's comparison are
untouched: the conversion in happens where a colour is resolved, and every backend sees the
inks.

`examples/press_census` prints, per press, whether the profile carries a `B2A` and whether this
tree evaluates it. Over the crawl's 145 archives every press a file names — 187 by a page
group's `/CS`, 94 by an output intent, 6 by a `/DefaultCMYK` — carries one and every one is
evaluated; `doc/pdf.js` names no profile press at all, all seven of its four-component page
groups being the assumed inks, so the change has no corpus witness there.

## The pages, looked at

Page one of every crawl document naming a profile press was rendered at one pixel per unit
with the branch-point build and with this one (`tmp/witness-877.sh` in the round's scratch,
under `tools/bounded.sh`): **148 of the 290 move**, by a mean of up to 9.4 of 255 and at most
110 on one pixel. A page that moves is one that paints a colour *not* in the press's own
components — a `DeviceRGB` or `ICCBased` 'RGB ' fill or image, a `Lab` — into it; a page of `k`
operators and CMYK images does not move at all (§11.7.2 keeps their components), which is what
the 142 that stayed are. Two of the largest, beside `mupdf` (ICC-enabled, and converting into
the group's profile) and `poppler` (which draws the page in device RGB), mean absolute
difference over the page in levels of 255:

| | ours before | ours after |
|---|---|---|
| `6942845.pdf` against `mupdf` | 11.90 | **9.58** |
| `6942845.pdf` against `poppler` | 7.40 | 15.41 |
| `0300111.pdf` against `mupdf` | 6.91 | **4.14** |
| `0300111.pdf` against `poppler` | 7.50 | 12.24 |

At one pixel of `0300111.pdf`'s green artwork the four are (0, 67, 31) before, (42, 85, 48)
after, (30, 85, 42) `mupdf`, (1, 85, 0) `poppler`. The reading moved this tree toward the
reference that takes the clause's route and away from the one that does not, which is the
direction principle 5 allows as evidence and no more: the pictures are the profile's
separations of sRGB, looked at, and they are a brochure and a poster in the colours a press
would print them.

## The mask, and what §11.5.3 says of a component

The second debt is §11.6.5.1's `/G` composited "in the colour space in which the compositing
computation is to be performed". For a device space that has been exact since ADR 0217; for a
CIE-based one this tree composited on the device and took the grey of the sRGB, a choice
`soft_mask::luminosity_departure` records. §11.5.3 is not silent about it:

> For CIE-based spaces, convert to the CIE 1931 XYZ space and use the Y component as the
> luminosity. This produces a colorimetrically correct luminosity.

For a space of **one** component the whole construction exists since ADR 0792: the group is
painted in its own component under `Compositing::Calibrated`, the backend composites that
number in three equal channels, and the luminosity is a function of one number — §8.6.5.2's
gamma times a white point whose `Y` Table 62 makes 1.0, or a profile's tone curve — so
`luminance_derivation` samples it onto the same 256-entry table `derivation` uses for the
device branch and composes Table 142's `/TR` after it. `/BC` is "n numbers, where n is the
number of components in the colour space specified by the CS entry", one number, and it is
painted as that component. Exact, and a real change: a component of 0.5 under `/Gamma 2` is a
luminosity of 0.25 where the sRGB route gave 0.54.

`bug1721218_reduced.pdf` is the corpus witness — its eight `/Luminosity` groups all state a
one-component profile — and the page moves by at most 5 of 255 on 666 of its 1 938 816 pixels
at two pixels per unit, because the masks are nearly binary and `Y` is the identity at both
ends. Rendered and looked at (trap 1): the artwork is the same router.

**Three components are not taken here, and that is a choice written down**: a `CalRGB` or an
RGB profile as a mask group's `/CS` — three groups in `doc/pdf.js` — still takes the grey of the
sRGB, because the clause's `Y` there is a three-curve sum per pixel a backend would have to
compute where every backend now reads a table, and a *four*-component profile group is
§11.4.7's pair inside a mask; `doc/todo/23` prices both.

## The choice, written as one

Nothing here is a choice between two readings — each clause states one route — but one thing
is chosen and should be legible: **a device colour's XYZ is sRGB's.** §10.3.2 has the processor
"establish CIE-based colour specifications for device colour spaces", and this tree established
sRGB for `DeviceRGB` in ADR 0009. The `B2A` route inherits that, so what a `1 0 0 rg` becomes
inside an `ICCBased` 'CMYK' group is the press's separation of sRGB red — which is what any
ICC-enabled processor with the same source assumption produces, and which differs from
`poppler`'s and `mupdf`'s exactly as far as their source assumptions differ from this one.

## What it cannot do, each reported or recorded

- A profile without a `B2A`: converts in by the right inverse of its `A2B` (ADR 0263), not
  reported — the clause's `shall` is the file's, and the crawl holds no such profile.
- Which intent's table beyond `1` over `0`: as for `A2B`, not selected (§8.6.5.8's row).
- A three-component CIE-based blending space, for a page or a group or a mask: composited on
  the device's three channels; named in §11.3.4's and §11.5.3's rows as what keeps them
  `partial`, beside the route-into-grey choice ADR 0790 recorded.
- A four-component profile as a *mask* group's `/CS`: composited on the device and its grey
  taken; recorded in §11.5.3's row.

## Consequences

- §11.3.4 stays `partial`, and what it names is now the three-component spaces and the
  route-into-grey choice — not `ICCBased` 'CMYK', which it had named on a claim eleven sessions
  stale. §8.6.5.5, §10.3.1, §10.4.2.1, §10.4.2.4, §11.7.2, §11.7.5.3, §11.6.6, §11.5.3 and
  §11.6.5.1 record the reading.
- `press_for_profile`'s argument that no `B2A` is needed is replaced by the clause that says
  one is, in the same comment.
- `clippy.toml` declares `AToB` and `BToA` as valid identifiers, because a quotation may not
  gain backticks and the lint would otherwise have to be silenced at each site.
