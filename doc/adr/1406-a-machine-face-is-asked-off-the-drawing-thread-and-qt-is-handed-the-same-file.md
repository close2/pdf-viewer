# 1406 — A machine face is asked off the drawing thread, and Qt is handed the same file

Session 1284. Status: **accepted** and built.
Context: `crates/viewer-host/src/machine_faces.rs` (new), `crates/pdf-font/src/substitute.rs`
(`face_covers`, additive), `crates/viewer-ui/src/chrome.rs` (`Chrome::wake_with`, `take_arrivals`,
`searching`, `settle`), `crates/viewer-ui/src/bin/{quorra,quorra-confined}.rs`,
`crates/viewer-qt/src/{host,bridge}.rs`, `crates/viewer-qt/cpp/window.{h,cpp}` (`pumpFaces`),
`crates/viewer-ui/tests/panel.rs`, `crates/viewer-ui/examples/chrome_coverage.rs`.
Amends: ADR 1382 section 3 (the walk's place). Keeps ADR 1382's search and ADR 0133's argument.
Clauses: ISO 32000-2 §9.7.4.2 (the covering search reused), §9.6.2.2.

## 1. What was measured

`quorra --trace=frames,launch` under `Xvfb` on `llvmpipe`, release build, one launch per row; "cold"
is the font files evicted from the page cache with `posix_fadvise(DONTNEED)` first. Scratch drives in
`scratchpad/r1284/fx/`.

| launch | first present | the frame that searched |
|---|---|---|
| two Latin-named documents | 109 to 170 ms | host 0.2 ms |
| a Latin- and a Chinese-named document | 104 to 130 ms | the frame after it: host 1024.6 / 1032.2 ms cold, 100.0 / 120.4 warm |
| one Chinese-named document | 104 to 141 ms | none: a strip of one is not drawn |
| a document opening on its outline panel (`/PageMode /UseOutlines`) whose one item is Chinese | **1136.9 / 1017.9 ms cold, 205.4 / 204.3 warm** | the frame before first present: host 1029.9 / 916.3 cold |

So ADR 1382's walk lands before first present whenever page one's chrome holds such a character —
`CLAUDE.md` principle 2's "[n]o system font enumeration … on the launch path", broken — and where it
lands after, it holds the event thread and every input for a second.

## 2. Decision

- `viewer_host::machine_faces::MachineFaces::ask` never searches on the calling thread. It answers
  a character from what is known (including a face already found, one `cmap` lookup), else queues it
  on one thread of its own and answers `Pending`. The thread asks faces found earlier first
  (`pdf_font::substitute::face_covers`), then `installed_covering`, records the answer and wakes the
  window. No thread exists until a character needs one.
- `quorra`'s chrome draws the box for a pending character, and the loop's `EventLoopProxy` wakes
  it to draw again (ADR 0391's rule: what a refresh waits for is the present, never another
  thread's work). `quorra-confined`, which has no proxy, looks on its drawing poll.
- **`quorra-qt` takes the same answer.** Eleven installed files share the family name `Droid Sans`
  here; Qt falls back by family and drew the strip's Chinese as boxes. The host asks
  `machine_faces` for every character of a label that `compiled_in_lacks`, and `pumpFaces` hands the
  file the search named to `QFontDatabase::addApplicationFontFromData` on a timer armed only while a
  search is out, then sets the strip in the family it registers (`Droid Sans Fallback`) after the
  platform's own. Registering the file alone mends Qt's fallback: a standalone Qt program drew the
  same label as boxes without it and correctly with it.
- `quorra-gtk` is unchanged: Pango draws the label from the same machine without being told.

## 3. After

| launch | first present | the frame that drew the face |
|---|---|---|
| outline panel, Chinese item | 104.4 / 92.6 ms cold, 93.6 warm | host under 1 ms |
| a Latin- and a Chinese-named document | 114.4 / 122.9 ms cold, 75.6 warm | at 0.98 s cold: host 0.3 ms |

Cost: a first frame may draw boxes where a face will stand a second later, cold, and a line's width
changes when it lands. The popup's count of undrawable characters counts them until then.
