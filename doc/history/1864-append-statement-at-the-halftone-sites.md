# 1864 — `append-statement`'s page at the graphics family's sites: the three halftone rows

The fourth archive slot of batch 127, in two sessions; ADR 2562, with ADR 2563 and Q730 unused.

**The premise.** The brief named the three halftone sites "and whichever of image smoothing and
overprint mode `state-every-loss` takes"; the profile takes neither (both fall to `default =
"stop"`), so the family is the three halftone rows. The hypothesis held, with one addition: the
profile's `try = ["append-statement", "discard"]` reads only if a page that cannot be composed
hands the object on (RFC 0014 section 3), so at these rows it now does.

**What was built.** `statement.rs` states a `GraphicsEntry` (object, entry, value as the source
writes it, pages whose resources reach it) after the embedded files, the page walk bounded by
`MOST_VISITS`; `prepare.rs` builds the entries from the rewrite's own `halftones` populations;
`config.rs` and `steps.rs` admit the step at `STATED_GRAPHICS`; `mod.rs` hands an uncomposed page
to `discard`; `StatementPage::site` became `sites`.

**The finish line.** A part 4 document holding all three entries converts under `state-every-loss`
with one statement page, each entry read back against the source by a test; a part 2 copy lists
two, HTO being part 4's alone. veraPDF passes the outputs at 4 and 2b and fails both sources and
the part 4 output at 2b. **Plant**: the page made to name `HTP` failed the readback with "the
source's object 9 states no HTP"; restored.

**The probe** found the page walk quadratic: `Pages::get` per index rereads a flat `/Kids`, so
80 000 pages held a conversion past 650 s. It is now `Pages::every` (ADR 1953, ADR 2562 point 3).

**Gates.** `cargo clippy -p pdf-transform --all-targets -D warnings` exit 0; `--lib` exit 0 (178
passed), `statement` after the fix exit 0 (4); `--test profiles` exit 0 (12); `--test archive`
exit 0 (493) in session 1, exit 101 (492, 2 failed) in session 2: the JPX tests at
`tests/archive.rs` 12850 and 12925 at load average 16, passing alone (exit 0, 14). rustfmt clean.
Census in a snapshot of the whole worktree (1861, 1862, 1869 landed): no pack exit 0, 383 s, peak
1.58 GiB; pack exit 0, 263 s, 1.76 GiB. It runs `as-if-printed` and `as-shown`, not
`state-every-loss`, so the four `no-halftone-name` lines are HEAD's byte for byte; the moved lines
are R03a's +1 at pdf.js (1861) and the header's +1 at pdf.js and six corpora refusals (1862).
`rank.py` exit 0. Max RSS by getrusage, this binary against HEAD's, at 4: 1 030 pages of a
500-wide shared dictionary (page composed) 680 900 against 678 268 KiB; 1 045 (past the bound)
687 732 against 688 032 KiB; 80 000 pages 637 096 against 636 440 KiB in 12.4 against 10.9 s;
200 000 pages 1 557 160 against 1 556 628 KiB in 23.9 against 35.6 s; 2 000 pages of a 20 000-wide
dictionary exit 2 in both, the worker out of memory (4 121 256 against 4 121 720 KiB). `--bin
cited` exit 0, `--bin quotations` exit 0, `cargo test -p conformance --no-fail-fast` exit 0 (470
passed); no pixel moves (§10.6.1), so no arms.
