# 1851 — A site answers in order, and a page states what the conversion removed

Batch 126, slot 1 (archive), six sessions. RFC 0014 built, part one, on the owner's A706; then A707.

**Built.** Sessions 1 and 2: one outcome per embedded file, a text attachment laid out as pages
(ADR 2536), and its annotation made a §12.5.6.5 link (ADR 2537). Session 3, A706 items 1 and 2:
- `try = [...]` (`archive/config/steps.rs`): each step resolved against the target at read time, a
  refused step passed over and listed (`Configuration::skipped`), an empty list an error; `remedy` with
  `on-failure` is the same list. Two steps per object (a keeping step, then `stop`, `discard` or
  `append-statement`) at the embedded-file rows; any other shape refused by name.
- `append-statement` (`archive/statement.rs`): `discard`'s loss, then one appended page listing each
  removed file (requirement, clause, loss, object, name, length, media type, SHA-256, steps tried and
  not admitted), set by ADR 1014's composer; bounds 32 objects, 64 KiB, 16 pages, refused by name; the
  report and `xmpMM:History` name the page. `CLAUDE.md`'s first amendment carries A706's sentence.
- Session 4, R15: a PDF, PNG or JPEG attachment drawn as pages (`archive/drawn.rs`, ADR 2536), a PDF
  by round 1852's `derived` path one level deep, an image one page. Session 5: a drawn file whose page
  cannot be composed takes `on-failure = "discard"` instead of refusing the file; `keep-media =
  "attach"` keeps a removed Screen or Movie annotation's embedded clip at 4f and 4e (ADR 2536).
- Session 6, A707: under `as-if-printed` at 2b and part 4 a tagged document's photographed page leaves
  the structure tree's leaves that name it (`archive/fonts/untagged.rs`, ADR 2516 item 7), the rest
  kept, under `tagging-of-photographed-pages`, counted per page in report, JSON and `xmpMM:History`;
  Level A keeps the refusal. **Owed**: a third step (RFC 0014 section 5); optional content (Q717).

**Evidence.** veraPDF, each output compliant and each plant failing: the stated profile's outputs
(`fileAttachment_fileAttachmentAnnotation` at 2b and 4, `issue17056` at 2b, `annotation-fileattachment`
at 2b and 4, a hostile 32-file document at 2b; the plant the source at 2b); session 4's drawn outputs at
2b and 4 (plant: intent renamed, 3 rules); session 5's kept clip at 4f and 4e (plant: tree key renamed, 1
rule). `issue18030` is refused by name at 2b (its 47.8 by 70.4 pt page puts the statement past 16
pages). 32 files of 16 MiB at 2b: HEAD's `discard` 61 516 KiB in 4.1 s, this tree's 62 368 KiB in 3.4 s,
`append-statement` 79 552 KiB in 6.2 s; the digest plant fails 2 of 4 statement tests. Session 6: the
tagged fixture passes veraPDF at 2b and 4; of round 1841's seven held documents 2 convert at 2b and 4
(veraPDF passes all 4), 3 are held by optional content, 1 is not drawn whole, 1 by a font row.

**Gates.** `cargo test -p pdf-transform --lib --test archive --test profiles` exit 0 each session
(session 3: 167, 468, 11 passed; 4: 170, 474, 12; 5 and 6: 170, 477 and 479, 12; 1 and 6 ignored);
clippy `-p pdf-transform --no-deps --all-targets -D warnings` exit 0 (sessions 4 to 6); `--bin cited` and
`--bin quotations` exit 0; `cargo test -p conformance --no-fail-fast` exit 101, one red each time, another
round's record (1858, then 1853); `tools/state.sh` exit 0 after 8 448 s (session 5's tree).
Exports: `archive_corpus --ignored` 7 passed, `cross_check --ignored` 1 passed, exit 0, sessions 2 and 4
(218 s, 40 s, peak 2.42 GiB). Durations: sessions 1-2 7 980 s, 3 4 300 s, 5 2 175 s, 6 2 400 s.
