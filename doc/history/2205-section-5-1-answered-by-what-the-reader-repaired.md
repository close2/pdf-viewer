# Round 2205 — section 5.1 answered by what the reader repaired

The silent-repair campaign's first fix round, finished from round 2201's handover, in the `gwg`
worktree.

## What changed

- `pdf_syntax::Site::Content(Option<ObjectId>)`: the PDF/A survey records what lexing a page's or a
  form's content needed there, so a repair in content is told from the same class in a dictionary
  (ADR 3521). `pdf_archive` places it at the object, as before.
- `conformance/adheres-to-the-base-standard` leaves `WRITER_EMITS` for a per-document `REMEDIES` row
  (`pdf_transform::archive::base_standard`): a repair the rewrite leaves behind loses nothing; a
  repair in content is refused by name under the fence, class and clause in the sentence, unless
  its page is photographed or the run rewrite writes the content again; a `/DecodeParms` that does
  not pair is answered only where a restatement re-encodes the stream.
- `damaged-data-as-decoded` (ADR 3522): a stream a stage decoded only in part is written as the bytes
  decoded, `reencoded_through_flate` under `Cut::DamageAdmitted`; the loss, its rewrite, the three
  profiles, the report's sentence and JSON, `xmpMM:History`, `doc/pdf-a-mitigations.md`. A cut
  font program stays the font routes' (ADR 3256).
- Undecodable data is left to the output's check, because another rewrite may drop the stream
  (`issue19484_1`/`_2` conform that way); an output failing 5.1 now names its log's classes.
- `475121.pdf`: an integer past 64 bits in page 5's content, logged at the page, re-read in the output;
  now refused by name (`syntax/integer-past-i64`).

## Trap

A repair in content is not one class of answer: the run rewrite writes `number-salvaged` and a lone
`delimiter-unmatched` out, and a photograph takes the whole page. The first census refused 18
documents at 4 that had converted; settling content repairs after those rewrites removed every one.

**Census.** Against round 2201's, with the pack, documents converted: as-if-printed pdf.js 2b
938 to 940, 4 953 to 954, corpora 2b 439 to 441, 4 470 to 473; as-shown pdf.js 910 and 924 (from
906, 920), corpora 414 and 434 (from 410, 429). Without the pack, run one source step earlier, every
profile the same or better but pdf.js 4 (`issue19484_1`/`_2`, which the final source converts).
veraPDF 1.30.3 passes 5438 of 5504 pack outputs; the 46 it fails are round 2201's, rule for rule.

**Gates.** fmt 0; clippy `-D warnings --all-targets` 0; `pdf-sandbox --bins` 0; nextest of the four
crates 100, 2193 of 2205 passing: the manifest's nine, and viewer-qt's three `played_here` tests,
which pass (0) once `pdf-media-worker` is built; rustdoc 0 on each; fuzz `cargo check --bins` 0;
`cargo test -p conformance` 0, 478 passed, once the bounded self-test ran without the census's lock.
