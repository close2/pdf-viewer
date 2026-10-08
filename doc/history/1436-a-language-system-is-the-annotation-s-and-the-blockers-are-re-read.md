# 1436 — A run is kerned in its annotation's language system, and four rows' blockers are re-read

Slot 5 of batch sixty-eight, 2026-10-08, a clause round and the batch's `partial`-row work. ADRs
1708 and 1709; Q332 not used. No row changed status: §7.4.7, §7.4.9, §12.7.4.3, §12.7.5.3,
§12.5.6.6 and §12.8.3.4.4 stay `partial`, each note with today's reading of its blocker. **Premises.**
Three held; `doc/todo/65`'s claim that §7.4.9's note dated each text did not.

**§12.7.4.3 (ADR 1708).** XFA's `locale` belongs to a template's `draw`, `field` and `subform` and
governs data formats and text direction, so it names no language system. §14.9.2 names the
language: the widget's or note's own Table 166 `/Lang`, else the catalog's. `pairs::Language` maps a
BCP 47 tag through ISO 639-1 to OpenType's language system tags. Both registries are vendored and
compiled in by `build.rs`. The script table's matching system replaces the default one. Four unit
tests and one `rich_text.rs` test (Turkish `TRK ` writes 97.65625, the default 146.48438); three
plants each failed. Both pages were rendered and looked at.

**§7.4.9.** PIMA 7667 (IS&T, USD 25) and CIE 131 are still sold; CIE 159 was withdrawn in 2022. A
non-D50 Lab's white point is stated (§8.6.5.4's EXAMPLE; the CIE's free CC BY-SA 4.0 spectra), so
that case waits on this tree: `ColourSpace::Lab` reads no `/WhitePoint` and draws every Lab as D50.

**§7.4.7.** The fork `close2/hayro` exists and is pinned; no branch of it, nor upstream's 0.3.1
(2026-10-04), carries the extended-template patch.

**§12.8.3.4.4 (ADR 1709).** The census covered the 1 706 documents with a `/ByteRange`. Sixteen
signatures name explicit policies under four identifiers. None names a syntax and none carries a
store. Thirteen sign under a Spanish policy whose archived PDF's SHA-1 is the signed digest. That
policy is human-readable, so a validator can bind and show it but not enforce it.

**Launch path.** The first table's slice pointers were 1 023 load-time relocations: an A/B of HEAD
`e833db88` counted 1 162.6 / 1 359.3 k without, 1 174.9 / 1 371.4 k with, past both ceilings. Plain
numbers now: 1 162.7 / 1 359.0 k, and 1 163.0 / 1 359.7 k on the shared tree (ADR 1708).

**Unfinished.** The fourteen wait on Q308; Chinese is not split by script or region; a structure
element's `/Lang` is not read; `Lab`'s white point and the policy fetch-and-show are later builds.

**Gates.** rustfmt `--check` on the seven touched sources: exit 0. `RUSTFLAGS="-D warnings" cargo
clippy -p pdf-font --all-targets`: exit 0; `-p pdf-model --all-targets`: exit 101 on
`tests/script_annotations.rs:135`, slot 1's new file, and every other target, listed: exit 0.
`cargo nextest run -p pdf-font`: 281 passed; `-p pdf-model` without `corpus`: 1987 passed, 18
skipped. `pdf-model --test corpus -- --ignored` behind the lock as `--tree 6`: exit 0, 43 s, 1.71
GiB; the census held 67 s. `cargo test -p conformance --no-fail-fast`: exit 0, 412 passed.
