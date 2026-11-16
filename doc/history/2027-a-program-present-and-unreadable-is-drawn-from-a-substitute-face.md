# 2027 — A program present and unreadable is drawn from a substitute face

Session 2027, a font and PDF/A round in the audit worktree. Rows 9.5, 9.7.5.2, 9.8.1 and 9.9; owner answer A837;
ADRs 2870 and 2871.

**The slice.** A837 whole: the window draws a font whose embedded program it cannot read from a substitute face,
and the archive writes that face instead of emptying it.

**The premise.** Held: `pdf-model` drew every `FontError::Malformed` from metrics alone (ADR 1094) and
`cut_program` emptied every cut or unparsed program (ADR 2830); only effort stood in the way. Q837 and ADR 2830 cite
the first sentence they quote as §9.1's and quote the second as "enable"; they are §9.5's and §9.8.1's "enables".

**Built.** `LoadedFont::substitute_for_unreadable` reads a `Malformed` font through `without_program`'s copy of it
(the descriptor without §9.9's three keys) and loads it as a font naming none. One refusal is left out by name: a
program that parsed and whose encoding reaches no glyph. `load_font` reports the refusal with "drawn from a substitute
face instead" and keeps the font out of the cross-page cache. A composite font with no route by character (§9.7.4.2)
still draws nothing, and its report is the program's own refusal. §9.7.5.2's sentence is untouched. In the archive,
`CutProgram::drawn_as_nothing` asks the window's question. ADR 2636's route is written whole unless that answer is
nothing, `machine_cid::embed` reads the window's font, and `unreadable-program` is narrowed rather than retired.

**Tests.** `unreadable_program_substitute.rs` (six; the witnesses read text on every substituted page),
`refused_font_metrics.rs` (two Table 120 arms read `metrics_only` directly), `cut_program.rs` (nine).
**Plant**: `substitute_for_unreadable` answering `None` fails nine (five window, four archive).

**Witnesses.** In the window, all four draw their text. Archive: `issue11651` converts under `as-if-printed` and now
also under `as-shown`, both parts, in the machine's face (0 px from the window). `178360` and `435321`
(`as-if-printed`, both parts) and `268346` (part 4, 0 px) draw their text in the shipped face under the face losses.
Their other conversions stop on HEAD's reasons (ISO 19005 sections 6.2.2 and 6.1.13). veraPDF 1.30.3 passes all nine
outputs.

**Gates.** Over `pdf-font`, `pdf-model`, `pdf-transform` and `pdf-archive`: fmt 0; clippy `-D warnings --all-targets`
0; nextest 4580 of 4580 passed; `cd fuzz && cargo check --bins` 0; conformance 0 after `git add -N`. Tier 2: `corpus`
101 as HEAD's (the same pair joined, 61 incomplete; three now report a substitute); `raster_golden` 101, moving
`issue11651`, `bug1050040` (raster) and `issue13316_reduced` (reports) besides HEAD's 23. All three were looked at,
are this change, and were re-taken; after that the 23 are HEAD's by name. `pdf-archive` corpus 0; `pdf-transform`
`archive_corpus` 0; `state.sh docs` 0.

**Left.** Rows stay `implemented`. R09's census counts predate ADR 2871. A substitute that reaches no glyph for a
code is not reported per code (`issue13316_reduced` stays blank); that is ADR 0007's route, not changed here.
