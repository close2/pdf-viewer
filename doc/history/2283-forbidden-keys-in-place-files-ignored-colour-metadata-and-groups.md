# Round 2283 — review 2282's fix groups: forbidden keys, files in place, ignored colour, metadata and groups

A fix round in the `lane3` worktree for audit 2282 (ISO 19005-2 and -4 clause 6): its five
behaviour findings and its documentation finding, each with the converter's answer.

## What changed

- **B01** (ADR 3833): every rule forbidding a key asks `table::states`; the two private `keys`
  became `table::stated_keys`, which skips an entry resolving to null (§7.3.10, §7.3.7). The
  action removal reads `/A` and `/AA` the same way.
- **B02, D01** (ADR 3834): the embedded-file walk visits every dictionary carrying `/EF` inside a
  cross-referenced object, reported at its holder. The converter completes `/F`, `/UF` and
  `/AFRelationship` in place; its removal refuses by name for a specification written in place
  unless 6.3.1's removal takes the holder. The `Order` row names a group `object N G`.
- **B03**: `set_device_colour`, `set_space` and the initial-space record return where
  `State::uncoloured` holds.
- **B04** (ADR 3835): the metadata population adds every stream a `/Metadata` entry names; a new
  row `metadata/metadata-streams-state-their-type`, answered by `Rewrite::MetadataStreamType`.
- **B05** (ADR 3836): `pdf_archive::optional_content_groups` is `/OCGs` and every `/Type /OCG`
  object a reference names where a group stands; the row reports an `/OCGs` gap too, and the
  converter completes both arrays. A first census by `/Type` alone made `issue9949.pdf`'s `/Usage`
  dictionary (Illustrator's, typed `/OCG`) a nameless layer, and veraPDF failed it on 6.9-3.

## Ledger

ISO 32000-2 rows unchanged before and after; the 19005 coverage claims 2282 named now hold, its
findings are `fixed` and `expected-failures-current.json` no longer lists them.

**Gates.** Over `-p pdf-syntax -p pdf-model -p pdf-archive -p pdf-transform -p viewer-qt -p
conformance`: fmt 0; clippy 101 (`explicit_iter_loop`, `doc_markdown`, `implicit_clone`,
`too_many_lines`), fixed, then 0; `pdf-sandbox` 0; nextest 5938 run, 0 failed, the manifest's 0;
archive and transform again after the census fix, 1834 passed; rustdoc 0; conformance 0; fuzz 0.
Census against 2275, pack and none: no document moves. veraPDF 5471 → 5467 compliant, the four
`issue9949` outputs, 4 of 4 after the fix; fixtures 3 of 3. Corpus 0 missed 0 over. Launch 101:
`peak_anon_mib` over for BPC, xfa and opt_demo against a device baseline of 11.40 MiB (2276, 2281:
12.7); nothing this round touched runs on the launch path. Owed: a rerun.

## Habit

Count a dictionary by where a reference to it stands, never by the `/Type` it states.
