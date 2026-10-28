# 1854 — the signature loss at every target, and a TrueType code on .notdef given a glyph of its own

Session 1854, Slot 4 of batch 126, an archive round of three sessions; ADRs 2542 and 2543 used, Q720 unused. I worked
in an export of HEAD and landed it in the shared tree by three-way merge.

**Built.** (0) `Configuration::authorisations` authorises `signature-assertion` at every target where a profile
discards `file-structure/document-signature-states-no-digest`, the loss being one loss at both parts; a target-qualified
`stop` keeps a signature at part 4 (ADR 2542). (1) R03a for a simple TrueType program (ADR 2543): `fonts/notdef_glyph.rs`
appends a copy of glyph 0 per distinct width among the codes on it, the restated `cmap` maps each code to its copy,
and a reload proves route, outline and width; a direct descriptor is read and written out as an object (§9.6.3, Table
109). Bounds checked before the work, each with a test: 256 copies, 1 MiB of copied outline, an `hdmx` holding the
records it states, one bounded pass of `post` names. `THE_GLYPH_IS_NOT_IN_THE_PROGRAM` now names the programs it is for.

**Found, fixed, tested.** A copy under a long `loca` was padded and failed its proof; `post` gave only 64 names.

**Premise.** "One glyph appended per program" held only where the codes on `.notdef` share a width (section 6.2.11.5);
two widths get two glyphs. Of R03a's 31 documents, 10 are simple TrueType; 6 convert at 2b and 5 at part 4.

**Probes against HEAD's binaries.** The 5 signed part-4 documents: HEAD refused 5 of 5 at part 4, this tree converts 5
of 5, each passing veraPDF at 4, and the 3 that convert at 2b pass at 2b. The 10 TrueType documents: 11 freed outputs,
11 veraPDF passes, pages written equal to pages stated in every one (1 signed source gained 1 appended page under its
packet `preserve`). Plant: a source among them fails veraPDF section 6.2.11.4.1 test 2; two copies given one width are
refused by the readback; an advance one unit off in a 1 000-unit em passes, which is section 6.2.11.5's tolerance.
Host high-water on the largest TrueType source: 17 508 KiB against HEAD's 15 952 KiB.

**Census against HEAD's.** Without the pack, `as-if-printed` converted: pdf.js 2b 875 → 878, part 4 881 → 889;
corpora 2b 398 → 401, part 4 401 → 405; `as-shown` pdf.js 816 → 819 and 818 → 825, corpora 336 → 336 and 333 → 334.
With it, pdf.js `as-if-printed` 876 → 879 and 882 → 890, the rest as without. `rank.py`, both: R03a 31 → 25
documents; R16 at part 4 13 → 6 (`as-if-printed`) and 23 → 18 (`as-shown`); `signature-assertion` 7 → 0 documents.

**Gates.** Export: clippy `-p pdf-transform --all-targets -D warnings` exit 0; `cargo test -p pdf-transform --lib`
exit 0 (151 passed), `--test archive` exit 0 (450), `--test profiles` exit 0 (8), `--test archive_corpus -- --ignored`
exit 0 (7 passed); `-p pdf-archive --test cross_check -- --ignored` exit 0 (1 passed). Shared tree: `cargo check -p
pdf-transform --tests` exit 0; `rustfmt --check` on my files 0 diffs; clippy and tests fail there in siblings' edits;
`tools/state.sh --round 1854 archive` exit 0 (10 377 s); `--bin cited` and `--bin quotations` exit 0; `cargo test -p
conformance --no-fail-fast` exit 101 and `--test records --test evidence` exit 101, every red a sibling's (1853's and
1858's Gates paragraphs, a `pdf-model` quotation). Census exit 0 both (523 s, 236 s).

**Left to batch 127.** R03a for `CIDFontType2` (a `CIDToGIDMap` restated), CFF and Type 1 programs (a charstring
writer); the two TrueType documents that leave the restatement with no code on glyph 0; R10, R06a, R12, R07.
