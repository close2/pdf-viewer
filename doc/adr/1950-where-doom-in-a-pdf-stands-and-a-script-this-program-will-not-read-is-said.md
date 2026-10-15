# 1950 — Where Doom in a PDF stands, and a script this program will not read is said

Session 1557, a script round of batch eighty-eight. Status: **accepted**; section 3 **built**, section
5 is the list a later round builds from (`doc/todo/70`). Context: the owner, 2026-10-15 ("evaluate
where we stand with tmp/doom.pdf … not to get the pdf to work, but only to a) fix obvious bad code /
bad paths … b) find out, what still needs to be implemented"); RFC 0008 sections 4.3, 5 and 6.8; ADRs
1590, 1591, 1602, 1702, 1752, 1904. Code: `crates/pdf-model/src/view/{scripts,script_sites}.rs`.
Test: `crates/pdf-model/tests/script_unread.rs`. Instrument:
`crates/pdf-script-worker/examples/script_cost.rs`. Clauses: §12.6.4.17, Table 221, Table 31,
Table 224, §12.7.4.3. No ledger row moves.

## 1. What the document is

Read with `qpdf --json` rather than with grep, which the brief's counts came from. PDF 1.3, four
objects, one 712 × 620 page. Its `/Annots` is 234 **direct** dictionaries: 200 text fields of 712 × 2
points (`field_0` to `field_199`, the frame's rows), 25 console rows, one text field `key_input`
whose `/K` calls `key_pressed(event.change)`, and eight push buttons whose `/D` and `/U` call
`key_down` and `key_up`. The catalog has **no `/AcroForm`**, no widget has a `/DA` or an `/AP`, and
the page's `/AA /O` is one ECMAScript action of 6 415 756 bytes: a 3.6 kB head, a 5 594 731-character
base64 literal (the shareware WAD), and Emscripten's asm.js runtime and module (817 kB), which sets
`app.setInterval("_doomjs_tick()", 0)` once Doom's start-up ends. The brief's raw counts hold for
`/Widget` (234), `/Btn` (8), `/JavaScript` (18) and `app.setInterval` (2). They do not hold for
`/AA`: 7 096 of its 7 326 matches are inside the base64 literal, and the document has ten `/AA`
dictionaries. Nor for `/Tx`: 51 are in the literal, 2 in the code, and 226 are fields.

## 2. What happens today, in `quorra --scripts on` under Xvfb with the trace

The page's text and the buttons' `/MK /BG` squares draw. Every widget's text is refused once with
§12.7.4.3's reason ("requires a /DA holding a /Tf, and this one holds none"), which is right: `/DA` is
required, and with no `/AcroForm` there is no form-level `/DA` to inherit. A click on a button and
typing into `key_input` raise nothing (the accessibility line counts 0 widgets). The interactive
model keys every field, event and edit by object, and reads fields from `/AcroForm`; this file
gives neither. Table 31 says `/Annots` "shall contain indirect references to all annotations", and
Table 224 makes `/Fields` Required. The page's `/O` script was **not run, and nothing said so**.
`script_text` answered `None` past `MAX_SCRIPT_BYTES` (1 MiB), the chain walk counted it with the
non-script actions, and the page site dropped the count. The host drops the action path's
refusal of a page's scripts at every level but `off` (ADR 1752), so no sentence was left.

## 3. Built: an ECMAScript action whose text is not run is said, with why

`view::scripts::read_script` answers the text, *not a script*, or *not run* with a phrase: the
length against the bound, or that `/JS` is neither of Table 221's two forms. The chain walk keeps
those phrases apart from the actions of other types. Every chain site says each one once where a
runner is supplied, before it runs the rest of the chain: the name tree, `/OpenAction`, a page's
`/O` and `/C`, Table 197's events, an outline item, and Table 200's. Table 200's sentence about
actions "that are not ECMAScript" now counts only those. On `doom.pdf` the open now says "page 1's
/O chain holds an ECMAScript action of 6415756 bytes, past the 1048576 a script may be in this
program, and it was not run". **Left as it is:** at Table 197's events and an outline item, a host
keeps the action path's Tier 0 sentence where nothing was handed over
(`viewer_core::interact::handed_over`), so a long annotation script is said twice, once with the
wrong reason. Narrowing that needs
`ScriptsRan` to carry what was said, and `viewer-core` is not this round's.

## 4. Where the time goes — measured on repaired copies, each change named

To see past section 2, three copies were written with `qpdf --json-input`. **Repaired**: the 234
dictionaries made indirect, an `/AcroForm` listing them with `/DA (/Cour 0 Tf 0 g)` and a Courier
`/DR`. **Chained**: the `/O` split into a `/Next` chain of pieces under the bound (the literal pushed in
900 kB parts, then the program with the literal replaced by their join). **Staged**: the program's
`try` removed, its `typeof print` test replaced by `false`, `Module.print` also logging, and the program
cut into three runs. **Clocked**: the staged copy with Emscripten's one `Date.now()` answering 29 ms
more at each read. The in-process engine ran under a budget no script reaches (`script_cost
--lifted`). The figures are the round's one clock hold at load 3.7, except the two runs of 600 s and
3 600 s, which no hold could carry. `doc/history/1557` has them all:

- **Before any of Doom runs.** At `Budget::FIELD_EVENT` the chained copy is stopped by the 100 ms
  wall, 514 ms into its last run, because the wall is read only between the engine's slices.
  Lifted, the program throws at `typeof print`, because a refused member throws when it is *read*
  (ADR 1591), and Emscripten tests for a shell's `print`. The reason is Tier 2's and is right; the
  place is a name, where RFC 0008 section 4.3 describes how an excluded *call* fails.
- **The head's base64 decode** takes 35.9 s, and the process's resident high water is then 128.5 MiB
  (outside the hold). The script worker's address-space ceiling is 96 MiB. **The runtime and asm.js
  module** take 0.93 s.
- **Doom's start-up does not end while the clock stands still.** On the staged copy `run()` was
  stopped at 600 s, and again at 3 600 s, with its log at `ST_Init` both times. A realm's `Date`
  answers one moment for the whole run (`pdf_script::Request::moment`), so that a script cannot time
  the host (RFC 0008 section 4.2), and Doom waits for its first tic by reading the clock. On
  the clocked copy start-up ends in 0.87 s, with 926 edits (the first frame and the console).
  **Each `_doomjs_tick()` then takes 182–201 ms** (mean 191 ms over ten) in the engine alone.
  Thirty-five tics a second allow 28.6 ms. The resident high water is 240 MiB.
- **The framebuffer pattern without Doom** (`framebuffer.pdf`: a 0 ms interval rewriting all 200
  rows of 320 characters): the view state's timer run, the engine's 200 edits included, is a mean of
  6.2–6.5 ms, and **the page interpreted again is a mean of 98.8–105.0 ms** (70.5–134.0 ms).
  Callgrind puts 42 % of that page's instructions in `pdf_font::LoadedFont::load` from
  `variable_text::set_in`: the `/DA`'s font is loaded once per field per interpretation. In
  `quorra` a tick holds the window thread for a median of 70–74 ms, and **at a 0 ms interval no
  frame is ever presented**: 258 ticks in 20 s, and 0 of the changed page's 257 frames, each
  "nothing to show". At 250 ms, 79 ticks and 97 frames presented.

## 5. What a round would build, in order, each priced

1. **Built here**: section 3.
2. **Variable text's font loaded once per interpretation, not per field.** S–M. Every form pays it:
   one load is about 1.7 million instructions (668 loads, 1.12 billion in the profile), so a page of
   300 text fields pays about 500 million per interpretation. Either a cache passed from the
   interpreter to `set_in`, or the standard-14 substitute's tables held once per process in
   `pdf-font`. No pixel may move: `raster_golden` and the six arms are the proof.
3. **A timer that cannot starve the window.** S, `viewer-ui` and `viewer-core`. A due timer waits
   until the frame its last run asked for has presented. ADR 1702 already runs a late timer once
   rather than once per period missed, which allows that. Driven at 0 ms and at 250 ms.
4. **A widget no `/AcroForm` reaches, and an annotation listed as a direct dictionary, said once by
   name** under Table 224's and Table 31's sentences. S. Today both are silent (trap 5), and trap 11
   says to derive the condition from the clause.
5. **A refused method refused at its call, not at its name.** M. `crate::surface` gains a member's
   kind, a method answers a function that throws `NotAllowedError` when called, and a property keeps
   throwing on read. Both script-corpus gates are held again, because the throws they count move.
6. **Running Doom at all**, not recommended. Each part is a decision more than a build:
   (a) direct annotations as interactive objects, L, because the whole model keys by object;
   (b) fields built from page widgets that no `/AcroForm` names, a repair no clause defines;
   (c) the bounds: 1 MiB a script in the view state and on the wire, 3 MiB a run, 96 MiB the worker,
   100 ms a run, 1 s an open sequence, a 250 ms deadline. A tick alone exceeds the first, and the
   decode exceeds them all. (d) A clock that moves inside a run. It is RFC 0008's rule against
   timing the host, and nothing a round may relax unasked. (e) A widget with no `/DA`: §12.7.4.3
   gives it no appearance to construct, so the frame's rows are never drawn without a repair.
   (f) The engine. With (a)–(e) granted, Boa runs a tick in about 190 ms against 28.6 ms, and the
   page's 200 rows re-interpreted add about 100 ms. That is three frames a second at best, a slide
   show rather than a game. RFC 0008 section 5.1's loop row has QuickJS about ten times
   faster. Every engine faster than that is a JIT in C++, refused there on principle 3. **Doom does
   not become playable here.** Items 2–5 are worth building for every document, and 6 is the honest
   answer to "what still needs to be implemented". A `Q` file is owed only if a round proposes to
   start any part of 6.
