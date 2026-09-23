# 1316 — A layout is asked for outside the allocation, an answer says it is one, and an embedded document is a tab

Session 1239. Status: **accepted** and built.
Context: `crates/viewer-core/src/interact.rs` (`Arrival`, `import`), `crates/viewer-core/src/viewer.rs`
(`respond`, `supply`), `crates/viewer-gtk/src/host.rs` (`apply_chrome`, `extracted`,
`opened_beside`), `crates/viewer-qt/src/host.rs` (`extracted`, `opened_beside`),
`crates/viewer-host/src/documents.rs` (`Named::embedded`, `Arriving::bytes`, `Arrivals::wait_held`),
`crates/viewer-host/src/policy.rs` (`opens_as_document`, `opening_embedded`),
`crates/viewer-ui/src/bin/quorra/{files.rs,arrivals.rs,app.rs,dispatch.rs}`, `doc/todo/30`.
Builds: ADR 1303 (what round 1233 left), ADR 1275 (`Arrivals`), ADR 1291 (the submission's
answer), ADR 0431 (`ef`'s remainder), ADR 1190 and ADR 0713 Decision 2 (the confined window).
Clauses: ISO 32000-2 §12.7.6.2, §12.7.8 (Table 246), Table 29, §O.2.1.

Every item below was driven under Xvfb with two documents, the one stating a page mode or a
fragment opened second as well as first.

## 1. A server's answer is worded as one

Table 246's `/Status` is "a status string that shall be displayed indicating the result of an
action, typically a submit-form action". `Command::Respond` and §12.7.6.4's action share one import,
because §12.7.6.2's answer is incorporated "into the interactive form" as an imported file is. What
differs is the word in front of each sentence. `interact::Arrival` supplies it: `import-data` for the
action, `submit-form answer` for the answer, whose count sentence says the fields were `imported from`
the URL.

## 2. GTK asks for a layout outside the allocation that hid the chrome

Page one opens from `resized`, which is `GtkDrawingArea::resize`, inside GTK's allocation. Under
`FullScreen`, `apply_chrome` hid the panel and the status line there, and GTK did not lay the window
out again until something else asked for a frame. With one document that was the first key press.
`--trace=launch,frames` showed `TRANSITION Wipe … two 509x1019 pages` and then `Resize { 890x1090 }`
in that order. With two documents the second tab's arrival hid the defect, which is why only the
first transition after launch showed it. `apply_chrome` now queues a resize from the idle queue, so
the resize and page one's new raster land about 80 ms after launch and before any key. Driven: both
transitions are shaped at 890x1090.

## 3. Qt's first menu heading: not reproduced in fifty

Fifty launches were tried: map, then a click on the first heading. Twelve had no delay, twelve waited
0.5 s and six waited 2 s, with two documents in both orders. Ten more pressed and released with the
pointer parked over where *copy → off* would be, and ten more opened the full-screen document
alone. None chose a level; the detector (a second `Restrict` in `--trace=events`) was checked
against a deliberate choice. ADR 1303's record stands as written.

## 4. `quorra-confined` still holds one document

ADR 0713 Decision 2 scoped the window to one document. ADR 1190 replaced that decision's ground and
kept its conclusion, with the rule that a host owes the control for an operation exactly when it
performs that operation. The window takes no second path, offers no chooser and refuses §12.6.4.3.
The wire carries `Command::Beside` because the wire encodes every command, not because this window
performs one. The decision is unchanged, so there is no ADR 1315. `doc/todo/30` now says so, and it
names the question a reversal must answer first: whether two documents share one worker's confinement.

## 5. §O.2.1's `ef` opens a tab in all three windows

The row's `shall` is "the PDF processor shall open the embedded file contained within the
EmbeddedFiles name tree identified by name". `quorra` carried it out by replacing its first document,
whichever tab had named the file. GTK and Qt did not open it at all; they declined to write it. Now
each window queues the bytes in `Arrivals::wait_held` and opens them in a tab of their own, with the
rest of the fragment applied to them. `Arriving::bytes` serves the first open and §7.6.4.1's second
attempt, so a held document is never looked for on disk. It opens in front where its holder was in
front, and behind where its holder was behind. `quorra`'s `embedded` field is gone, since the
document in front now always has a path.

`doc/todo/38`'s descriptor route for `Edit::Attach` is left unbuilt, because no host on the confined
boundary makes an edit. A route with no sender waits for its first one.
