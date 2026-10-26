# 74 — The god classes: what the census finds, in the order to split it

Status: **open** — entries 1 to 14 are split (ADRs 2004, 2018, 2032, 2046, 2064, 2080, 2081, 2096, 2097, 2112, 2128, 2144, 2160, 2176, 2192, 2288, 2304, 2320), and so are the files entry 15 names (ADRs 2216, 2217, 2226, 2227, 2256, 2272); section 17 is the order. Ranking rule: ADR 2005.
Priority: the owner's, 2026-10-19: "Refactor god classes,..." and "Do not fix any bugs.", in a round
asked to improve code quality for "the 'project which we can show students' part".
Instrument: `python3 -I -B tools/god-census.py --top <n>`; `tools/state.sh gods` prints the top five
of each of its four tables. The census counts and this file orders; a number is read from the
command, never copied here.

**A split here is a move.** Behaviour held byte for byte, proved by the multiset of the moved lines
(habit 93); the ledger rows that named the file re-pointed to the module holding the row's item
(habit 96); every `pub` narrowed to what a sibling reads; a module's doc comment says what it is for
and nothing about how it came to be (ADR 1023). A bug met on the way is a line under "Found, not
fixed" below, never a change.

## 1. Built — `Viewer` (`crates/viewer-core/src/viewer.rs`), ADR 2004

One `impl` of the census's largest block, over a hundred methods, now one module per concern under
`viewer/`: `opening`, `layout`, `navigation`, `pointer`, `outcomes`, `editing`, `printing`,
`search`, `geometry`, `accessibility`, `answers`, `media`, `scripts`, `anticipation`; the root keeps
the state, the identities, the bounds and the one routing function.

## 2. Built — `ViewState` (`crates/pdf-model/src/view.rs`, with `view/scripts.rs`), ADR 2018

The second block, and the second type by methods once its files are summed. `view.rs` is now the
root — the state, the types a host names, the constructor, the re-exports and the tests — and ten
modules each hold an `impl ViewState` for one concern: `settings`, `optional_content`, `annotations`,
`markup`, `attachments`, `fields`, `field_tree`, `writing`, `pages`, `actions`. The `view/script_*.rs`
files remain separate concerns. ADR 2202 splits `scripts.rs`'s method block into
`commit`, `events`, `edits`, `calculation`, `queries`, `keystroke` and `state` under
`view/scripts/`; its root keeps the script types, limits and shared helpers. The entry named eight modules; the field-tree walk
(`field_tree`) and the reading of an annotation (`annotations`) are the two it did not, and the
fields did not have to be cut last: they call the others through `pub(super)` and no field was
widened.

## 3. Built — `Walk` (`crates/pdf-transform/src/redact.rs`, with `redact/`), ADR 2032

The third block. `redact.rs` is the root — the state, the entry points and the page's planning and
writing — and seven modules each hold an `impl Walk<'_>` for one concern: `text`, `painting`,
`frames`, `images`, `inline`, `graphics_state` and `regions`, beside the six constructions
(`forms`, `located`, `mesh`, `paths`, `shading`, `type3`) that were already there. The entry named
`text`, `paths`, `images` and `forms`; `paths` and `forms` are constructions the walk calls, so its
side of them is `painting` and `frames`. What is left in the root past the walk is the page's
writing (`write_document`, `build_page`, `Carried`) and the sample arithmetic, free functions and
types that are a different split.

## 4. Built — `Rewriter` (`crates/pdf-transform/src/archive/rewrite.rs`, with `rewrite/`), ADR 2112

One `impl` of 65 methods, now a root and eight modules. `rewrite.rs` keeps `Rewrite`, the state
`Rewriter` holds, the object walk, the routing (`rewrite`, `rewrite_array`, `rewrite_dictionary`,
`owed_rewrites`) and the free helpers; each module holds an `impl Rewriter` for one concern: `values`
(an entry restated in place), `appearance_states`, `configurations`, `page_structure` (resources,
boundaries, content taken out of the page, appended pages), `attachments`, `annotations`, `catalogue`
(the catalogue and the interactive form, permissions, signatures) and `streams` (dictionary, filter
chain, data). About 1 100 lines of the root are the `Rewrite` enum and its descriptions, a different split.

## 5. Built — `Tree` (`crates/pdf-model/src/structure.rs`, with `structure/`), ADR 2046

