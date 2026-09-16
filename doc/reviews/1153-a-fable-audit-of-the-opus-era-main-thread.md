# 1153 — A Fable audit of the Opus-era main thread

Session 1153, Fable 5.1, asked for by the owner on 2026-09-16: the orchestrator's own work in `778bf48a..322c69ba`
(batches sixteen to twenty-two and one governing-document commit), audited on `main` at `322c69ba`. Nothing was fixed;
one `cargo clean -p conformance` was run, as briefed.

## 1. Gates on `main`, every line, exit codes as run

Tier 1: `cargo fmt --all --check` 0 · `RUSTFLAGS=-D warnings cargo clippy --workspace --all-targets` 0 · `cargo
nextest run --workspace` 0 (5013 passed, 38 skipped) · `cargo test --workspace --doc` 0 · fuzz fmt 0 · fuzz clippy
with `-D warnings` 0 · `cargo test -p conformance` 0 (259 passed; the stale binary was real, `cargo clean -p
conformance` removed 958 files first). Tiers 2 and 3, each alone under `flock` + `bounded.sh`, `RAYON_NUM_THREADS=4`,
foreground: `pdf-model --test corpus` 0 · `raster_golden` 0 · `oracle` 0 · `render-raster --test corpus` 0 ·
`viewer-core --test accessibility_census` 0 (every floor at slack 0) · `pdf-transform --test gate` 0 ·
`viewer-confined --test awkward_classes` 0; the three `--bins` builds ran first, each 0. The ledger binary prints
568/99/14/11 — the figures `d20ccab0` claims.

## 2. History: nothing forbidden left a mark

`git log --merges` over the range is empty; the reflog is seven `Fast-forward` entries and one commit, no reset; no
`:120000`/`:160000` entry at any commit; the tree is clean. **`stash@{0}`** ("WIP on
worktree-agent-aae6c9d70f66ccb90", base `ada5411`, 2026-08-14) holds 341 insertions in
`crates/pdf-font/src/{loading,metrics}.rs`: the `substitute_stretch` work, which landed two hours later as `b5c1f180`
and differs from the stash by 15 lines — a superseded draft, safe to drop. **Double launch:** every path and
identifier records 1105 and 1107–1111 name is in `b04a4d33`; no kept round touched
`crates/pdf-transform/src/archive/`; nothing was lost.

## 3. Quotations against `doc/md/ISO_32000-2_sponsored_EC3.md`

All 615 strings of 30+ characters added in ledger notes and crate sources were checked (brackets, curly quotes,
ellipses, case normalised): 108 verbatim, the rest test messages or note fragments, none a failed claim on the
standard. Sixteen read by hand, each verbatim under its heading: "shall always deliver 8-bit samples" §8.9.5.1;
"DCTDecode may obtain the parameter values it requires…" §7.4.8; "This entry shall apply only if EndOfLine is true and
K is non-negative." §7.4.6; "A PDF reader shall invoke the corresponding decoding filter…" §7.4.1; "A colour space
shall be defined by an array object…" §8.6.3; "shall return a number in the same range" §10.5; "if present, shall be
Trans…" §12.4.4.1; "A given annotation dictionary shall be referenced from the Annots array of only one page."
§12.5.2; "The topmost object at any point shall be defined…" §11.7.5.2; "the check box in the print dialogue
associated with input paper tray shall be checked" §12.2; "A PDF processor shall ensure that the backdrop's
contribution… is applied only once" §11.4.3; "the XObject stream dictionary shall contain an AF entry…" §14.13.7; "If
stroke adjustment is enabled…" §10.7.5; "A PDF processor is not required to process pronunciation hints." §14.9.6;
"[t]he first page of the PDF file shall be denoted by 1" §12.2; "In the last bead, this entry shall refer to the first
bead." §12.4.3. One quote is outside `doc/md/` and says so: "All XMP metadata in PDF shall be encoded as UTF-8." is
Errata Collection 3 Issue #296, quoted as an erratum (ADR 0601).

## 4. The twenty-four status moves, each against its note and ADR 1119

Fourteen `partial` → `departed` (§7.4.2, §7.4.8, §7.5.5, §7.10.2, §8.5.3.3.1, §8.6.5.7, §10.4.2.5, §10.7.5, §12.4.4,
§12.4.4.1, §12.5.2, §12.5.5, §12.5.6.19, §12.11.6): **all right** — each first sentence names the declined sentence
and its ADR, none carries an undecided build (§12.5.6.19's "what this row owes" is its ADR 0245 narrative). §12.5.5 is
loose only in label: Table 166's own `CA`/`ca` say the value "shall not be used if the annotation has an appearance
stream", so the declined two-thirds is the standard contradicting §12.5.5, as the note says. Ten `partial` →
`implemented`, **all right**: §7.6.4.3.2 (false residue; `pad_password` appends); §8.9.5.1, §8.10.2, aggregate §8.10
(`/AF`: §14.13.7's `shall` is the producer's, §14.13.1–3 bind no reader); §12.4.3, aggregate §12.4 ("Interactive PDF
processors *may* provide navigation facilities", so an uncalled `beads_on_page` owes nothing); §14.6.2, §14.6;
§14.7.2, §14.7 (§14.9.6's permission, verbatim). Every aggregate's descendants are settled. The reversal of round
1112's §12.5.6.23 `departed` to `partial` is **right**: painted paths, forms and `JPXDecode` are owed builds.

## 5. What a message or document claims that the tree does not show

1. `3fd56049`: "§8.4.2 to `implemented`" — it was `implemented` at `778bf48a`; only its `code` array moved.
2. `b04a4d33`: the duplicate-launch recovery "is in … doc/todo/02 section 8" — §8 has only `6d3ab4a1`'s zombie/`xargs`
   text; nothing on a double launch.
3. `doc/questions/A66` line 2: "Given: 2026-09-14 … transcribed by the round". Q66 entered the tree in `a64214f2`
   (2026-09-16 05:56); A66 was written by the orchestrator in `322c69ba` (08:15), and `doc/todo/65` line 123 says
   2026-09-16. The date is impossible.
4. `doc/todo/64` still says both builds are "set aside unbuilt" and "owed"; batch seventeen built both (ADR 1123,
   1124). Right is a `Status: done` line as `doc/todo/39` carries, keeping the design notes.
5. `doc/history/1107` lines 26 and 37 name "ADR 1121, future" for relocation, which became ADR 1123 (1121 is the
   colour-key ADR); records 1114, 1126, 1131 say "no ADR 1125/1129/1133" for numbers now taken by other subjects.
   Records are records; a one-line erratum at the foot is the fix.
6. `doc/conformance/ledger.toml` line 3864: §12.5.6.23's `test` array omits two tests its note names
   (`a_jbig2_image_in_the_region_…`, `inline_image_samples_…_spliced`); both exist.
7. Confirmed: `6d3ab4a1`'s twelve floors are twelve raised constants with reasons (gate at slack 0); `322c69ba`'s
   CLAUDE.md edit removes exactly the sentence named and no other prohibition.

## 6. Verdict

Sound where it matters: every merge is a fast-forward of a tree the gates pass on `main`, every status move holds
under ADR 1119 and the one reversal was right, the recovery lost nothing, the quotations are verbatim. Wrong only at
the bookkeeping edges — items 1 to 6 above, none touching code or a gate.
