# 3193 — A TrueType font a photograph takes out of the file needs no route

Session 2123, an archive round in the `lane3` worktree. Status: **accepted** and built. Answers
`doc/pdfa-blocking-issues.md`'s R11 for the three documents it stopped alone under `as-if-printed`
(`issue20489`, `issue13316_reduced`, `sample_fonts_solidconvertor`). Context: ADR 2520 (the restated
`cmap`), ADR 2744 (restated over the face the substitution embeds), ADR 2394 (the photographed page),
ADR 2454 (a font only photographed content selected is not reported), ADR 2804 (a page group a
photograph takes off its page needs no default); `crates/pdf-transform/src/archive/decision.rs`,
`fonts/photographed.rs`, `fonts/truetype_cmap.rs`, `rewrite.rs`.

## 1. What the seven documents stopped on

Each document R11 held was converted with a debugging print at every refusal of
`truetype_cmap::restate_one`. Five stopped at `as_written` (no `/FontFile2` in the descriptor as the
conversion writes it) and two at `codes_drawn` (a program this reader does not draw from). In every
one of the three that R11 stopped alone, the font the restatement refused was one the substitution
had *photographed around* (ADR 2394's wider gate, `as-if-printed`): the page showing it was replaced
by its photograph, and the font with the page's resources. The restatement read the source's
fonts, refused the first it could not restate, and so refused a whole row over fonts no object of
the output would hold. `SimFang-variant` and `XiaoBiaoSong` are the other kind: the substitution
refuses them before any photograph (a descriptor written inside the font, R10), so no route here
reaches them; `507676` stops on five other rows.

## 2. The decision

ISO 19005-2 section 6.2.11.6 and ISO 19005-4 section 6.2.10.6 judge the TrueType fonts a file
holds. **A font whose every route ran through a photographed page's resources is in no object of the
output**, so it is bound by neither clause. Three changes follow:

1. `Substitutes::photographed_away` holds the font objects the pages selected that the output no
   longer reaches once the photographs have taken their pages' content and resources
   (`photographed::taken_away`, the complement of ADR 2454's `still_held`).
2. `truetype_cmap::restated` passes over such a font: there is no program of it to restate.
3. `decision::decide` answers each of the seven rows R11 names, where every place it fails is such
   a font, by the photograph at the losses it already takes — the shape ADR 2804 gave a page's
   `/Group`. No loss is added: the photograph was already taken and already reported.

## 3. A page tree node's `/Resources` no page inherits

`sample_fonts_solidconvertor` states its fonts twice: in the page's `/Resources` and in the page
tree node's, the same object. ISO 32000-2 §7.7.3.4 makes a page tree node's `/Resources` an
inheritable attribute, used by a page that states none; the photograph states its own, so once
every page under a node is photographed no page inherits the node's. The writer reached it all the
same, and wrote every font it named. **So such a node is written without `/Resources`**
(`Substitutes::inherited_by_none`, climbed from each page's `/Parent` chain once; a tree the model
cannot read whole answers nothing). The two walks that model the output (`still_held` for the
report, `names::reached` for ISO 19005-2 section 6.1.8) drop the same key through one method,
`Substitutes::keys_away`, so the report, the name check and the writer agree on what the output
holds. The removal writes nothing a page draws, and is not a step of its own in the report.

## 4. Proof

Two tests in `crates/pdf-transform/tests/archive/truetype_cmap.rs`, each failing against the tree
before this change (the row was answered by a restatement of a font nothing wrote): a symbolic
TrueType font stating `/WinAnsiEncoding` shown on a page photographed for its `Adobe-Identity` font
is answered by the photograph, the output conforms and names no such font; and the same with the
page tree node also naming the font, whose output node states no `/Resources`. The three documents
convert at both parts under `as-if-printed` and pass veraPDF 1.30.3 at 2b and 4; their pages draw
as before but for the photograph's resampling (a page photographed at 300 dpi and drawn at 72) and
`issue20489`'s non-printing annotation, which `as-if-printed` removes as it did.

## 5. What is left

R11's four other documents: `SimFang-variant` and `XiaoBiaoSong` (R10's descriptor inside a font
with no program, then the UTF-8 name fence), `507676` (five other rows), and under `as-shown`,
which takes no wider photograph (A694 item 1), `issue6127`, `issue13316_reduced` and `issue20489`.
