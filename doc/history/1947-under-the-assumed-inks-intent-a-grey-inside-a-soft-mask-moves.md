# 1947 — Under the assumed-inks intent, a grey inside a soft mask moves

Session 1947, a single archive round. ADR 2708; Q788.

**The premise, checked.** The brief, after ADR 2700 section 3, blamed the blending space a CMYK
intent gives a page naming none. Five of the six pages state `/Group /CS /DeviceCMYK` and the sixth
states no group, and `transparency::page_press` composites a page with no `/CS` on the device's
three components with or without an intent. The premise did not hold.

**The cause, measured** (`r1947/exp/six.sh`). Each output's `/DeviceCMYK` groups were stated as the
intent's own `ICCBased` profile, with the intent renamed. Together they draw the output exactly,
and one at a time they find the carriers, every one inside a soft mask:
- `DeviceGray` images, fills or shadings in a `/DeviceCMYK` `/Luminosity` group: `bug1755507` (25),
  `issue9017_reduced` (25), `issue18032` (22), `issue12798_page1_reduced` (16);
- an isolated `/DeviceCMYK` group of black in a `/DeviceGray` mask group: `bug1703683_page2_reduced`
  (23), `issue13520` (9). Every other group, page groups included, moves at most one level.

**The reading.** §11.5.3's luminosity is not what moves: the reader takes its device branch in every
space (A23, ADR 2552). What moves is the conversion between the grey and the CMYK. The source takes
§10.4.2.3's black alone. The archive's group is the intent's profile, and §11.7.2 leaves a device
colour of another count "implementationdependent", so this reader separates it by `B2A` (ADR 0796).
ISO 19005-2 and -4 section 6.2.4.3 fix a grey under a CMYK intent as the classic conversion, which
draws the six as their sources and moves every plain grey of such a file. Hence neither (b) (no
statement keeps both) nor (c) (ISO 19005 removes the difference). (a) is the owner's: Q788
recommends the rule plus a `/DefaultGray` in the converter, after a measurement. The renderer is
unchanged; the report's sentence, which blamed the page's blending, now names the masks; section 8
of `doc/pdfa-blocking-issues.md` has the row.

**Tests.** `drawn_through_the_intent` gains 3. A mixed ink masks by EXAMPLE 2 in the source and
the archive, and a `DeviceGray` 0.5 masks by 0.5 in the source and differently in the archive.
A `/DeviceCMYK` page group with `Multiply` draws within a level. Plants: GRACoL in place of the
assumed inks fails 2 of the 3; a mask group that ignores the intent fails the two mask tests.

**Census and veraPDF.** Not run: the corrected sentence reaches only the report, so no output changes.

**Gates.** fmt, clippy `-D warnings --all-targets` (`CARGO_INCREMENTAL=0`), `-p pdf-transform`: exit 0.
nextest `-p pdf-transform`: 1087 passed, exit 0. Archive tier (`corpus`, `archive_corpus`): exit 0.
`cargo doc -D warnings -p pdf-transform`: exit 0; `state.sh docs`: exit 0 (rerun). `cargo test -p conformance`
after `git add -N`: exit 0. A first pass hit a clippy ICE, five `SIGSEGV`s in `actions.rs` and a rustdoc
ICE in `raster-compare`; none reproduced under `CARGO_INCREMENTAL=0`, and the cause is not traced.
