# 2146 — Page one drawn before its structure is asked

Session 2146, a measure round in the `lane3` worktree, on ADR 3262's open item: EC3's first frame carried
§14.9.4's walk to the structure root and its `peak_anon_mib` sat outside its band. ADR 3285; no ledger row moves
(14.9.2 to 14.9.5 keep `implemented`: when their statements are read changes, not what they say); no question.

**What page one needs.** §14.9.4 makes `/ActualText` "a character substitution for the content", and §14.9.2,
§14.9.3 and §14.9.5 say what text is spoken as: what a page reads back as, never a mark. In the EC3 profile the
marked-content operators spent 46.2 M of the 169.9 M first frame fetching elements and their ancestors
(`Document::get_key`, 657 calls), so page one is drawn without them and its readback asks them when first wanted.

**The change.** `ViewState::set_structure_deferred` (a two-state `StructureReading`, clippy's bool count) makes
`Interpreter::accessibility` read only the sequence's own property list and record, where a missing entry's
`/MCID` names a parent-tree entry, `Interpretation::structure_deferred` (carried by divided pieces and the
checkpoint). `viewer-core` reads a page it is about to draw that way (`interpret_to_draw` from
`interpret_arranged`, and `first_read`); `Interpreted` keeps its readback in a `Readback` with a `OnceLock`, and
every reader of text, placements and spans goes through `OnScreen::readback`, which interprets the page once more
under the open view where the first reading deferred. A deferred reading keeps no replacement and fills no search
cache entry; a search takes an on-screen page's readback from its reading. The first probe deferred only
`first_read` and moved nothing (169 903.7): launch reads page one through `interpret_arranged`.

**Tests.** `a_reading_that_defers_the_structure_draws_the_same_page_and_says_so` (pdf-model). The headless
`structure_actualtext_*` tests now read the settled readback; `an_edit_forgets_every_page_the_search_had_read`
expects 0 pages after the edit, not 1, its tagged page showing waiting for its readback (ADR 3285).

**Figures** (thousands, this program's object, release under 1.100.0; `measure.sh` children):
EC3 first frame 169 903.7 → 128 057.8, AN001 11 102.1 → 10 864.3, WTPDF 41 395.3 → 41 215.3; other frames
within 0.8 %. EC3's band moves (ADR 3285); AN001's returns to the band ADR 3262 moved it from.

**Gates.** fmt 0; clippy `-D warnings --all-targets` over `-p pdf-model -p viewer-core` 0 (after the bool became
an enum); `pdf-sandbox --bins` 0; nextest `--lib --bins --tests` of the same set under `xvfb-run -a` 0, 3547 run,
3547 passed, against an empty manifest; rustdoc `-D warnings --document-private-items` 0; fuzz `check --bins` 0;
conformance 0 (478 passed); `launch_path` 101: 36 banded, 3 outside, all opens (below). EC3's frame 128 062.8, its
`peak_anon_mib` 17.9 (was 21.457), inside. Selection census unchanged against the train's (0 readback differences
of 968; 0 searches re-reading a page); accessibility census unchanged (248 172 elements reached).

**Left.** HEAD's opens grew after round 2140 measured them (`3c3ed860`'s cross-reference runs, `4752821f`'s filter
checks): EC3 56 427.1 (55 063 .. 56 176), AN001 2 227.1 (2 179 .. 2 224), `xfa_filled_imm1344e` 287.1 (279 .. 286),
none this round's. A host could settle a tagged page's readback on idle after the first present.
