# 1319 — Deprecated metadata goes to the packet, and a merged root holds one `Document`

Ledger slot of batch forty-nine. ADRs 1473, 1474. No question written.

**Rows moved.** §14.3.1 `partial` → `implemented`, Annex L `partial` → `implemented`, and §6.2,
their aggregate, `partial` → `implemented`. `partial` 14 → 11, `implemented` 682 → 685 (counted with
`grep -c`; `tools/state.sh ledger` rewrites the file and was not run mid-batch).

**§14.3.1 (ADR 1473).** One `update::TABLE_349` for the three writers. `deprecated_in` sends every
entry but the two dates into §14.3.2's packet alone in a 2.0 file, and into both sources in an
earlier one. `restated_packet` removes then supplements, so the two never name two titles. An
update to a 2.0 file with no packet creates one and rewrites the catalog. Tier 3's `pdf-vfs
write_corpus` then caught `meta/info.json` reading only the dictionary, so a 2.0 file's written title
read back null. `update::stated_information` (dictionary, else the packet's counterpart, dates
excepted) is now the view and the write's "is this a change" test (ADR 1473 section 5).
§14.3.4's row was rewritten for the second writer, which now writes both sources.

**Annex L (ADR 1474).** `Carry::wrap_documents` puts PDF 2.0 `Document`s from more than one source
inside one `Document` of that namespace, reusing a listed namespace dictionary. The population is
`Tree::in_pdf_2_0_namespace`, the end of §14.8.6.2's role map. Not re-typed `DocumentFragment`:
which content a source's element encloses is the producer's statement.

**The twelve.** §7.4.7: the patch's `Base` is still the pinned `rev`. §7.4.9: `hayro-jpeg2000` 0.4.0
on crates.io is the pinned version, and the texts' availability note is dated 2026-09-29.
§7.6/§7.6.6: `Adobe.PubSec` is in 9 files, and all are published test keys or a device key, so
there is no trigger. Four `PDFBOX-4421-*` in `corpus-cache` are now named in §7.6. §12.1, §12.8,
§12.8.3, §12.8.3.4, §12.8.3.4.4: one requirement, a policy's own syntax, per file. Every §12.8.2 row
is already `implemented`, so nothing was small to build. §12.8.3.4's stale "DER check is what is
left" is corrected. §12.10/§12.10.2: Q171 is open and nothing was built.

**Gates.** See the report; the JPX and CCITT failures were a stale `pdf-sandbox-worker`, and
rebuilding it took them to 0.
