# Round 2290 — a copy nothing else draws takes the original's place

Lane 3. Subject: the leak round 2287 named (ADR 1196): a form shared through inherited or named
resources is carried whole wherever they reach it, drawn or not. ADRs 3861–3863, Q1111.

- **The probe first**: two pages inherit `/Fm1` and `/Im1` from `/Pages` (or name one
  `/Resources`); page one draws them under a region, page two never does. Before: `SECRET` and the
  covered image rows were in the output (three tests failed on that assertion).
- **An undrawn original takes the edit in place** (ADR 3861, `redact/undrawn.rs`): after every page
  is walked, each original a page copied — form, image, a shading or pattern by reference — is
  looked for in every page's content and every `/AP` and `/RO`, through each form a `Do` names.
  One nothing draws takes the first page's edit in its own place; every page that draws it is
  unchanged (pixels of the other page compared before and after in each test). The search errs
  toward *drawn*.
- **The sweep**, each held by a test: a soft-mask group (a form edit; passes, not run before the change), a located shading
  pattern (the same leak, fixed by placing the replacement in the pattern's slot), an inherited
  form holding a form the other page draws, and the keeping cases (the other page draws it, or an
  appearance on it does). One more leak, older than this round (ADR 3863): a form a page *owns*
  holding a form another page draws was rebuilt by `privatise` from the producer's object, its own
  covered text included; replaced objects now enter the page's copy map under their slot.
- Left in the sweep: a shading written directly in inherited resources (no object to replace).
- **A partly reached element** (ADR 3862): §14.9.3/§14.9.4 describe an element's content as one,
  and the element names the copy, so it stays restated whole; splitting it is Q1111.
- No redaction corpus census exists (`tools/state.sh` names none).
- Ledger: 12.5.6.23 stays `departed`; `undrawn.rs` and five tests added to the row.

**Gates.** Over `-p pdf-syntax -p pdf-model -p pdf-transform -p conformance`: fmt 0; clippy 101
(`type_complexity` on `place_located`, `similar_names` in a test), fixed, 0; `pdf-sandbox` 0;
nextest 5498 run, 5497 passed, 1 failed (`questions`: Q1111 needed a slug), renamed; `audit-diff`
0 expected; every redaction test passes in it, `redaction_leaves_no_trace` included; rustdoc of
`pdf-transform` 0; fuzz 0; `cargo test -p conformance` 0 (485 passed). Launch 101 then 0: the first run
found `PDF20_AN001-BPC.pdf`'s `peak_anon_mib` 6.605 over 6.500 at load 36; the launch binary does
not link `pdf-transform`, and the second run read 6 documents, 36 figures banded, 0 outside.

## Habit

A copy proves nothing about the original's readers: before keeping a producer's object, ask who
*draws* it, not who references it — inherited resources name everything to everyone.