The largest source file. `structure.rs` is the root — the state, the entry points that read `/K`,
`logical_range` and the types and free functions the concerns share — and five modules each hold an
`impl Tree` for one concern: `roles` (roles and namespaces, one walk), `attributes`, `tables`,
`owners` and `logical`; the tests are `structure/tests.rs`. `logical_range` stays in the root until
the clause slot of batch ninety-five that edits it has merged; moving it into `logical` is then one
cut. About 3 000 lines of types and helpers remain in the root, a different split.

## 6. Built — `LoadedFont` (`crates/pdf-font/src/loading.rs`, with `loading/`), ADR 2064

64 methods in one block, now a root and eight modules. `loading.rs` keeps the state (`LoadedFont`),
the entry points (`standard`, `load`, `metrics_only`), the types and free functions the concerns
share, and the tests; each module holds an `impl LoadedFont` for one concern: `opening` (the simple
and the composite route), `findings` (what loading found out and repaired), `codes` (a code, its
text and the inverse), `program` (what the embedded program states), `advances` (widths, vertical
metrics, pair kerning), `glyphs` (a code to a glyph index), `drawing` (outlines, the cache and the
hinting instance) and `substitution` (a stand-in face and its classes). About 1 200 lines of types and free functions remain in the root beside its four test modules (about 1 200 lines more); moving either is a different split.

## 7. Built — the transparency and pattern interpreters (`content/transparency.rs` and `transparency/`, `content/pattern.rs` and `pattern/`), ADRs 2080 and 2081

Two blocks of the one `Interpreter` type (the type is the largest by methods and is already split by
file), and the answer to "one clause or several" was several. `transparency.rs` keeps the group
entry points (`run_transparency_group`, `transparency_group`, `appearance_group`,
`imported_page_group`), the three types they share and the tests; twelve modules hold the rest by
concern: `transfer`, `commands`, `blending_space`, `into_parent`, `press`, `group_runs`, `soft_mask`,
`readings`, `shape`, `knockout`, `implicit_knockout` and `departures`. `pattern.rs` keeps the paint
types and the entry points `pattern`, `tile`, `fill_paint`, `stroke_paint` and `mark_transfer`; eight
modules beside `reach` hold the rest: `initial`, `colouring`, `shading`, `tiling`, `lattice`, `cell`,
`repeat` and `wrapping`. About 1 960 lines of tests remain in the transparency root, which is a
different split; `shape.rs`, `readings.rs` and `blending_space.rs` are each above 550 lines and are
the next cuts if one is wanted.

## 8. Built — `Document` (`crates/pdf-syntax/src/document.rs`, with `document/`), ADR 2096, and `ColourSpace` (`crates/pdf-colour/src/colour.rs`, with `colour/`), ADR 2097

`Document` was one 1 880-line block of 66 methods; `ColourSpace` one 1 740-line block of 45 methods in a
7 600-line file that also held the compositing key, the grey and RGB routes, the presses, the ink
search and the CIE arithmetic. Both are now a root and one module per concern.

`document.rs` keeps the state (`Document`), the entry points (`open`, `open_with_limits`,
`open_with_password`, `empty`), the bounds, the public result types and the tests; nine modules hold
`impl Document` for one concern: `objects` (resolution and the catalogue), `parsing` (one object at an
offset), `repair` (an entry the file disproves), `encryption` (§7.6, both directions), `object_streams`
(§7.5.7), `reports` (what a recovery found), `decoding` (§7.4's chain), `images` (a codec still to be
applied) and `sources` (how a caller reads the decoded bytes); `cache` holds the decoded-stream cache.
About 600 lines of the root are tests, which are a different split.

`colour.rs` keeps `ColourSpace`, `Tints`, `Separations`, `Reading` and the tests (about 1 460 lines);
fifteen modules hold the rest: the `impl ColourSpace` by concern (`parsing`, `separation`, `queries`,
`to_rgb`, `to_cmyk`) and the free types and functions by clause (`compositing`, `spots`, `grey_route`,
`rgb_route`, `conversion`, `device_cmyk`, `press`, `ink_search`, `cie`, `entries`). About 1 460 lines of
tests remain in the root, a different split; `ink_search.rs`, `press.rs` and `spots.rs` are each above
600 lines and are the next cuts if one is wanted.

