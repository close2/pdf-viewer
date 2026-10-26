# 1782 — A sound's own samples cross as a WAVE file, and rich media assets are saved

Session 1782, a host round in batch 118; 2026-10-26. ADRs 2398, 2399; no Q (Q648 unused).

**Contract.** Slot 4: A631's second and third steps — a RIFF wrapper for a sound holding its own samples
(Table 305's `/R`, `/C`, `/B`, `/E`, an unknown `/CO` said) through the hand-off ADR 2382 built, and
§13.7.2.3.2's `/Assets` listed and saved under the existing level for embedded files, never played.

**Premise.** Held: the wrapper is a pure function (`pdf_model::media::wave`) with a test per format, and
`Event::Media`/`Media` keep their shapes, so no window, wire or C ABI changed. Did not hold, twice: (1)
"walked by `attachment::attachments`" — an annotation sits on a page, and a document-wide list is the
page-tree walk at open ADR 0295 refused; the assets join the page-in-view list as a page `/AF` does (ADR
1186), and a click hands them out. `Command::Extract` read only the log's list, so a page `/AF` the panel
listed could not be extracted by name either; it now reads the panel's. (2) `doc/md/`'s Table 305 drops the
printed `ALaw` row and `Default value: Raw.` (read against the PDF itself); the code follows the print.

**Built.** `media/wave.rs`: `SampleFormat::read` (a `/CO`, an unnamed `/E`, non-8-bit µ/A-law and sizes a
header cannot state refused and said; a fractional `/R` rounded and said) and `wave` — PCM unpacked from the
packed big-endian stream into high-justified little-endian bytes, `Signed` ≤ 8 bits and `Raw` > 8 bits
re-biased, µ-law/A-law unchanged under RFC 2361's tags 7/6 with a `fact` chunk, bounded by
`max_stream_len`. `Data::Samples` carries the undecoded stream; `viewer_core`'s hand-off decodes, wraps and
reports. `attachment/rich_media.rs`: `assets` walks Table 333/341 (the clause's EXAMPLE as a test) and
checks the producer's naming rules, said on a click. The drive gains `89-sound-click` in both lists.

**Rows.** §13.3 `partial` → `implemented` ("If no F entry is present, the sound object itself shall contain
the sample data and all other information needed to define the sound"); §13.7.2.3.2 `out-of-scope` →
`implemented` ("The Assets entry takes a name tree of embedded file specification dictionaries"); §12.5.6.16
and §12.6.4.9 notes and tests re-pointed. `doc/todo/65` bucket 6 emptied; `doc/state-of-play.md` one clause.

**Gates.** In my export (a sibling's `xfa` edit broke the shared build, habit 138): `cargo fmt --check -p
pdf-model -p viewer-core`: exit 0. `RUSTFLAGS="-D warnings" cargo clippy -p pdf-model -p viewer-core
--all-targets`: exit 0. `cargo nextest run -p pdf-model -p viewer-core`: exit 0, 2652 passed; `-p
viewer-host`: exit 0, 302 passed. `RUSTDOCFLAGS="-D warnings" cargo doc -p pdf-model -p viewer-core --no-deps
--document-private-items`: exit 0. `cargo test --profile gates -p pdf-model --test actions -- --ignored
--nocapture`: exit 0, 2 passed. A planted revert of `Command::Extract`'s list fails
`a_pages_rich_media_assets_are_listed_and_extracted_by_name` (trap 13). The drive under Xvfb on my export's
release binaries, `sound_click` and `movie_click` in all four windows: 8 works, 0 wrong; the expected
sentence is the windows' own log line (trap 273). `cargo test -p conformance --no-fail-fast`: exit 101, 464 passed, 2 red, neither mine: the records of 1780, 1781 and 1785.
`t2-launch_path` not run: the one launch-path addition is a `/Subtype` lookup per annotation of the page in
view inside `Query::Attachments`. No arm compared: no file a page's drawing executes changed. `tools/state.sh docs` was withdrawn before its grant, queued behind two clock holds; the crate-scoped rustdoc above stands for it.
