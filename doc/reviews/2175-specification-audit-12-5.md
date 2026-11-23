# Specification audit of ISO 32000-2 §12.5 (annotations) — round 2175

The project's ninth specification audit, made beside the other lanes in the shape of the owner's
review 2038 ([`2038-specification-audit.md`](2038-specification-audit.md)) and of audits 2128 to
2171 ([`2171-specification-audit-11.md`](2171-specification-audit-11.md)). Baseline: main at
`f6e2316a`. Evidence: [`2175-specification-audit-12-5/`](2175-specification-audit-12-5/) —
`findings.json` (2038's schema) and `expected-failures.json` (the failing tests of each behaviour
finding). The same entries are in `2038-specification-audit/expected-failures-current.json`.

**This is an audit and fixes nothing.** A fix round takes the findings by the groups at the end.

## Method

1. Every `shall` and `should` of §12.5.1 to §12.5.6.25 (Tables 166 to 196) was extracted from
   `doc/md/ISO_32000-2_sponsored_EC3.md`, lines 11093 to 11870, and read in place; §12.5.3 and
   §12.5.5 were read whole.
2. 2038's §12.5 findings were read first and are not repeated: B35, B68, D12, D35, D40, D49, D68,
   T13, T25, T26, T34, T46, T75, T86, T87, T113, T124, T136, T140, T142, T143, T161. No audit since
   filed a §12.5 finding.
