# 1175 — Two refusals answered, and two clauses read against the catalogue

`doc/todo/66`'s three top refusals at PDF/A-2b, and the encryption family beside them.

## What was found

- **`file-structure/no-encryption` was the smallest refusal in the catalogue.** §7.6.2 makes
  encryption a property of the file, so the conversion's objects were already decrypted and its
  trailer held no `/Encrypt`: the act was being performed and nobody had named it. Missing were the
  *decision* — section 3.5's Ask — and the permission statement.
- **`graphics/separations-of-one-name-agree` could not take the catalogue's answer**: its explicit
  `colourants = { name = definition }` table needs a §7.10 function, which no configuration holds.
- **`fonts/cid-system-info-agrees-with-the-cmap`'s *none* holds, and one sentence of it was too
  strong.** §9.7.4.2 makes that entry a copy of the program's — "which should be copied into the PDF
  CIDFont dictionary" — so the file often does say which side is wrong, and the remedy is *correct a
  dictionary from the program it describes*. Raising a Supplement asserts glyphs the font may lack.
- **`file-structure/no-external-stream-data` is blocked on where the fetch lives**: a `supply`
  carrying a directory would put an `open` inside `apply` and cost RFC 0002 section 9's determinism
  claim. The catalogue holds the two shapes that keep it, and one thing to ask first — ISO 19005
  spells a key `FDecodeParams`, which names nothing in ISO 32000.

## What was built

- `crates/pdf-transform/src/archive/protection.rs`, new: what the source's `/Encrypt` asserted,
  carried into the report and the output's `xmpMM:History`, under `Loss::Encryption` and
  `Rewrite::EncryptionRemoved` — the `Crypt` filter row being the same act. ADR 1187.
- The separation site as a `supply`: `winner = "first" | "most-used"` picks among the document's own
  definitions, and `pdf_archive::same_parameter` exports the sameness the requirement is failed on,
  so the grouping cannot differ from the judgement. ADR 1188.
- A census of its own in `archive_corpus` (a configured remedy is invisible to a sweep that supplies
  nothing), and four fixtures in `archive.rs`.

## Measured

`archive_corpus` under the heavy-walk lock, before and after, as printed. **PDF/A-2b** 470 → **471**
converted, 140 → **139** refused; **PDF/A-4** 206 → **207**, 153 → **152**; the default run unchanged
at both, which is what an *Ask* means. The separation census, per target and per word: **4 documents
converted, 4 colourants decided, 0 still refused**. Over the eight witnesses the conversion moves
pixels — worst tile 39.62, max channel 242 on `6-2-4-4-t03-fail-a` — and the two words agree on every
one, each stating each definition once so `most-used` ties. ADR 1188 section 3 has the instrument.
