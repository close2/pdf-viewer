# 2110 — A packet no page sets is discarded as if printed, and a media box of no area is written as drawn

Session 2110, an archive round in the `lane3` worktree, on `doc/pdfa-blocking-issues.md`'s R14, R10
and R03 rows. ADRs 3141 and 3142; ledger row 7.7.3.3 (`implemented`, before and after); no question.

**Found.** R14's documents under `as-if-printed`: `241573_ftp` (a packet that is not UTF-8) and
`balloon_a1b_jp2k` (U+0000 in the packet header) at 2b and 4, `JPXTestGrey` at 2b (the
extension-schema container), `poppler-742-0-fuzzed` at 4 (a packet not editable in place). Both
`state-every-loss` conversions already worked by ADRs 2678 and 2682's second step; no other
profile named it.

**Built, ADR 3141.** `as-if-printed`'s three packet rows name `on-failure = "discard"`: the packet
or the control character is named in the report and `xmpMM:History`, with no statement page, since
a print carries no packet. Profile-only; tests in `packet_removed.rs` and `packet_characters.rs`.

**Built, ADR 3142.** A media box with a side of no length is written as the box `pdf_model::Page`
drew the page on (`DEFAULT_MEDIA_BOX` where the ancestry states none), each other box of no area as
§14.11.2.1's default, at factor 1: no `/UserUnit`, no bracket. `boundingBox_invalid` converts at 2b.

**R10, read and not built.** `499039`'s two widths are code 173 at 0 beside the hyphen's 0.333 in two
fonts: `TT3` (embedded) reaches `restate_metrics`, which has no twin route, and `TT0` (not embedded)
reaches a restatement of its dictionary, beside which ADR 2723's renaming is not tried. Each needs
its own build, and `issue11740_reduced` (a Type 1 program under a `CIDFontType0`) needs a format
change; neither fitted beside the census and the gates.

**Census.** Re-taken by section 1's commands. Against 2106's, `as-if-printed` converts 1 more pdf.js
and 2 more corpora documents at 2b, 2 more corpora at 4; `as-shown` 1 more pdf.js at 2b. Refused or
declined at every `as-if-printed` target: 28 to 26. veraPDF 1.30.3 passes the six outputs of the
freed documents; drawn by this reader beside the source, `boundingBox_invalid` and
`balloon_a1b_jp2k` are identical and `241573_ftp` differs only by its output intent's colours.

**Gates.** Over `-p pdf-transform`: fmt 0; clippy `-D warnings --all-targets` 0 (after a type alias
for `type_complexity`); `pdf-sandbox --bins` 0; nextest `--lib --bins --tests` 0, 1262 passed, none
expected; rustdoc 0; conformance 0. No public type changed, so no `fuzz` check.

**Left, in rank order.** R10 (`499039`'s two routes, `issue11740_reduced`), R03's pages not drawn
whole, R08, R03a, then R14's `JPXTestGrey` and `poppler-742-0-fuzzed`.
