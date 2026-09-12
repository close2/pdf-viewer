# ADR 1008 — A source of meaning travels with the conversion it changes, and a shared profile is a pointer

Status: accepted, 2026-09-12. Session 987.

## Context

ADR 1001 found that §14.11.5's output intent — the second of the three sources this tree ranks
for what a device colour space means, between §8.6.5.6's default and the device space itself —
reached the operator route and no other. It moved the ranking into `ColourSpace::device_family`
and carried the intent to `cs`, `k` and their kin through `ColourSpace::parse_with_output_intent`,
and it named what it could not reach: an image's `/ColorSpace` (§8.9.5.1, Table 87), an inline
image's abbreviated one (§8.9.7), a shading's (§8.7.4.3, Table 77) and the vertex colours a mesh
reads in it (§8.7.4.5.5), and a `/Luminosity` mask group's `/CS` (§11.6.5.1). Every one of them
still called `ColourSpace::parse`, which asks the default and stops, so on a page carrying a
readable `/DestOutputProfile` an image in `DeviceCMYK` drew through the assumed press beside a fill
drawn through the document's own.

It also priced what carrying the intent costs: `ColourSpace::Icc` held a `Box<Profile>`, and
`device_family` answers with a *copy* of the intent for every device space it substitutes — once
per `k`, once per `cs`, once more into the graphics state.

## What was measured

The copy first, before touching it, because the brief asked for a benchmark rather than an
assumption and the text-heavy page is the case that pays it. A fixture of 4000 `k` fills under the
one CMYK output intent the `doc/pdf.js` corpus carries — `issue20513.pdf`'s 718 KB v2 press
profile, `A2B` and `B2A` tables of 11⁴ and 25³ grid points — interpreted under callgrind:

| page | instructions before | after | wall before | after |
|---|---|---|---|---|
| 4000 `k`, no intent | 27.7 M | 27.7 M | 5–6 ms | 6 ms |
| 4000 `k`, the intent | **1 799 M** | **49.8 M** | 85–100 ms | 9.3 ms |
| 4000 `/DeviceCMYK cs … scn`, the intent | **3 568 M** | **71.6 M** | 100–118 ms | 11.3 ms |

Before, 95.4% of the intent page's instructions were one libc routine — the copy of the profile's
tables, about 430 K instructions per operator — and the `cs` form paid it twice. After, what is
left under the intent is the conversion itself: `Lut::apply` at 12.7 M, 3.2 K instructions per
colour, and one `parse_mft` of the profile at 3.6 M. Thirty-six times fewer instructions on the
operator route, for a change whose whole cost is a refcount where a `Box` was.

## Decision

- **`ColourSpace::Icc` holds an `Arc<Profile>`.** Every construction site — `parse_icc_based`,
  `first_usable_intent`, `codestream_colour_space` — was this round's, and the measurement above
  is the comment the variant now carries. `Press` keeps its own boxed copy of a bidirectional
  profile, which is one per press per process and was never on the operator route.

- **The intent travels in `Conversion`.** An image's, a shading's and a mesh's colour spaces are
  parsed where their samples are converted — `crate::image`, `crate::shading`, `crate::mesh` —
  after the interpreter has handed the work over, and two parameters already made that journey in
  one value: the compositing target and §8.6.5.9's black point. The intent is a third thing that
  is not a property of the colour, and putting it beside the other two is what ADR 0289 did for the
  black point: the omission becomes a type error rather than a habit, because there is no
  conversion a route can be handed that does not say what the page's device spaces mean.
  `Interpreter::conversion_under` and `Interpreter::image_conversion` put it on;
  `image::colour_space`, the JPEG 2000 route's declared space, `shading::Cache::space_of` and
  `shading::kind_of` read it off `Conversion::output_intent`. The alternative — a fifth argument on
  `Cache::build`, `decode_parts`, `decode` and their callers — reached three files this round did
  not own for a value those callers already pass in another form.

  What `Conversion` stores is the profile, not the space: Table 401 makes an intent's
  `/DestOutputProfile` an ICC profile stream and nothing else, and the caches keyed on a conversion
  — `shading::Cache`, `image::RasterCache` — need an identity to compare, which
  `Profile::identity` is. The derived `Eq`, `Ord` and `Hash` became written ones over
  `(target, black point, intent identity)`, `Compositing`'s construction one level up.

- **`Conversion::device()` carries no intent, and that is a decision at four sites.** A colour-key
  mask, an `/SMask` image, a `/Matte` and a stencil are read for coverage or for a count, and a
  `'GRAY'` intent must not bend an alpha through a tone curve. The same reading answers the two
  parses that stay on `ColourSpace::parse`: `image::short_of_its_grid` and
  `inline_image::unfiltered_length` ask for a component count, and `device_family` substitutes an
  intent only for a family with as many components, so the count is the same under any intent.

