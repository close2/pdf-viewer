# 1908 — The Qt host is a module tree by concern, and what the two hosts share is named

Session 1536, slot 3 of batch eighty-five, the first batch of the project owner's switch of
2026-10-14 to "code improvements/refactoring". Status: **accepted**. Context: ADR 0246 (the Qt
host), ADR 1023 (a comment carries its current reason), ADR 1865 §2 (the toolkit stand-in), and
slot 2's split of `viewer-gtk`'s host in the same batch, whose names these were offered to. It cites no clause and moves no
ledger row.

## 1. The decision

`crates/viewer-qt/src/host.rs` was one file of 6 483 lines: one `impl Host` block of 216 methods
and 41 free items. It is now a root and eighteen children, each one concern's `impl Host` block
with the free items that only that concern uses. A method is found by what it is about.

| module | what it holds |
|---|---|
| `host.rs` | `Host`, `Showing`, `HostError`, the constants, `Host::open`, the opening commands, and the helpers two or more concerns use (`quad`, `named`, `level`, `packed`, `nothing_changed`) |
| `opening` | the launch: the opening thread joined, page one shown as it is read, the preparation |
| `react` | the loop: `dispatch`, `pump`, `drain`, `run`, `turn`, `react`, `refresh`, `take_update`, and the open, the password and the failure events |
| `drawing` | the drawing thread: pages and windows taken back, the rows seen first, the long draw |
| `stand_in` | what the page area paints: `frames()`, `frame`, `frame_pixels` (ADR 1865 §2) |
| `presentation` | §12.4.4's clock and Table 164's transitions |
| `window` | the title, the status line and its sentences, the window's placement, the chrome |
| `input` | the key table's meanings, the window acts, the wheel, the toolbar, the selection, §12.9's measuring |
| `tabs`, `faces` | the strip of open documents and those arriving; the machine's faces for chrome strings |
| `panels`, `find` | the six panels' rows; the find bar |
| `forms`, `notes` | §12.7's placed controls and §12.5.1's focus; §12.5.6.14's popups and the note editor |
| `questions`, `restrictions` | the dialogues a person answers; the levels menu |
| `requests`, `saving`, `printing` | what a document asks of this machine; §7.5.6 and §7.11.4's files; RFC 0004 |
| `tests` | the module's tests, unchanged |

## 2. What moved and what did not

- **No code line changed.** Comparing HEAD's file with the new tree as a multiset of lines leaves
  only these: imports, `pub(super)`, rustfmt re-wrapping a signature that `pub(super)` pushed past
  100 columns, the test file one indent shallower, two test calls written as
  `super::notes::editor_position` and `super::notes::contents_byte`, and the new module documentation.
- **The three `// ---` section comments inside the `impl` block** became the root's, `tabs`' and
  `react`'s module documentation. `window_act`'s documentation had become separated from it, left
  above `showing_what_would_print`; it is back on `window_act`.
- **Visibility.** A private method or item that another concern calls is `pub(super)`. 71 are, and
  each one was checked by taking the widening away and compiling: all 71 failed without it.
  The same check found one field widened for nothing, which was taken back. Nothing is wider than
  `pub(super)`. The bridge's four rule and leader functions are re-exported at `crate::host`, so
  `bridge.rs` and the C++ side are untouched.
- **Three doc links** whose names the moved code no longer imports get their paths through a
  reference definition. The prose is unchanged. `cargo doc --document-private-items` prints HEAD's
  three pre-existing warnings and no new ones.

## 3. The two hosts, side by side

The concerns map one to one onto the GTK host's methods at HEAD. Where they differ, the difference is the toolkit's
event model. Qt's C++ side *pulls*: it asks `rows`, `frame`, `controls` and `popups` across the
bridge whenever an update flag says so. GTK's host *pushes* into widgets it owns
(`place_fields`, `place_popups`, `refresh_chrome`), and it builds those widgets in Rust. Qt builds
them in `cpp/window.cpp`. So GTK has widget-building code with no Qt module to match it, and Qt
has bridge getters where GTK has sinks. Neither is a fix.

**Thirty methods, 430 lines, are the same code in both hosts at HEAD** (identical once comments
are set aside):

- drawing: `ask_for_windows`, `mind_a_long_draw`, `seen_of`, `stop_the_long_draw`, `take_the_thread_back`, `waiting`
- react: `run`, `turn`, `open_failed`
- presentation: `arm_transition`, `face_arrived`, `start_the_clock`, `viewport_rect`
- tabs: `open_the_next`, `open_arriving`, `offer_a_name`, `given_up`, `cancelled`, `open_behind`, `retain_pages`
- requests: `take_the_answers`, `fetch_policies`
- input: `copy_selection`, `has_selection`, `measure`
- window: `caption`, `named`
- saving: `open_embedded`
- find: `searched`
- `Debug::fmt`

Twelve more are 85–96 % alike. A shared module in `viewer-host` that took them would need three
things:

1. The toolkit-free half of the window's state as one struct both `Host`s own: the viewer, the
   `Documents<Showing>` and `Showing` itself, the drawing, the arrivals, the retained and oversized
   pictures, the ahead, the clock and the presenting state, the submitter and its answers.
2. A narrow trait for the two things each toolkit does differently inside those methods: `say`
   (a status label in GTK, an update flag in Qt) and asking for a repaint.
3. `react`'s event match split into the arms both hosts answer alike and the arms that are
   widget work.

It is not built in this batch, as the brief says. Built, it would bring the two hosts' drawing,
tabs and presentation code down to one copy.

**Gates.** Before and after, in exports of HEAD `512af7e5` and of HEAD plus `crates/viewer-qt`:
the drive of `quorra-qt` whole on Xvfb (`doc/history/1536` gives the verdicts);
`cargo nextest run -p viewer-qt` 35 of 35 passed;
`RUSTFLAGS="-D warnings" cargo clippy -p viewer-qt --all-targets` exit 0. `launch_path`
cannot move: `cargo tree -i viewer-qt --workspace` names no crate that depends on `viewer-qt`.
