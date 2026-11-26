# 2261 — A stale linearization dictionary states no page count

Lane 3, the fix round of audit 2260 (Annexes D, E, F, I). ADRs 3745, 3747, 3748; Q1082. Two of
the three `*_spec_audit` tests pass unchanged and leave `expected-failures-current.json`; 2260's
B02, B03, D01–D04 are `fixed`, B01 is `open` on Q1082.

- **B02** (ADR 3745). `pdf_syntax::linearize::stated_page_count` reads Table F.1's `/N` only from
  the file's first object inside §F.3.3's 1024 bytes and only where `/L` is the file's length;
  `pdf_archive::pages_held::linearized_page_count` wraps it, and `NoPageHeld::given` and
  `recovered::refuse_a_count_the_reader_fell_short_of` read it there; unlike `state` it needs no
  `/H`, `/O`, `/E`, `/T`. The fixture in `tests/archive/recovered.rs` states a true `/L`.
- **B03** (ADR 3747). `merge::text_string` (merge's and split's labels), the output intent's
  `/Info` and — found by the writers' census — `archive/attach.rs`'s attachment name keys (and so
  `/F`, `/UF`) go through `encode_text_string`. Nothing else in eight crates' writers formats a
  text string by hand.
- **B01, D01, D02** (Q1082). The census (`examples/standard_encoding_census.rs`, 117 833 documents):
  4 name `StandardEncoding`, no page draws differently. Reading the name as absent was built, and
  moved TrueType fonts in the archive: `font-maps-read-as-written`'s removal fails its proof on a
  nonsymbolic TrueType font, so two archive tests that convert mechanically today would need a
  loss. Reverted, the departure stated in `PERMITTED_ENCODING_NAMES` and the D.1 row; the owner
  answered Q1082 with option 1 (A1082), owed to the next round.
- **D03**: the E.2 row names the class of writer-chosen names. **D04** (ADR 3748): a `%PDF-1.8`
  or `1.9` source written as 1.7 at PDF/A-2 has its header recorded in `xmpMM:History`, with a
  unit test. Ledger notes of F, F.3.3, I.2, 7.9.2.2, E.2, D.1 restated; statuses unchanged.

**Gates.** Over `-p pdf-syntax -p pdf-font -p pdf-model -p pdf-archive -p pdf-transform -p
viewer-qt -p conformance`: fmt 0, clippy `-D warnings --all-targets` 0, `pdf-sandbox --bins` 0,
nextest `--lib --bins --tests` (`xvfb-run -a`) 6297 of 6301: the manifest's two §10.5 entries,
B01's, and `records` over this file's placeholder before it was written (`audit-diff.py`: no
other difference); rustdoc of `pdf-syntax`, `pdf-font`, `pdf-archive`, `pdf-transform` 0; `fuzz`
check 0 (`PERMITTED_ENCODING_NAMES` unchanged in the end, two public functions added);
`cargo test -p conformance` 484 passed, 1 failed (the same `records` test), then 0 alone.

**Census.** With the pack, against 2258's: one document moves its reason and not its answer —
`REDHAT-1531897-0` is still refused on each profile line, now for its `/Root` rather than for a
linearization `/N` the file's length no longer backs; the run without the pack queued two hours
and was stopped. veraPDF 1.30.3: 5471 of 5523 compliant, 40 not, 12 unfinished, as 2258's.

**Trap or habit.** The archive maps a reader's repair to a PDF/A row, so a reading change the
viewer census calls free can still move a conversion: run the archive tests before calling it free.