- **The soft mask's function reads the intent; its caller does not yet hand it over.**
  `soft_mask::entry_with_output_intent` takes it, `luminosity` parses the group's `/CS` through it,
  and a `/DeviceCMYK` group on a page with a four-component intent is composited in that press with
  §11.5.3's `Y` off the press's grid — the answer `transparency::page_press` already gives the
  page's own group. `soft_mask::entry` is the same function with `None` and exists for one line,
  `content/ext_gstate.rs`'s `gs`, which was a neighbour's file in this session. When that line
  passes `self.output_intent.as_ref()` to the new function, `entry` goes, and §14.11.5's row —
  `partial` for exactly that line — closes. Its unit test is in `soft_mask.rs` because the
  interpreter-level fixture cannot pass until then, and a test that cannot pass is not committed.

- **`'Lab '` data-space profiles stay refused, with the reason written where the refusal is
  made.** ISO 15076-1 was read for it, and the answer is not in the table type: a Lab-input `A2B`
  is the same `mft2` or `mAB ` as a CMYK one. It is in the *input encoding*. The lookup-table
  clauses define a Lab encoding for the connection-space side of a table only and say outright
  that the definition does not reach the header's data-colour-space field, so a Lab device value
  lands on the table's 0..1 input by whatever scale the profile's maker assumed — and the one
  statement of that scale a PDF carries is Table 66's `/Range`, "[t]hese values shall match the
  information in the ICC profile". This tree does not read it. **And §8.6.5.5's row said it did**,
  for nine hundred sessions, beside a comment in `ColourSpace::initial_colour` that had said it
  did not the whole time; the row is corrected. For `'GRAY'`, `'RGB '` and `'CMYK'` the entry is
  the identity by Table 66's own default, which is why nothing has drawn wrong for want of it; for
  `'Lab '` it is the whole of what is missing, and it belongs to `ColourSpace::Icc` rather than to
  `Profile::parse`. The population is measured: a scan of every directly filtered stream in the
  1249 documents across `doc/pdf.js/test/pdfs`, `doc/corpora/` and `doc/corpora-own/` found 333
  embedded profiles — 235 `'RGB '`, 95 `'GRAY'`, 3 `'CMYK'` — and no `'Lab '`. A conformant file's
  `/Alternate` for one is `[/Lab …]`, so what refusing costs is the profile's own CIELAB model
  against §8.6.5.4's, on no file this disk holds.

## What moved, and what was looked at

`examples/raster_digest` over the 974 `doc/pdf.js` first pages differs on **one** page,
`issue20513.pdf`, the only document in that corpus whose intent is a four-component profile: its
two `/DeviceCMYK` axial shadings and one `/DeviceCMYK` image now draw through the same press as
the fills beside them. 6 384 of 1 767 168 pixels, at most 53 levels, all inside the shaded
lettering of a logo at the page's top right; both arms were rendered and the crops read the same
words in the same shapes, the outline a warmer brown through the press than through the assumed
inks. The ten other documents stating a `/DestOutputProfile` in that corpus carry three-component
profiles under pages that select no device space by name, and their digests are byte-identical.
ADR 1001's finding that the corpus cannot see this class was true of the *operator* defect and is
false of this one by exactly one page, which is worth knowing: the shading and image routes are
the ones real files put a device space on.

The new integration test drives one cyan through an image XObject, an inline image, an axial
shading by `sh` and as a pattern, and a type 4 mesh whose `/ColorSpace` is an indirect reference,
and holds every route to the operator's colour by name. Calibrated by dropping the intent from the
conversion, under which all five answer the assumed press's `(0, 173, 239)`; with it, all five
answer the intent's green.

## Found and not fixed

`apply_soft_mask` decodes an image's `/SMask` under `Conversion::device()`, so no intent reaches it
— but it is handed the resources in force, and `image::colour_space` asks §8.6.5.6's `/DefaultGray`
of them for the mask's `/DeviceGray`. A `CalGray` default with a gamma would remap an *alpha* as if
it were a grey. No `doc/pdf.js` document states a `/DefaultGray`; the fix is the mask's own decode
reading no resource dictionary, since Table 143's `/ColorSpace` "shall be DeviceGray" and that name
never refers to one. §11.6.5.2's row carries it as a fourth residue.

`cargo fmt --all --check` failed, while this round ran, on a neighbour's temporary probe file under
`crates/pdf-syntax/tests/` (session 990's `zz_find_990`, deleted before that round finished — no such
file exists); this round's crate is formatted with `cargo fmt -p pdf-model` and the unscoped check is
reported rather than repaired, which is `doc/habits/tests-gates-and-reports.md`'s rule.

## The habit

**A parameter that changes what a value means travels with the value, not beside it.** Two
parameters already went from the interpreter to the three late-converting routes in one struct,
and the third was left behind for eight sessions because it was added to a *function signature*
one route took and not to the *value* every route took. The instrument is ADR 1001's grep for the
field's name, and the fix is the one ADR 0289 used: put it where the omission cannot compile.

**And measure the copy before believing it is small.** A `Box` in a variant that is cloned per
operator is a deep copy per operator; here it was 95% of a page, on the route that a text-heavy
document under a press profile takes for every glyph run, and it had been described as a
refcount-shaped cost since ADR 1001 without a number. The number took one fixture and two
callgrind runs.
