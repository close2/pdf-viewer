# 1528 — Qt's document node is placed by the page area, measured 371 × 61 pixels off before

Status: accepted. Session 1346. Carries ADR 1516's construction to `viewer-qt`, which that ADR's
section 3 left unmeasured.
Context: ISO 32000-2 §14.7 on AT-SPI; `viewer_accessibility::Reading::at`; trap 102.
Code: `crates/viewer-qt/cpp/window.cpp` (`MainWindow::reportPageArea`), `crates/viewer-qt/src/bridge.rs`
(`page_placed`), `crates/viewer-qt/src/access.rs` (`Host::page_placed`, `Host::speak`).
Tests: the drive's `29-field-extents`, which now reads the document's node in Qt too.

## 1. The measurement

`quorra-qt` hands AccessKit the window's frame and contents (`QWidget::frameGeometry`,
`QWidget::geometry`), and AccessKit adds the contents' screen origin to every node. The nodes'
bounds are the viewport's, and the viewport is the page area, which sits below the menu and the
toolbar and beside the panel. Driven under Xvfb (release, at-spi, `drive-field.pdf`), the extents
probe read `GetCharacterExtents(1)` on Qt's `QLineEdit` at 461, 428, 7 × 18, inside the field's
454, 428, 303 × 40. On the document's node for the same field it read 83, 367, 10 × 18, which is
371 pixels left of the field and 61 above it. The offset ADR 1516 suspected was there.

## 2. The fix is ADR 1516's, with the number sent from C++

`MainWindow::reportPageArea` sends `page_->mapTo(this, (0, 0))`, scaled to device pixels, as
`page_placed`. It is sent wherever the window reports its place and wherever the page area
reports a resize: a panel opened beside the page changes its size and its place together. The Rust
side keeps the value and passes it to `Reading::at`, so the document node carries it as its
transform. A change re-publishes the tree. Driven again: the document node's character 1 is at
464, 439, 10 × 18, inside the field. `29-field-extents` now appends the document node's box to
Qt's line and calls the step `works` only when that box, with a nonzero width (trap 102), also lies
inside the toolkit's field. A step that read only the toolkit's widget could not have seen this.
