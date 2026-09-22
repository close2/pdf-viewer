# 1241 — A space that reverts is the space it reverts to, and §8.6.7's EXAMPLE says so

Status: accepted. Session 1202.
Context: ISO 32000-2 §11.7.4.3 (NOTE 2, Table 146), §8.6.7 (its EXAMPLE), §8.6.6.4, §8.6.6.5,
§10.8.2, §11.7.2, §11.7.3; ADRs 1157 (amended here), 1158, 1169, 1170, 1178, 1181, 1182,
1229 section 6 (which found the defect).
Code: `crates/pdf-model/src/content/colour.rs`, `crates/pdf-model/src/content/overprint.rs`.
Tests: `crates/pdf-model/tests/overprint.rs::two_separation_spaces_overprinting_are_the_colour_the_alternate_names`,
`::the_overprint_clauses_own_example_is_an_equivalence`.

## 1. The defect

`content::colour::cmyk_tints` answered `Some` only for a literal `ColourSpace::Cmyk`, so
`Interpreter::special_overprint` bailed for every `Separation` and `DeviceN` space, whatever it
reverted to. §10.8.2's own example measures the cost: cyan then yellow over one area, with
`/OP true /OPM 1` inside a `DeviceCMYK` page group, came out **green** written `1 0 0 0 k` /
`0 0 1 0 k` and **yellow** written through `/Separation /Cyan /DeviceCMYK` and
`/Separation /Yellow /DeviceCMYK` — the two spellings of one page, drawn as two pages.

§11.7.4.3's row claimed the opposite in its own note: "NOTE 2 closes the route that might have
escaped". NOTE 2 states the rule; nothing carried it out.

## 2. What the clause says, and what ADR 1157 got wrong about it

NOTE 2 names three spaces whose current colour space is not the one the `cs` operator selected:

> In the previous descriptions, the term current colour space refers to the colour space used
> for a painting operation. This can be specified by the current colour space parameter in the
> graphics state (see 8.6.2, "Colour values"), implicitly by colour operators such as rg
> (8.6.8, "Colour operators"), or by the ColorSpace entry of an image XObject (8.9.5, "Image
> dictionaries"). In the case of an Indexed space, it refers to the base colour space (see
> 8.6.6.3, "Indexed colour spaces"); likewise for Separation and DeviceN spaces that revert to
> their alternate colour space, as described under 8.6.6.4, "Separation colour spaces" and
> 8.6.6.5, "DeviceN colour spaces".

ADR 1157 section 4 read NOTE 2 and set it aside, on two arguments. **Both fail.**

- *"The four numbers the zero test is about are then the tint transform's output rather than
  anything 'defined within the PDF file'."* §8.6.7's sentence is about **precision**, and its
  own second half says which contrast it is drawing: the determination "shall be made on the
  tint value defined within the PDF file, **before quantisation into a device tint value for
  the output device**". A tint transform's output is a float the file's own function defines;
  it is the device tint the sentence excludes, not a computed one.
- *"Table 146 puts such a space in its own rows."* Table 146's `Separation or DeviceN` rows are
  for a space that is **still** a `Separation` when the blend function is evaluated — one whose
  spot colourant the group's space maintains. §11.7.3 makes that unreachable here ("[i]f any
  other colour space has been specified for the group, the Separation or DeviceN colour space
  shall be converted to its alternate colour space"), which ADR 1157 section 3 had already
  established for row 6. A space that reverts is not in those rows *because* it reverted; NOTE 2
  says where it goes instead.

**And §8.6.7's EXAMPLE settles it without either inference.** Under `OP true` and `OPM 1` the
clause states an equivalence:

> EXAMPLE If the overprint parameter is true and the overprint mode is 1, the operation 0.2 0.3
> 0.0 1.0 k is equivalent to 0.2 0.3 1.0 scn in the colour space shown in this example.

The space it shows is `[/DeviceN [/Cyan /Magenta /Black] /DeviceCMYK <tint transform>]` with the
transform `{ 0 exch }`, which inserts the zero yellow. Every component name is a process one, so
that space reverts (§8.6.6.5). Two operators a clause calls equivalent may not take different
blend functions — and as built, the `k` took the special mode and the `scn` took Normal.