## 9. Built — `App` (`crates/viewer-ui/src/bin/quorra/surface.rs`, with `surface/`), ADR 2128

`surface.rs` held one `impl App` of 1 652 lines and 30 methods: the frame of a window with a device,
the frame of one without, the bring-up and the settling after it. It is now a root and five modules.
The root keeps `Surface`, `State`, the four accessors (`device_window`, `device_window_ref`,
`composer`, `composer_ref`), `redraw_requested` (the tick that routes to a path) and the free
functions `predicted`, `in_the_window` and `wake_on`; each module holds an `impl App` for one
concern: `device_frame` (adopt, ask, place, the placements and `present`), `device_adoption` (taking
a finished picture in, naming the lane, a reported fault or a refused swapchain), `processor_frame`
(the same tick over `composer`), `bring_up` (the device or the software surface) and `settling`
(the cadence and the compiled pipelines). The entry said to read it against `window.rs` and
`viewer-device`'s `renderer/` first; the cut follows the two paths, and neither file was touched.

## 10. Built — `Encoder` (`crates/render-raster/src/scene.rs`, with `scene/`), ADR 2144

The census's second impl block by lines once the first four were split: one `impl Encoder` of 1 511
lines and 40 methods, the walk that turns a display list into the window's scene. `scene.rs` is now
the root — the state (`Encoder`), the types the concerns share (`ClipOutline`, `ResolvedClip`,
`Admitted`, `ShadedPaint`, `GroupParts`, `FunctionPaints`), the constructors, `commands` (the one
routing walk) and the view bookkeeping — and six modules each hold an `impl Encoder` for one concern:
`clips` (chains, what they admit, §14.11.2.1's crop), `groups` (a group, a soft mask, the knockout
state), `fills`, `shadings` (ramps, function programs, sampled grids, meshes), `images` and
`outlines`. `vocabulary` holds the free functions that state a display-list value as raster's
(transform, colour, blend mode, path, image spec); `own_space` was already there. About 330 lines of
the root are state and types; `shadings.rs` is the largest module at about 530 lines and is the next
cut if one is wanted.

## 11. Built — `Open` (`crates/viewer-core/src/open.rs`, with `open/`), ADR 2160

