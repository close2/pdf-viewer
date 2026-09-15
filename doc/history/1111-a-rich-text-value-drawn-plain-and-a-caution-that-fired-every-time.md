# 1111 — a rich-text value drawn plain, and a caution that fired every time

2026-09-15. ADR 1122. Contract: §12.7's fourteen `partial` rows, sorted three ways.

**The sort.** (a) Buildable this round, and built: §12.7.5.3 bit 26 `RichText`. (a) Buildable but
found already smaller than the note said, so rewritten not rebuilt: §12.7.6.2's `<f href>` residue,
§12.7.4.1's `/TM`. (b) Excluded and checked for decay only, unchanged: §12.7.8.3.1's `/JavaScript`
and §12.7.8.3.2's `/RV`/`/A`/`/AA` (ECMAScript), §12.7.5.5 (signing, owed the day this tree signs).
(c) None was genuinely the owner's, so no Q66. The other §12.7 partials (§12.7.4.3, §12.7.5,
§12.7.5.4, §12.7.6, §12.7.8.3.3, §12.7.8.3.4) each rest on a residue outside this round's diff and
were read, not touched.

**Census first (trap 8, trap 13).** `examples/field_flag_census` gained §12.7.5.3 bit 26's own
population — the flag, the flag with Table 228's `/RV`, and both in a `/NeedAppearances` document —
calibrated on four planted files (3/2/1, a non-rich control counted nowhere). Curated 1452 docs:
5 / 4 / **0**. Crawl 65 720 docs: 252 / 33 / **0**. No untouched page is reached, so the report
costs nothing gated.

**§12.7.5.3 bit 26, built (ADR 1122).** The row read the flag's two `shall`s as "addressed to the
file"; §12.7.4.3 writes a third at the processor — "the entire annotation appearance shall be
regenerated each time the value is changed" — and that appearance is XFA, which `CLAUDE.md`
excludes. So the plain `/V` is drawn and the formatting is *reported* (`Owed::RichTextFormatting`),
on the `/RV` condition and not the flag alone (trap 11); the flag without an `/RV` owes nothing.
Read up the field's own `/Parent` chain, since `/RV` is not inheritable.

**§12.7.6.2's `<f href>`, a trap-39 correction.** ADR 1108's writer put `<f href> not written` on
`Submission::owed` for *every* XFDF submission. ISO 19444-1 section 5.6.2 requires no `<f>` and
section 5.6.3's own exported form has none, so nothing is owed — a caution wearing a signal's
clothes. Removed from `owed`; its test now asserts the absence. Row note rewritten to match.

**Notes rewritten to what is left.** §12.7.5.3's bit-26 disposal (report, not hand-off);
§12.7.6.2's `<f href>` (owed → not owed); §12.7.4.1's `/TM` ("named by nothing outside §12.7.6.2's
excluded export" → named by `submission::mapping_name`, the export built since session 1047). All
three stayed `partial`; no status moved, no other row touched, header untouched (1105 owns those).

**Also corrected:** `Owed::TransformedTextMatrix`'s doc and `detail()` string, stale since ADR 1114
closed the scaling half — it now names a rotation/skew/mirror, not "scales or rotates".

Gates (siblings share the tree, so clippy/fmt/records are scoped to mine). rustfmt --check my six files 0; clippy -p pdf-model -D warnings — 0 in my modules, 3 in siblings' image.rs/viewer_preferences.rs; nextest -p pdf-model --lib 493, --test variable_text --test submission 99+31, --doc 0; conformance quotation/ledger checker 7 passed (my new quotes verify). Tier 2 under the lock: --test corpus 0, every ratchet at ceiling; --test actions 2; raster_golden held 974 moved 0; viewer-core 211. batch.sh check exit 1 — all of it siblings' (over-budget 1108/1109/1110, their scratch); my record 39 lines, ledger no \uXXXX, fmt clean.
