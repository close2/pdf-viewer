# Specification audit of ISO 32000-2 §8.6 and §8.7 (colour spaces and patterns) — round 2184

The project's eleventh specification audit, made beside the other lanes in the shape of the
owner's review 2038 ([`2038-specification-audit.md`](2038-specification-audit.md)) and of audits
2128 to 2180 ([`2180-specification-audit-9-5-9-10.md`](2180-specification-audit-9-5-9-10.md)).
Baseline: main at `516e71df`. Evidence:
[`2184-specification-audit-8-6-8-7/`](2184-specification-audit-8-6-8-7/) — `findings.json`
(2038's schema) and `expected-failures.json` (the failing tests of each behaviour finding). The
same entries are in `2038-specification-audit/expected-failures-current.json`.

**This is an audit and fixes nothing.** A fix round takes the findings by the groups at the end.

## Method

1. Every `shall` and `should` of §8.6 and §8.7 (Tables 61 to 84) was read in
   `doc/md/ISO_32000-2_sponsored_EC3.md`, lines 4797 to 6237, extracted sentence by sentence
   (468 of them) and set against the code that reads the entry or performs the step. Most are
   constraints on the file; those were read for what a reader does when a file breaks them, and
   only a silent wrong result was taken further.
2. 2038's findings on these clauses were read first and are not repeated: B20, B25, B70, B71,
   B72 (all fixed), T27 to T169 and D11, D15, D54, D61, D64.
3. The readers traced: `pdf-colour`'s `colour/parsing.rs` (every family, Table 65's three
   answers, the `Indexed` table's scaling), `colour/separation.rs` (`All`, `None`, Table 70's
   `/Colorants` and Table 71's `/Process`), `colour/queries.rs` (component ranges, §8.6.8's
   initial colours, `entry_of`'s rounding and `hival` clamp), `colour/to_rgb.rs` and
   `to_cmyk.rs`, `icc.rs`'s header reading, `shading.rs` (`/Domain`, `/Extend`, `/Function` as
   one or a group, `/BBox`, `/Background`) and `mesh.rs` (Tables 81 to 84: bit widths, edge
   flags, `/Decode`, padding, the lattice's triplets); `pdf-model`'s `content/colour.rs` (`cs`,
   `sc`/`scn`, the operand-count recovery, `cmyk_tints`), `content/pattern/` (tiling, the cell's
   state, the uncoloured binding, shading patterns and `sh`), `content/ext_gstate.rs` and
   `content/run.rs` (§8.6.8's restriction), `content/overprint.rs` (§8.6.7).
4. The lens was the earlier audits' habits: a rule keyed on a name applies wherever the name is
   read (§8.6.8's list of `gs` entries, read in one place by two guards); list every route to a
   thing (Table 65's alternate is reached two ways, §11.7.4.3 NOTE 2's current colour space
   three); an absent entry is a third answer (`/Alternate` omitted).
5. Each candidate was probed through the public API (`pdf_model::interpret` and the CPU
   oracle), and each B finding has a failing `*_spec_audit` test whose control passes on main:
   `pdf-model/tests/colour_8_6_8_7_spec_audit.rs`, a module of `colour_suite`.

What holds, probed or traced, is not listed one by one. Among it: §8.6.8's initial colours for
every family, the `Lab` and `ICCBased` ones held to `/Range`; `Indexed` rounding half up, the
`hival` clamp on both sides and Table 88's index `/Decode`; the table scaled onto a `Lab` base's
ranges; `All` and `None` decided before the alternate and tint transform are read, and a
`DeviceN` of `None` alone discarding its output; `NChannel`'s per-component evaluation through
`/Process`; Table 65's `/Alternate` held to `/Range` first where it is stated (ADR 2973);
`/Background` ignored by `sh` and painted by a pattern; `/BBox` as a clip on both routes; mesh
edge flags read in their low two bits and a type 4 strip's continuation order; Type 5's triplets
as §8.7.4.5.6 lists them; §8.6.8's operator list and image rule inside a `d1` glyph and an
uncoloured cell, and `/TR`, `/TR2`, `/HT`, `/UseBlackPtComp` and `/RI` ignored there; a
negative step tiled as the same lattice as its absolute value.

Readings left, each a question about the standard rather than a defect found: Table 65 says
"there shall not be conversion of source colour values, such as a tint transformation, when
using the alternate colour space" and also admits any non-`Pattern` space as the alternate, so
a `Separation` alternate has no reading that obeys both; §8.6.6.4's `All` inside a `DeviceCMYK`
blending space is drawn as the grey `1 − t` converted, where "all available colourants" could
be read as the group's four; and §8.6.6.5's reserved names `Cyan` … `Black`, "always … process
colours" that "need not have entries in the process dictionary", make an `NChannel` space whose
`/Components` spells them otherwise revert to its tint transform, which NOTE 3 permits.

## Findings

Four behaviour gaps. In order of how much a person would see:

- **2184-B04** — a `Pattern` space read where it carries no colour is drawn as opaque black and
  the page reports itself complete: a shading whose `/ColorSpace` is `/Pattern` (the `8.7.4.3`
  row says it is refused), an `Indexed` space over a `Pattern` base, and a bare number given to
  `scn` in a `Pattern` space, which fills black where the initial pattern paints nothing.
- **2184-B02** — an `ICCBased` space with an unreadable profile and no `/Alternate` becomes the
  bare device space `/N` names, so it starts at `DeviceCMYK`'s black instead of the `ICCBased`
  all-zero, and its `/Range` is not applied first; the stated-`/Alternate` route does both.
- **2184-B03** — nonzero overprint mode skips an `Indexed` colour whose base is `DeviceCMYK`;
  §11.7.4.3 NOTE 2 makes the base the current colour space, beside the `Separation` reversion
  the code already follows.
- **2184-B01** — a `d1` glyph's `gs` stating `/BG2` and `/UCR2` converts the text's colour into a
  `DeviceCMYK` group through that pair, where §8.6.8 lists both among the entries "shall be
  ignored"; the read sits outside the guard `/TR` and `/UseBlackPtComp` are behind.

Two documentation findings: D01, the comment beside that guard says the four black-generation
entries are "read for a flag and never evaluated", which is false and is why they were left
out; D02, a zero or absent `/XStep` or `/YStep` is tiled at the `/BBox`'s extent, silently, on
the reason "what other readers assume".

## Ledger rows contradicted

All are `implemented` and stay so until the fix round, which re-reads each: `8.7.4.3` (B04: "a
`Pattern` space is refused as the table requires"), `8.6.6.3` (B04's `Indexed` base), `8.6.8`
(B04's `scn`; B01: "every /ExtGState entry" of the list ignored), `8.6.5.5` (B02), `8.6.7` (B03).

## Suggested fix groups

1. **`pdf-model/src/content/ext_gstate.rs`** — B01 and D01: the black-generation read behind
   `!self.uncoloured`, the comment restated.
2. **`pdf-colour/src/colour/parsing.rs`'s `parse_icc_based`** — B02: the `/N` device space
   wrapped in the same `IccAlternate` as a stated `/Alternate`.
3. **`pdf-model/src/content/colour.rs`'s `cmyk_tints` and `reverts_to_cmyk`** — B03: an
   `Indexed` arm asking the base with `entry_of`'s entry.
4. **`pdf-colour`'s `Pattern` space** (`shading.rs`'s `kind_of`, `parsing.rs`'s `parse_indexed`,
   `to_rgb.rs`, `queries.rs`) with `pdf-model`'s `set_colour` — B04: refused and reported where a
   clause excludes it, and no colour for a number in a `Pattern` space.
5. **`pdf-model/src/content/pattern/tiling.rs`** — D02: the step fallback written as a choice
   with its reason, and reported, or the pattern refused by name.
