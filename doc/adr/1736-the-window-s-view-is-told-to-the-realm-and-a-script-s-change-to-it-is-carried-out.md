# 1736 — The window's view is told to the realm, and a script's change to it is carried out

Status: accepted and **built**. Session 1450. Supersedes ADR 1724 section 3's first and third rows
(`zoom`, `zoomType`, `scroll`, `layout` and `app.goBack`/`goForward` refused by name); builds on
ADR 1688's focus request and ADR 1640's page request.
Code: `crates/pdf-model/src/view/script_model.rs` (`ScriptEdit::View`, `ViewChange`, `ZoomType`,
`WindowView`), `crates/pdf-model/src/view/scripts.rs` (`ScriptEvent::view`, `set_window_view`,
`take_view_requests`, `ask_view`, `MAX_VIEW_CHANGES`), `crates/pdf-script/src/engine/window.rs`
(the four members and `zoomtype`), `crates/pdf-script/src/engine/pages.rs` (`unrotated_point`),
`crates/pdf-script/src/wire.rs` (version 11, with ADR 1737), `crates/viewer-core/src/scripting.rs`
(`window_view`) and `crates/viewer-core/src/viewer.rs` (two named hunks: the told view in `settle`,
`carry_out_view_requests` and `scroll_to_middle` after `carry_out_focus_requests`).
Tests: `crates/pdf-script/tests/window_view.rs`, `crates/viewer-core/tests/script_view.rs`,
`pages::tests::a_rotated_point_is_read_back_to_where_the_page_states_it`, `tests/wire.rs`.

## 1. The premise, and the half that did not hold

ADR 1724's census: `this.zoomType` 3 documents, `this.zoom` 2, `this.layout` 1; `scroll`, `goBack`
and `goForward` none. ADR 1724 refused the four `Doc` members because no script edit reached a host
as a view and no host told the realm its view. Both are now built. **The history pair is not**: the
brief asked for `app.goBack`/`goForward` as "the viewer's own back/forward", and this viewer keeps
no history of views — `grep -rn 'GoBack\|go_back\|history of views' crates/` names only the census
and the refusal. A person has no previous view to return to, so a script's request would reach
nothing; the pair stays in `surface::REFUSED` with that reason, and moves when a host round builds
a view history for a person first.

## 2. Each member, a documented choice under principle 5

Adobe's *JavaScript for Acrobat API Reference* at `adobe/dc-acrobat-sdk-docs` commit `ab3b42a7`,
"Doc properties" and "Doc methods", is cited and never quoted.

- **The view is told with every event** (`ScriptEvent::view`, `Request::view`), not measured
  against what the realm last heard: it is three small values, and the delta machinery is for the
  fields and the document. `viewer-core` tells the view state after every `settle`, beside the
  magnification §12.5.3's `NoZoom` already reads. Until a host tells, a script reads the
  `/PageLayout` Table 29 says "shall be used when the document is opened" and an `undefined` zoom.
- **`zoom`** is a percentage where 100 is one logical pixel per default user space unit — the
  reading `Open::apply_view` already gives Table 151's `/XYZ` magnification of 1. A write inside
  the reference's 8.33 to 6400 per cent is `Zoom::Scale(n / 100)` and reads back `NoVary`; outside
  it, or not a number, nothing changes and the run says so.
- **`zoomType`** reads `FitPage`, `FitWidth`, `FitHeight` for the host's three modes and `NoVary`
  for a fixed scale. A write of those is the mode; `NoVary` fixes the scale a mode resolved to;
  `FitVisibleWidth` is §12.3.2.2's `/FitBH` pushed as a pending view. `Preferred` (the reader's own
  magnification, which the realm is not told) and `ReflowWidth` (no reflow here) change nothing,
  noted. The `zoomtype` constants are the reference's seven.
- **`layout`** is Table 29's six names, read and written as `Command::Layout` sets them.
- **`scroll(nX, nY)`** reads its point "in rotated user space" as ADR 1724's `getPageBox` does,
  undoes the turn (`unrotated_point`), and crosses in default user space with `this.pageNum`'s
  page. The host turns to that page if it is not showing and sets the scroll so the point's place
  on the raster is the viewport's middle; `settle` clamps it as a wheel's scroll.
- The latest of each kind in one run stands; a view state holds at most 16 changes
  (`MAX_VIEW_CHANGES`), a later one replacing the latest of its kind. Only the document in front
  changes its view, as only it turns a page.

## 3. What was found beside it

Building the viewer test, a page turn's Table 198 `/O` script never reached the runner: in
`viewer-core` no code calls `ViewState::run_page_scripts` or `run_annotation_scripts` (their only
callers are tests), and the action path declines every `JavaScript` action (`action::refused`). So
in a window, past the open sequence, a button's mouse-up script and a page's open and close
scripts are declined, while field, document and timer scripts run. That is `viewer-core`'s, not
this round's; it is in the report for a host round.
