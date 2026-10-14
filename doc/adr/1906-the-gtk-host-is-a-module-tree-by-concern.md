# 1906 — The GTK host is a module tree by concern

Session 1535, slot 2 of batch eighty-five, the first batch of the project owner's switch of
2026-10-14 to "code improvements/refactoring". Status: **accepted**. Context: ADR 0244 (the GTK
host), ADR 1023 (a comment carries its current reason), ADR 1908 (the Qt host's split in the same
batch, whose module names these are). It cites no clause and moves no ledger row.

## 1. The decision

`crates/viewer-gtk/src/host.rs` was one file of 7 955 lines: two `impl Host` blocks holding 150
methods, and 71 functions, types and constants beside them. It is now a root of 912 lines and eighteen children under
`src/host/`, each one concern's `impl Host` block with the free items only that concern uses. The
names are slot 3's for `viewer-qt` (ADR 1908), so that a method is looked for in the same file in
both hosts.

| module | what it holds |
|---|---|
| `host.rs` | `Host`, `Opening`, `HostError`, `Ui`, `Chrome`, `Panel`, `Showing`, the constants two concerns read, `Host::open`, `Host::me`, `with`, the opening commands |
| `opening` | the viewport's first size, the opening thread joined, page one's read on the status bar, the preparation |
| `react` | `dispatch`, `pump`, `turn`, `run`, `react`, `refresh`, the open, password and failure events, a turn's and a page's reports |
| `drawing`, `stand_in` | the drawing thread's pages and windows taken back and the long draw; the pictures a frame puts up (`placements`, `place`, `Picture`) |
| `presentation` | §12.4.4's clock, the script timers, the transitions' faces |
| `window` | `build_window`, the page area, the header, the title, the status sentences, Table 29's catalog entries, the chrome layer |
| `input` | the key table's GDK side, the pointer, the wheel, the selection, §12.9's measuring, the window acts |
| `tabs`, `panels`, `find` | the strip of open documents; the six panels; the find bar |
| `forms`, `notes`, `rich_text` | §12.7's placed controls; §12.5.6.14's popups and the note editor; Table 172's `/RC` as Pango runs |
| `questions`, `restrictions`, `requests` | the dialogues; the levels menu; what a document sends or fetches |
| `saving`, `printing` | §7.5.6 and §7.11.4's files; the print dialogue |
| `tests` | the module's tests, three paths aside unchanged |

**`rich_text` is the one concern Qt has no file for**: Qt's popup is laid out in C++, and GTK's is
built here from Pango spans, tab arrays and leader painting — 530 lines that would double `notes`.
Qt's `faces` (the machine's faces for chrome strings) has no GTK counterpart, since GTK asks Pango.

## 2. What moved and what did not

- **No code line changed.** `scratchpad/r1535/py/same.py` takes each of HEAD's 232 items — each
  `Host` method on its own — with whitespace, `pub(super)` and rustfmt's trailing commas set aside,
  and finds it as a substring of exactly one new file. 229 are found once; the three others are the
  two tests whose calls now name `super::panels::notebook_page` and `super::rich_text::pango_tabs`,
  and `Ui`, whose one doc link gained a target.
- **Visibility.** A method or item another concern calls is `pub(super)`: 89 declarations, each
  checked by taking every widening away and compiling, and putting back only what failed, then
  `RunFace` (a sibling's signature names it) and `NO_POSITION_TO_SET` (a sibling's doc links to it).
  Ten fields are `pub(super)` because a sibling reads or builds the struct. Nothing is wider than
  `pub(super)`; the crate's public interface is still `Host`, `HostError` and `Opening`.
- **Four root items with one user moved next to it**: `READING_THE_OUTLINE` to `panels`,
  `ScriptAnswered` to `questions`, `SCROLL_STEP` to `input`, `MEASURED_POINT` to `window`.
- **Eleven doc links** whose names the moved code no longer imports gained a path target; the
  prose is unchanged. `cargo doc --document-private-items` prints HEAD's eleven warnings and no
  other.

**Gates.** Before in an export of HEAD `512af7e5`, after in an export of HEAD plus
`crates/viewer-gtk`: the drive of `quorra-gtk` whole on Xvfb, 87 works and 4 not offered in both, every verdict the
same step for step and every pixel count equal; `cargo nextest run -p viewer-gtk` 16 of 16 passed;
`RUSTFLAGS="-D warnings" cargo clippy -p viewer-gtk --all-targets` exit 0; `launch_path` cannot
move, as `cargo tree -i viewer-gtk --workspace` names no crate that depends on `viewer-gtk`.
