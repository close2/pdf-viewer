# 1349 — A page list found by scanning is said, and a published password is answered

Robustness slot of batch fifty-four. ADRs 1533 and 1534. No question written. No ledger row moved.

**The premise.** The brief asked for a page tree rebuilt from `/Type /Page` objects. ADR 0097
built that, and ADRs 0782, 0784 and 0786 refined it. The residue was loudness: a page found by
scanning a whole dictionary said nothing (`issue9418.pdf`).

**Built (ADR 1533).** `Pages::found_by_scanning() -> Option<usize>`. `viewer-core`'s open notes now
say the count, and that the order is the object numbers'. `pdf-model/tests/corpus.rs` holds
`FOUND_BY_SCANNING` by name: `issue19484_1`, `issue19484_2`, `issue9418`, `poppler-395-0-fuzzed` and
`poppler-742-0-fuzzed`, each with its reason. Five fixtures were added to `page_tree_nodes.rs` (a
tree; `/Kids` at nothing; no `/Pages`; `/Root` at nothing with no catalogue; two pages in object
order) and one pair to `notes.rs`.

**Read against the bytes.** `bug1020226`, `REDHAT-1531897-0` and `poppler-937-0-fuzzed` hold no
`/Type /Page`. `poppler-85140-0`'s page is under generation 2^64, past §7.5.4's 65,535, and its
`/Contents` is too. Brotli's filter is not in Table 6. `PDFBOX-4352`'s `E<` leaves no entry whole.
All stay as they are. Of the three `ONE_REFERENCE_REBUILT`, only `issue9418` is a scan, and none moves.

**Found (ADR 1534).** `encrypted-attachment.pdf`'s password is not "published nowhere":
pdf.js's `test/unit/api_spec.js` opens it with `000000`, which matches as its owner password. It
is now the tenth row of `corpus_passwords.rs`. The page draws *Example* at 612x792. That was looked
at, and it matches what mutool draws.

**Lists moved.** `LOCKED` 1 to 0, `NO_RENDER_NEEDS_A_PASSWORD` 1 to 0, raster `NOT_COMPARABLE`
7 to 6, `save_round_trip` `REFUSED_OPEN` 2 to 1, and accessibility `REFUSED_OPEN` 2 to 1. The page
joins `SAVE_REFUSED_ON`, because its xref is rebuilt by scanning. The untagged-honest floors moved
877 to 878 and 890 to 891. The `raster_golden.tsv` row went from locked to drawn.

**Gates.** rustfmt 0. clippy `pdf-model`, `pdf-syntax`, `viewer-core` 0. `render-raster` clippy
fails at `src/scene/own_space.rs:175`, a sibling's `TMPFRAME` line. nextest on the three crates:
2374 passed. `cargo test -p conformance` fails only `records` (sibling record 1346). Behind the
lock: `pdf-model --test corpus` 0 (59 incomplete, 5 pageless, 5 scanned); `raster_golden` 0 after
update; oracle 0 (1967 pages); `render-raster --test corpus` at 1× 0 (968 / 0 / 0 / 6);
`save_round_trip` 0; `accessibility_census` 0; `selection_census` 0; `pdf-vfs` `read_corpus` and
`write_corpus` 0; the five `pdf-transform` corpus walks 0.