One document's state and the walk over it: one `impl Open` of 1 356 lines and 44 methods in a
2 545-line file. `open.rs` is the root — the state (`Open`), the types the concerns share (`Spot`,
`Chosen`, `OnScreen`, `Interpreted`, `Done`, `Committed`, `Read` and the rest), the three
constructors (`new`, `read`, `around`), `first_read`, `interpret`, `Preparation` and `Prepared`, and
the tests — and seven modules each hold an `impl Open` for one concern: `deferred` (the outline, the
placed page tree and the document's report, read at first use), `interpretation` (what is stale, the
page showing at a point, the selection's spans), `pages` (a page, its size, the count),
`edits` (resolve, apply, log, replay, commit), `magnification` (zoom, step, scroll origin, with
`raster_extent`, `content_box` and `fitted`), `fragments` (Annex O's parameters) and
`destinations` (§12.3.2.1's views). About 1 150 lines of the root are the state's field
documentation and the types, a different split; `fragments.rs` is the largest module at about 390
lines and `apply_parameter` in it the next cut if one is wanted.

## 12. Built — `Walk` (`crates/pdf-archive/src/survey.rs`, with `survey/`), ADR 2176

The survey's one `impl Walk` of 1 457 lines and 52 methods, in a 4 300-line file. `survey.rs` is the
root — `Survey` and the observation types a requirement reads, the walk's state (`Walk`, `State`,
`Context`, `Operands`), the free functions that classify a space or a profile, and the tests — and
eleven modules each hold an `impl Walk` for one concern: `pages`, `operators` (the stream loop),
`fonts`, `xobjects`, `graphics_state`, `colour_spaces`, `painting`, `patterns`, `records`, `literals`
and `marked_content`. About 1 500 lines of tests and 1 000 of `Survey` and its types remain in the
root, a different split.

## 13. Built — `Session` (`crates/viewer-ffi/src/session.rs`, with `session/`), ADR 2192

The highest ranked block outside the active interpreter work: 1 291 lines and 97 methods.
The root keeps the state, `new`, `reads`, the one command router `handle`, the free helpers and
the tests. Nine modules hold the wrappers by concern: `documents`, `navigation`, `frames`,
`printing`, `editing`, `policy`, `scripts`, `reports` and `archiving`. No visibility widened;
all method signatures and bodies retain their tokens, including literal contents.

## 14. Built — the interpreter's §9 block and its operator loop (`content/text.rs` with `text/`, `content/run.rs`), ADRs 2288, 2304 and 2320

Two blocks of the one `Interpreter` type again. `text.rs` is the root of the §9 block — the types a
sibling outside `text/` reads (`TextObject`, `Coverage`, the re-exported `WordGaps`) and the tests —
over eight children: `show`, `readback`, `word_gaps`, `glyph_painting`, `placement`, `paint`,
`end_object` and `type3` (ADR 2288). `show_text` in `show` is the setup of one string and a loop of
one method per step of §9.4.4 — `read_code`, `place_in_text_layer`, `draw_code`, `advance_past` —
with the string's constants and tallies in one private `ShowString` (ADR 2320). `run.rs`'s loop
reads a token and calls `run_operator`, whose one `match` over Table 50 sends each operator to the
method for its family, the loop's thirteen locals one `Machine` each method destructures (ADR 2304).
The block is still the census's largest by lines, because its families are; section 16 names the
arms that are the next cuts if one is wanted.

## 15. Files with no large block

`viewer-ffi/src/abi/` (the exported surface, one child per concern since ADR 2272; `wc -l` sizes them),
`pdf-colour/src/colour.rs`, `pdf-model/src/appearance.rs` (one `construct`
arm per annotation subtype), `pdf-signature/src/signature.rs`. Test files (`pdf-model/tests/oracle.rs`,
`pdf-transform/tests/archive.rs`, `viewer-core/tests/headless.rs`) are lists of scenes, ranked by
the census but not god classes: the remedy is a directory of test modules by clause, and it is a
different round.

The confined protocol is a root and one codec per concern under `protocol/` — the question and
answer dialogues in `protocol/script_dialogues.rs` and the signature-policy codecs in
`protocol/published_policy.rs` among them (ADRs 2226, 2227, 2256); each move kept helper bodies,
tags and errors, and `tools/state.sh gods` sizes what remains.

The signature root's security-store and legal-attestation concerns live in
private `signature/security_store.rs` and `signature/legal_information.rs`
(ADRs 2216–2217). Fourteen items moved with unchanged tokens and literals;
explicit root re-exports retain the nine public names. The cryptographic
implementation and original tests remain in `signature.rs`.

## 16. Functions

`archive/prepare.rs::of`, `linearize.rs::place` and `lay_out`, `archive/rewrite.rs::describe`,
`notes.rs::about_one` and `function.rs::apply_operator`; in `content/run.rs`, `run_text_op` and
`run_marked_content_op`'s `BDC` arm, which reads six property-list entries in turn. The codec
functions under `protocol/` (`command.rs`'s `encode_command` and `decode_command_holding`,
`answer.rs`'s `encode_answer`, `event.rs`'s `encode_event`) are exhaustive matches over a
`viewer-core` enum on purpose and want splitting only if the vocabulary does. The longest functions
the census prints are tests — `protocol/tests/panel_answers.rs`'s round trip and `launch_path.rs`'s
bands — which are lists of cases, the remedy section 15 names for test files.

## 17. Next, in the census's order

`tools/state.sh gods`'s impl-block table, read past the blocks split above: `Host` in
`viewer-ui/src/bin/quorra-confined.rs`, `Vfs` in `pdf-vfs/src/lib.rs`, `Carry` in
`pdf-transform/src/structure.rs` and `App` in `viewer-ui/src/bin/quorra/typing.rs`, in that order
until the census says otherwise; each is split as entry 1 was, by concern, behind the proof the
opening paragraph names.

## Found, not fixed

- None in the code the entries moved: each move changed no line of behaviour.
- Several `pub(super)` fields on `GlyphPainting` and `WordGaps` (`content/text/`) are read in one
  sibling only and could be reached through a method (`doc/history/1727`).
- The intra-doc links the splits wrote to `Interpreter::…` and `super::…` are long lines a later
  pass may rewrap (`doc/history/1719`, `doc/history/1727`).
