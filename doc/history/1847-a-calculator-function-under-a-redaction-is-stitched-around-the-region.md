# 1847 — A calculator function under a redaction is stitched around the region

Slot 8 of batch one hundred and twenty-five, 2026-10-28, a clause round (Sonnet, high); ADRs 2528 and 2529 used, Q713
used; all numbers of mine used. I worked in `scratchpad/r1847/` and in my own export built there with its sandbox worker
(habit 185). Ledger: §12.5.6.23 stays `departed` on two refusals, the JPX family and the two-input calculator (its note and
`doc/todo/65` say so); §12.8.3.4.4 stays `partial` and its note names the block as outside the tree.
**(b) Premise** ("a one-input calculator function is cut by a §7.10.4 stitching function whose `/Bounds` cut the
region's interval out, one dictionary written where the refusal is returned"): held for the construction, not for its
size or its exactness. `build_calculator` writes the stitching dictionary, the producer's calculator object named by
its reference in the pieces outside the spans and ADR 1124's constant (a Type 2 of zeros, carrying the program's
`/Range`) inside them; a calculator under another stitching function is reached through the outer `/Encode` already
mapped. Not one dictionary: it needs bounds (1024 pieces, 64 outputs, both with a test at the bound), a plan that
survives a reader narrowing each bound to `f32` (`stitch_plan`, three unit tests), and §7.10.4's half-open pieces
destroy a span's left-end parameter too (ADR 2528 states all three). A two-input calculator keeps a refusal of its own
sentence. **The row's other refusal did not go with it:** the note named the `JPXDecode` decoder bound (past 24 bits)
and also a deep raw codestream of two, five or more components with no colour box (`images.rs`, `deep.rs`), which
this slot's brief did not name and which stays.
**(a) §12.8.3.4.4**: RFC 3125 (ASN.1, Experimental) is free; ETSI TS 119 172-1 and TR 102 272 are refused to this
client with HTTP 403, as is EN 319 122-2 that ADR 1085 fetched; no spoofing tried. Nothing built (ADR 2529), Q713 asks
the owner to place the documents.
**Evidence:** eight integration tests in `tests/redact.rs` evaluate the written function with the viewer's own reader
(destroyed inside the span, the producer's value outside it, both reversed and at two spans); planting
either the piece's `/Encode` or the constant wrong fails five of the eight. The pixels outside the region differ from the original's by
at most one level, `held_outside_within(.., 1)`, from the reader's single-precision interpolation (ADR 2528).
**Memory probe:** the 511-placement, 64-output test, 34 MiB resident (debug); the refusing test 13 MiB.
**Found, not fixed:** the shared worktree's `pdf-colour` did not compile for a while (a neighbour mid-edit, habit 138),
so my gates ran in my export. `/usr/bin/time` is absent on this machine (the probe used `ru_maxrss` from Python's
`resource`). `cited` lists six front-matter pairs in files not mine.
**Validator FAIL, fixed (three findings):** the tree written over calculators is counted against the viewer's function
reader's budget (`build_function_tree`; an outer stitching of k pieces naming one calculator had multiplied the per-
calculator bound by k: the probe at k = 200 reached 429 MiB and wrote a tree the reader refuses, now refused by name at
76 MiB; tests at 2047 and 2048 pieces, the first read back with the reader); the row and `doc/todo/65` count the
two-input calculator as the second refusal; a test comment names the reader's `MAX_VALUES` and not Table 38; the ledger's
test list gains the four tests it omitted; `stitch_plan` names Table 41's last-piece exception. `MAX_FUNCTIONS` and
`MAX_VALUES` are made `pub` in `pdf-colour`.
**Gates.** On my export, `cargo test -p pdf-transform --test redact` 111 passed and `--lib` 139 passed, exit 0;
`cargo fmt --check -p pdf-transform` exit 0; `RUSTFLAGS="-D warnings" cargo clippy -p pdf-transform --all-targets`
exit 0; `RUSTDOCFLAGS="-D warnings" cargo doc -p pdf-transform --no-deps` exit 0; `cargo run -p conformance --bin cited`
exit 0 (six pre-existing front-matter pairs, none mine); `--bin quotations` exit 0 (no divergence in a file of mine);
`cargo test -p conformance --no-fail-fast` exit 101 with three reds, none mine (pdf-syntax's `cross_references.rs:1696` quotation, §11.7.5.3 naming ADR 2532 which has no file yet, an ignored pdf-model test file in no gate line) after one of mine (a ledger test name I had replaced) was fixed; `rustdoc -D warnings --document-private-items -p pdf-transform` exit 0; `tools/state.sh --round 1847 docs` queued 16 minutes for the large lane behind other rounds and was stopped by TERM, so the workspace-wide rustdoc was not run. **Validation.** An Opus validator failed the first reading on three findings — the rewritten function tree had no total bound under an outer stitching function (429 MiB and an output the viewer's reader refused), the second departure was uncounted, and a sixty-four was attributed to Table 38 — and passed the round after it fixed all three, its probe re-run refused by name at 73 MiB.