3. The reading was traced through `pdf-model`'s `annotation.rs` (flags, `decide`, §12.5.5's map,
   the appearance chosen by `/AS`), `content/annotations.rs` (the drawing, the group, `/BM`),
   `appearance.rs` (each subtype's construction, borders, colours, `/RD`, the line, polygon, ink,
   text markup and free text), `markup.rs` and `popup.rs` (§12.5.6.2's groups and replies),
   `view/annotations.rs`, `view/fields.rs`, `link.rs`, `tab_order.rs` and `restriction.rs`, and
   `viewer-core`'s `interact.rs`, `viewer/pointer.rs` and `viewer/editing.rs` — what a pointer, a
   key and an edit reach.
4. The lens was the earlier audits' habits, and one of them carried the audit: **a rule keyed on a
   name applies wherever the name is read.** Table 167's flags are read where an annotation is
   *drawn*; the findings are where it is *reached* — by a pointer, a click, a tab, an edit.
5. About twenty fixtures were probed end to end on `render-cpu` and through the public hit tests
   and `viewer_core::Viewer`. Each B finding has a failing test with a control that passes on main:
   `pdf-model/tests/annotation_clause12_5_spec_audit.rs` (a module of `annotations_suite`) and
   `viewer-core/tests/annotation_interaction_flags_spec_audit.rs`. No portable-suite test was added:
   four findings are interaction, which `portable-conformance/` does not state, and two are reports
   on malformed input.

What holds, probed or read and passing on main, is not listed one by one. Among it: §12.5.5's
three steps (the transformed box's extreme corners onto `/Rect`, `AA = Matrix × A`) and a stored
appearance run as a transparency group, isolated and knockout from its `/Group`; `/R` and `/D`
falling back to `/N`; an `/AS` naming an undefined state drawing nothing; the `Invisible`, `Hidden`,
`Print`, `NoView` and `ToggleNoView` bits on screen and on paper, including "If the annotation does
not contain any appearance streams this flag shall be ignored"; NoZoom and NoRotate pivoting on
`/Rect`'s upper-left corner in the drawing, and §12.5.6.4's Text override; `/BS` ignoring `/Border`
whole, radii included; Table 168's unknown `/S` read as solid and `/D`'s default `[3]`; `/C` and
`/IC` with 0, 1, 3 and 4 components and a report for 2; `/RD` within both bounds; `/LLE` and `/LLO`
non-negative; `/Path` making `/Vertices` ignored; Table 182's quadrilaterals in either winding;
§12.5.6.2's group attributes read from the primary, both by the popup and by every construction's
`/C`; Table 166's `/M` shown as written; Table 176's `/QuadPoints` falling back to `/Rect` when any
coordinate leaves it; `LockedContents` refusing a retyped free text or note.

Readings checked and left:

- **A stored appearance ignores the annotation's `/ca` and `/CA`** (the item audit 2171 left
  unfiled). §12.5.5 composites the group "using the values of the BM , ca and CA entries in the
  annotation dictionary"; Table 166 says of each opacity "The specified value shall not be used if
  the annotation has an appearance stream … the appearance stream shall specify any transparency",
  and §12.5.2's last sentence lists both among the keys a reader shall ignore. The standard
  contradicts itself, ADRs 0030 and 0579 chose Table 166's two statements over the one, and rows
  12.5.2 and 12.5.5 are `departed` for it. Not a finding.
- **An annotation hidden by optional content still interacts.** §8.11.3.3 and Table 166's `/OC`
  speak of visibility and drawing only; Table 167's "or allow it to interact" is the flags'. A
  reader may well expect an invisible link not to be clickable, but no sentence says so.
- **"Selected" in Table 167's `ToggleNoView`** is read as the pointer's hover or press; keyboard
  focus is not taken as selection. Acrobat's "selected" is a comment-editing state with no
  counterpart here, and the clause does not define the word.
- **A `/D` subdictionary that lacks `/AS`'s state draws nothing while pressed**, where `/N` has the
  state. That is §12.5.5's "reasonable behaviour (such as displaying nothing)" for the stream the
  pointer asks for, and Table 170's default to `/N` is for an absent `/D`.
- **Malformed `/C` component values and a negative `/BS /W`** are clamped or read as no border in
  silence; noted, not filed, as B06 carries the pattern.

## Findings

Six behaviour gaps. In order of how much a person would meet them:

- **2175-B01** — a NoZoom or NoRotate annotation is hit-tested at its unaltered `/Rect`. The
  drawing applies §12.5.3's alteration; `annotation_at`, `widgets_at` and `link::at` do not, so at
  magnification 0.5 a NoZoom square inked over [10, 50] × [50, 90] answers nothing at (40, 55), and
  a NoRotate one turned on a `/Rotate 90` page answers nothing where it is drawn. Every Text note is
  affected (§12.5.6.4 sets both flags for it), and so its popup, rollover and triggers.
- **2175-B02** — Table 167's interaction bits are read by `annotation_at` alone. A Hidden, NoView or
  ReadOnly link is activated by a click (a `/F 6` link to page two turns the page);
  `field_at`, which a host's click on a field asks, returns a Hidden or NoView widget; and the Tab
  key's focus walks every annotation of the page, hidden ones included.
- **2175-B03** — a widget of a ReadOnly field interacts: its own bit 7 is ignored as Table 167
  says, and the field's Ff bit 1 that "subsumes" it is not read, so Table 227's "should not respond
  to mouse clicks nor change their appearance" is not met.
- **2175-B04** — `LockedContents` on a widget does not restrict filling its field. Bit 10 is asked
  only of `Operation::Annotate`; bit 8's row names "the value of a form field" as an annotation's
  contents. It is a restriction, so the fix asks the policy rather than refusing.
- **2175-B05** — an `/AP` subdictionary with no `/AS` draws nothing in silence. Table 166 makes
  `/AS` required there; §12.5.5's "displaying nothing" is for an `/AS` that names an undefined
  state, and an absent one is the third answer.
- **2175-B06** — `/Rect`, `/BBox` and `/Matrix` are read through a `filter_map` that drops a
  non-number and shifts the rest: `/Rect [10 /x 10 50 50]` is drawn at `[10 10 50 50]`, a
  five-number `/Matrix` is the identity, both unreported.

## Ledger rows contradicted

All stay as they are until the fix round, which re-reads each: `12.5.3` (B01, B02, B03, B04 —
its note states NoZoom and NoRotate "applied" without the active area); `12.5.1` (B02 — its note
says §12.5.3's flags are read "through `annotation::interacts`, for every annotation"); `12.5.5`
(B01's active area, B05, B06's `/Matrix`); `12.5.2` (B05, B06's `/Rect`); `12.5.6.4` (B01);
`12.5.6.5` (B02, B01); `12.7.4.1` (B03 — "ReadOnly … constrain interaction" holds for the value,
not for the widget's pointer); `7.9.5` (B06).

## Suggested fix groups

1. **One interaction gate** — B02 and B03: `annotation::interacts` gains the widget's field Ff
   bit 1, and every route a person reaches an annotation by asks it: `link::at`'s callers in
   `viewer_core::interact` (and `Query::LinkAt`), `view::field_at`/`widgets_at`, and the focus
   step over `tab_order::order`. `pdf-model` and `viewer-core`.
2. **The active area moves with the drawing** — B01: the point mapped through the inverse of
   `ViewGeometry::adjustment` (with `view_flags`'s subtype override) before `/Rect` or
   `/QuadPoints` is tested, in the same three hit tests. Best taken after group 1, as both edit
   the same functions.
3. **`restriction::asserted` for a field** — B04: `FillInForm` asserts `AnnotationLocked` where a
   widget of the named field sets bit 10, so the four levels attach as for every restriction.
4. **`annotation.rs`'s readers** — B05 and B06: an absent `/AS` reported beside the nothing it
   draws; `rectangle` and `matrix` reading exactly four and six numbers and reporting anything
   else, with `rectangle_covers` and `widgets_at` taking the same reading.
