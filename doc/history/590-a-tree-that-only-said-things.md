# 590 — A tree that only said things, and the queue nobody could drain

**A screen reader could hear this program's page and could not act on it**: not one node of the
AccessKit tree declared an action, so a conforming client had nothing to request. Three are
declared now, each resolves to a *place* in the viewport's own pixels, and the host carries each
out with a `Command` it already had. Two defects came out of making it work — the request queue was
drained from a function that runs only on a page turn, and the tree went stale on every edit, so a
check box ticked *with the mouse* had been announced as unticked since ADR 0214.

Date: 2026-08-18.
ADR: [0425](../adr/0425-a-tree-that-only-said-things.md).

## What it took, and why that order

`doc/todo/README.md`'s index line for item 31 named five remainders and **two of them had been
closed for rounds** — a `Form`'s control role in session 503 (ADR 0338) and the `Text` interface,
with the thousand-page measurement, in session 559 (ADR 0394). The file itself was current; the
index line was not, and it is corrected. What the file's own evidence ranks first is what this
round took: *actions*, which ADR 0394 calls "the sharpest entry left on the file" and which two
earlier rounds had each recorded as an invitation their own work created.

## Files

- `crates/viewer-core/src/accessibility.rs` — `AccessibilityNode::annotation`, the annotation an
  element's own §14.7.5.3 object reference names.
- `crates/viewer-confined/src/protocol/panels.rs` — it crosses the confinement's wire too.
- `crates/viewer-accessibility/src/tree.rs` — the three declarations.
- `crates/viewer-accessibility/src/bridge.rs` — `Act`, the resolution against the published tree,
  and the waker `Bridge::new` now takes.
- `crates/viewer-ui/src/bin/pdf-viewer/{access,window,dispatch,typing}.rs`,
  `crates/viewer-ui/src/bin/pdf-viewer.rs` — `App::act`, `App::click_page`, the event-loop proxy,
  and republishing on an edit.
- `crates/viewer-core/tests/accessibility_census.rs` — one count added and the whole census
  ratcheted, which closes `doc/todo/05`'s third instrument.
- `doc/conformance/ledger.toml` — §12.5.1, §14.7, §14.7.5.3, §14.8.4.7.2.

## The errata run

`spec-errata emit` over clause 14 before writing, as `doc/todo/02` §4 requires. Issue #437 replaces
Table 368's `Annot` and `Form` descriptions, and **the §14.8.4.7.2 ledger row has recorded that
since session 418 while quoting the struck sentence two sentences later**. Four places in `crates/`
quoted it as current text; all four are corrected. `spec-errata check` sees the ledger and the ADR
and not the four, because prose broken across `//!` lines is below its five-word threshold in the
form it takes there.

## The bus

`doc/verify.md`'s recipe with a client that asks for things rather than only reading. Nine `Form`
elements of `annotation-button-widget.pdf` carry `org.a11y.atspi.Action` with one action named
`click`; `DoAction` ticks the unticked box, unticks the ticked one, and is refused on the read-only
one with Table 227 named. On ISO 32000-2 the page answers `CharacterCount` 512 and
`SetCaretOffset`, and `DoAction` on the cover's two `Link` elements opened both URIs. No screen
reader was run: Orca is not installed here.

## Gates

The whole of `doc/todo/02` §2, plus the accessibility census as a new §2 line. Every one green;
the numbers are `tools/state.sh`'s to print.
