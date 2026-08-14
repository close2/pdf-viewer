# 492 — The backdrop retained, and the press a group brings

**Finding.** `doc/todo/23`'s last two constructions were both sayable after all, and the file had
already priced them exactly: §11.4.6's non-isolated knockout group wants the initial backdrop
*retained beside the accumulation* — a buffer discipline, not a new quantity, with the identity
`f × E = S − (1 − f) × B` recovering the clause's shape-1.0 stage from an ordinary draw — and
§11.6.6's group that introduces a four-component space is §11.4.7's pair one scope down,
`Command::Group` carrying the space and a second element list (`pdf_render::GroupBlending`). A
third thing neither item named had to move first: `/AIS` was a page-wide monotone flag, and
`issue18032.pdf` states it inside a form whose group draws nothing, two forms before the knockout
group the flag was refusing — it is a graphics state parameter now, scoped to the group whose
content stated it. Both corpus witnesses left: `issue18032.pdf` **agrees with the CPU oracle**,
and `bug1721218_reduced.pdf` draws in ink and joins `AMBIGUOUS_PAGE_DRAWN_IN_INK` with its own
reading — nearer to `poppler` (0.116 mean grey of 255) than any two references sit to each other
(0.130–0.338). `render-gpu` and `render-quorra` refuse both constructions by name; the frames go
to the oracle.

**Date.** 2026-08-14.
**ADR.** [0327](../adr/0327-the-backdrop-retained-beside-the-accumulation-and-the-press-a-group-brings.md).

**Rebased onto session 493's banding before landing** (ADR 0328, `cec91d5f`): the retained
backdrop `B` is copied to the group's *band* exactly as 0328's `initial_backdrop` copy is, by
the same argument extended one step — the only reader of this construction's result is the
caller's band-rows interpolation, the per-element scratches are clones of `B` and hold the same
rows, and no operation moves a value between rows. Both rounds' unit tests pass together on the
rebased tree.

**Gates, re-run whole on the rebased tree.** Workspace **1800 tests, all green** (this round's
nine among them; doctests clean; clippy silent; fmt clean; conformance 5/5). Corpus: 974
documents, incomplete **62** — this round's two witnesses (`issue18032.pdf`,
`bug1721218_reduced.pdf`) and session 486's `issue19517.pdf` all off the list, nothing arrived. Oracle: 1794 pages, agrees
906, contradicted 67 (unchanged), ambiguous 786; one page moved buckets — `bug1721218_reduced.pdf`
page 1, judged for the first time, ambiguous with the diagnosis above; `issue18032.pdf` page 1
judged for the first time and agreeing. Quorra corpus (rebased tree): 956 pages, 917 agree, 37 differ
(unchanged, pinned), refused 1 → 2 by name — `bug1721218_reduced.pdf`'s refusal renamed from the
texture capacity it still also has, `issue18032.pdf` new. Text extraction 99.3% (24014/24193
words) — the readback rewind around the pair's second run did not double a word. Ink sweep
(todo/00 §7) over all 786 ambiguous pages: 22 at or past −0.68, seventeen of them documents this
tree already calls incomplete, the rest carrying their standing diagnoses (`issue16038.pdf`
−6.17, `issue12295.pdf` −2.84, `checkbox_no_appearance.pdf` −1.200 exactly as recorded,
`issue7821.pdf` −1.12, `issue14297.pdf` −0.93); nothing unexplained, and the round's blast radius
is bounded by three instruments agreeing — the corpus list moved by exactly the two witnesses,
the oracle flagged exactly one bucket change, and the quorra differing list is unchanged.

**Cross-backend discipline (trap 2).** Both new render-cpu closed-form tests were confirmed to
fail with the construction substituted: the knockout scene draws its Multiply element blue
instead of black on the transparent-backdrop substitute (255 apart on two channels), and the
blending scene draws 127 instead of 76 with the pair collapsed to one RGB list — ADR 0251's
51-of-255 gap. Each scene keeps a fractional-coverage pixel (a half-covered column) where
source-over differs from the clause's weighted average by 89 of 255.

**Touched.** `crates/pdf-render/src/display_list.rs` (`GroupBlending`, the `blending` field, the
two-flag knockout documentation), `crates/pdf-render/src/lib.rs`,
`crates/pdf-model/src/content.rs` (`GraphicsState::alpha_is_shape`, `nested_space_departed`,
scoped `alpha_is_shape` doc), `crates/pdf-model/src/content/transparency.rs` (the pair runs with
readback rewind, `group_press`, `stated_elements`, `paired`, the AIS scoping, the
nested-departure record), `crates/pdf-model/src/content/ext_gstate.rs`,
`crates/pdf-model/src/content/{pattern,path,text}.rs` (field), `crates/render-cpu/src/lib.rs`
(`composite_in_own_space`, `knockout_on_backdrop`, `encode_group_command`),
`crates/render-cpu/src/blend.rs` (`knockout_average`), `crates/render-gpu/src/scene.rs` and
`crates/render-quorra/src/scene.rs` (refusals by name), `crates/test-scenes/src/lib.rs` (two
scenes), new `crates/render-cpu/tests/group_constructions.rs`, refusal tests in both backend
suites, `crates/pdf-model/tests/transparency_groups.rs` (three new tests, two pins flipped to
drawn, AIS scoping pinned), `crates/pdf-model/tests/text_render_modes.rs` (pattern),
`crates/pdf-model/tests/oracle.rs` (`AMBIGUOUS_PAGE_DRAWN_IN_INK` gains the page and its
reading), `crates/render-quorra/tests/corpus.rs` (`REFUSED` 1 → 2, documented), ledger rows
§11.4.4, §11.4.5, §11.4.6, §11.4.7, §11.6.4.3, §11.6.6, §11.7.2, §11.7.5.3, `doc/todo/23`
amended, ADR 0327, this file.

**For the next round.** The pair deliberately declines four shapes, each still reported by name:
a three- or one-component group inside a four-component parent (a per-pixel conversion between
two presses — no corpus witness), four components no profile backs, §11.7.5.3's stated black
generation, and a knockout group that also states a press. `/AIS` remains honoured only as a
refusal — honouring it means composing the mask into the shape instead of the object. The owner
asked mid-round for `trim-paths = "all"` in Cargo.toml for sccache; it does not parse on the
pinned stable cargo 1.97.1 (`feature 'trim-paths' is required`, probed on this machine), so it
was not applied — the stable-channel alternative is `--remap-path-prefix` via rustflags, or a
reviewed nightly bump.
