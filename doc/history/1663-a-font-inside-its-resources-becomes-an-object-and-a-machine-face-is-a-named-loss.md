# 1663 — A font inside its resources becomes an object, a machine's face is a named loss, and veraPDF's two font rules are read

Slot 1 of batch one hundred and three, 2026-10-21, an archive round. ADRs 2162, 2163; no question.
**Built.** (1) ADR 2162 §1: a font written directly inside its resource dictionary is written out
as an object holding what it gains (`fonts::gains`). The rewriter puts the reference wherever an
equal dictionary stood (§7.3.10's equivalence), and equal fonts share the object. (2) ADR 2162 §2:
a font this machine draws from its own face (ADR 0270) takes the shipped face where that face has
every shown glyph. It is held to the shipped face's glyph code by code, and it is the loss
`machine-face`: `as-if-printed` answers `discard`, `as-shown` keeps `stop` (`doc/todo/72` §3).
(3) ADR 2163: veraPDF reads both rules right, so the premise "one is theirs" did not hold.
6.2.11.2-6 is Table 109's `/Widths` length, which this validator leaves `Unchecked`; 6.2.11.5 is
`wp315`'s CFF `FontMatrix` 0.00049 (318.01 against 317), read by `pdf-font` at a whole-number
units per em. Both owed.

**Real corpora** (converted / conforming / refused / unreadable; HEAD → export of this round only):

| corpus | 2b `as-if-printed` | 4 `as-if-printed` | 2b `as-shown` |
|---|---|---|---|
| pdf.js (974) | 751/0/212/11 → 757/0/206/11 | 697/0/266/11 → 702/0/261/11 | 747/0/216/11 → 752/0/211/11 |
| `doc/corpora` (503) | 266/0/221/16 → 345/0/142/16 | 235/0/252/16 → 309/0/178/16 | 265/0/222/16 → 324/0/163/16 |

Font in its resource dictionary 8, 71 → 0, 0: 64 of 79 convert, and 15 stop on their own
refusals. Machine face 5, 36 → 0, 0 at 2b `as-if-printed`: 21 of 41 convert, two show a code the
shipped face lacks, and `316267.pdf` is refused by its output's second verdict,
`fonts/widths-agree-with-the-program`. Hypothesis "seventy more of `doc/corpora`": 59 by the
direct route. Drawn differently 0, 9 → 0, 10. **Witnesses** (100 dpi, page 1): `T02-01_006…`
and `176446.pdf` byte-identical, `008677.pdf` 1310 px at 1 level of 255.

**Gates.** In the `mine2` export (habit 138; a sibling's `render-cpu` fails clippy in the worktree):
`cargo fmt --check` and `-D warnings` clippy `--all-targets` on pdf-transform and pdf-archive exit 0;
nextest both 902 passed, 0 failed. `tools/state.sh --round 1663 archive`: exit 0, `--remedy-sites`
2b 23 of 69 and 4 17 of 57 (the new conditional site has its `discard`), validator missed 0 and
over 0, `archive_corpus` 2b 525/86 and 4 266/93, `cross_check` 0 disagreements. `t3-foreign_corpus`
`--clock --tree 12`: exit 0, 160 s. Census: three holds, exit 0. veraPDF 1.30.3 at 2b over six
converted witnesses: 6 pass. Worktree `cargo test -p conformance`: all ok but `records`, which
fails on a sibling's record 1667.
**Found.** The unrestated route embeds the shipped face over a machine's face unasked
(`doc/todo/72`). `pdf_font::metrics::stated_widths` reads widths past `/LastChar`.
**Unfinished.** The 6.2.11.2 row and its remedy; the ten drawn differently, undiagnosed. 13 800 s.