## 3. Table 146 row 1 and row 4 stay apart

Row 1 — "DeviceCMYK , specified directly, not in a sampled image" — is what a reverting space
over a `DeviceCMYK` alternate is in, on the four components the alternate receives. Row 4 —
"Any process colour space (including other cases of DeviceCMYK)", `C_s` in all three columns —
is what a space reverting to a **different** process space is in, and that is the first bullet's
own condition doing the work: it needs "the current colour space and group colour space [to be]
both DeviceCMYK", and a `Separation` over a `DeviceRGB` alternate painted into a `DeviceCMYK`
group satisfies neither half. Nothing about row 4 changes here.

## 4. The construction

`cmyk_tints` recurses: a `ColourSpace::Separation` answers with its alternate's tints,
`Tints::eval(values)` applied, and the base case is the literal `ColourSpace::Cmyk` it always
was. `ColourSpace::Separation` is *exactly* "a Separation or DeviceN space that reverts" — the
variants that do not revert are `AllColourants` (§8.6.6.4 requires the alternate to be ignored),
`NoColourant` (never painted) and `Simulated` (§10.8.3's four steps, not a reversion) — so the
condition needs no second test.

**The transform is evaluated only where its answer is wanted.** `cmyk_tints` runs beside every
`sc`, `scn` and `cs`, and a tint transform is a function the operator has already evaluated once
for its colour; asking `reverts_to_cmyk` first — discriminants only, nothing evaluated — means a
page painting in a `Separation` over a `Lab`, `DeviceRGB` or `ICCBased` alternate pays a
comparison rather than a second evaluation. It is not an optimisation of a measured hot path but
a refusal to compute a value the next line discards, which is why it carries no number.

Nothing else moves. The zero test, the half-local question ADR 1169 corrected, the implicit
group of ADR 1170, the verdict ADR 1181 settles and the Porter-Duff reading of ADR 1182 all take
the tints as they find them.

## 5. What this does not build, and why it is not the clause's fault

NOTE 2's **`Indexed`** clause is unanswered: an `sc` in an `Indexed` space over a `DeviceCMYK`
base has `DeviceCMYK` for a current colour space, and its tints are the table's entries, which
are bytes in the file. Reading them needs `ColourSpace::entry_of`, which is `pub(crate)` to
`pdf-colour` on purpose — §8.6.6.3's index is rounded and clamped in one place, and a second
reading here would be a second chance to round it differently. That crate is not this round's,
so the entry is named rather than half-built: the work is to make `entry_of` public and add the
arm, with a fixture of the shape the two above have.

## 6. Consequences

- `doc/pdf.js`'s 974 first pages are **unmoved**: `raster_golden` held 974 and moved 0. The
  defect is real and the corpus does not witness it, which is `doc/todo/02` §7's inverse — a
  count that does not move is not evidence that nothing happened.
- **The population the two refusing backends lose grew by 11 documents and 27 pages**, measured
  rather than guessed: `examples/overprint_ink_group_census` re-run over the same 65 944 crawled
  documents ADR 1178 walked, 2927 s, peak 2.32 GiB, the same 65 720 opening and the same 17 134
  interpreted over 248 615 pages. **1799 documents and 9890 pages paint under the mode**
  (1788 and 9863), 27 462 715 marks (27 435 261); **146 documents and 494 pages** under a
  non-Normal blend mode (145 and 493). The census's own check stays at zero — no page carries the
  verdict without a mark, and no page carries an `Unsupported::Overprint`.
- **The shape of the kept set moved with it, and against `render-raster`**: 13 134 marks now keep
  a proper subset rather than 12 486, and **1702 of 1799 documents state none at all** where it
  was 1711 of 1788 — so `Compose::DestOver` alone covers 94.6% of what the refusal costs rather
  than 95.7%, and the per-channel choice is 97 documents and 220 pages. `doc/todo/23` and
  `doc/QUORRA_FEEDBACK.md` section 49 carry the figures the ask is written from.
- §11.7.4.3's row keeps `implemented` and its note loses the sentence that was false.
