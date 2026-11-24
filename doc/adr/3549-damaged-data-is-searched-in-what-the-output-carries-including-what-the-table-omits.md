# 3549 — Damaged stream data is searched in what the output carries, the objects the table omits included

Status: **accepted**, implemented (`archive::base_standard::carried_and_unlisted`, `restated`).
Session 2212. Builds on ADR 3521 (section 5.1 answered by what the reader repaired) and ADR 3522
(`damaged-data-as-decoded`). Context: ISO 32000-2 §7.4, §7.5.4, §7.3.10; ISO 19005-2 sections
5.1 and 6.1.4 (paraphrased).

## Context

govdocs1 `027613.pdf` was refused under `as-if-printed` at 2b with ADR 3521's "no stream object of
the file decodes that way when asked again". Located: its cross-reference table names 260 objects of
a 429-page file. `pdf_syntax::Document::get` loads an object the table omits by its own `N G obj`
header where a section declares it and holds no entry for it, so the pages' content streams (objects 93, 169, … — eleven streams whose zlib check value
disagrees, two cut part way) are drawn; `pdf_archive::check` decodes them through
`hexadecimal_spans`, which follows each page's `/Contents`, and logs `syntax/stream-data-damaged`.
`restated` searched `Examination::objects()` — the table's population, which section 6.1.4's
exemption of objects no cross-reference section names makes the population the *source's* rows
bind — and so found nothing. The whole-file rewrite (`rewrite::walk`) copies everything the catalog
and `/Info` reach, so those streams are in the output under a table that names them, and the output
fails section 5.1 at them.

## Decision

The search for damaged streams is over what the output carries: the table's objects the reach walk
reaches, and the objects a walk from the trailer's `/Root` and `/Info` reaches that no section
names. Each found is restated as ADR 3522 says, and a `/DecodeParms` mismatch found there is refused
as anywhere else. The examination's population is not changed: section 6.1.4 binds it, and every
other row stays the table's.

## Consequences

The answer for a file whose table is incomplete now matches the file the rewrite writes. A row that
edits objects by the table's population (`sites.rs`'s walks) still does not see an unlisted object;
a row the output's own check then fails at such an object names it there.
