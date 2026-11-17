# 2067 — An empty encoding slot names `.notdef`, a unit's certificate in the report, a window's grid fault across the pipe

Session 2067, a loose-ends round in the gwg worktree. Rows 9.6.5.2, 12.8.3.4.5 and 7.10.5.3; ADR 2948. What rounds
2064, 2057, 2061 and 2066 left. The machine rebooted mid-round; edits survived, the running builds did not.

**The slice.** Four items in the brief's order, each whole or not at all: an empty encoding slot (2064), a report-level
time-stamping unit (2057), Shift + T over a document whose policy references a list (2061), and the confined worker's
empty `Declined` (2066). Items 1, 2 and 4 built; item 3 not started.

**The premise.** All four re-checked open on HEAD: no ADR or record after 2942, 2928, 2936 or 2946 touches them, and
none needs the owner.

**Item 1 (ADR 2948).** §9.6.5.1 gives every program a built-in encoding, §9.6.5.2 makes it "an 'encoding' array that
is part of the font program", and the Adobe Type 1 Font Format (a normative reference) indexes that array by every code
and fills its unassigned slots with `/.notdef`. So an empty slot names `.notdef`, which every program contains, and
§9.2.4 advances it by that glyph's width. `notdef_advanced` no longer skips a code with no name; a name-alone
`Times-Roman`'s code 10 is 0.250 under `StandardEncoding` and `MacRomanEncoding`, Courier's 0.600; a descriptor keeps
ADR 2687's 0. Reported, not changed: ADR 0932 reads an embedded program's never-assigned code as unencoded.

**Item 2.** `cms::fixtures::timestamp_token_naming` carries a certificate and names it by issuer and serial;
`two_timestamps_by_a_unit` lays down round 2057's two-revision document with tokens carrying `TOKEN`'s unit. `Report::of`
on the first timestamp raises clause 9.5's two `OtherKeyOutlived` for the unit (Table 2's PKCS #1 v1.5, Table 6's
2048 bits to 2036); `two_timestamps` raises neither.

**Item 4.** `confined.rs`: a type 1 shading whose program errs for x in (0.1, 0.2), opened at `zoom=400`, crosses as its
window and one fault crosses with the open's events; a scroll's redraw says nothing again. A program erring for x below
a quarter faulted at the interpretation, so the window was not what said it; the narrower range fixed that.

**Plants.** `notdef_advanced`'s empty-name filter restored, the unit dropped from `document_timestamps`, and
`draw_windows` sending `Declined` only when non-empty: each fails its new test; reversed, all pass.

**Gates.** Over `pdf-font`, `pdf-signature`, `viewer-confined`: fmt exit 0; clippy `-D warnings --all-targets` exit 0;
nextest `--lib --bins --tests` under `xvfb-run -a` 1169 of 1169 passed, exit 0; rustdoc exit 0. Conformance after
`git add -N`: exit 101 on this record's gates paragraph, then exit 0. Fuzz not owed: no public type changed.

**Left.** Item 3: a window-level Shift + T test needs `viewer-core`'s policy fixture shared (a feature-gated fixtures
module, its helpers moved from `notes.rs`'s tests), a `NativeWindow` double in `viewer-host`'s tests, and a transport a
window's `Refresher::start` can be given. ADR 0932 against ADR 2948's reading. Any `raster_golden` page whose name-alone
standard font shows an empty-slot code moves; none is known, and the train compares.
