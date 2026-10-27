# 1788 — The seventh sweep reads the tables a clause names, and §13.7.2.3.1 is implemented for its assets

Session 1788, slot 1 of batch 119, the ledger slot (ADR 2370); ADR 2410; ADR 2411 unused; Q654 unused.
I worked in the shared worktree and in my scratch directory, `scratchpad/r1788/`.

**Instrument.** `--bin inapplicable` gives each swept row the terms of every table its own clause
names in `doc/md/` — captioned, cited by number, or named by its caption's subject as a dictionary —
keyed on the row's number (trap 272): `Table N` as a whole designation, not one a `32000-1` stands
before, and the caption's subject, two words or more, as a folded phrase. `--root <tree>` reads a
past tree's ledger, sources and `doc/md/`.

**Calibration.** On the commit before ADR 2376's merge, unpacked by `git archive`, the sweep reads
118 rows, 31 cited (ADR 2376's figures), and lists §13.2.4.1, §13.2.6.1 and §13.2.7.4.2 each by a
caption term of reach 1 (`Table 284`, `Table 294`, `software identifier`, all `media.rs`), with
§13.2.7.2, §13.2.7.2.1, §13.2.7.3 and §13.2.7.4.1 beside them. The planted unit test is those three.

**Rows.** §13.7.2.3.1 from `out-of-scope` to `implemented`: Table 341's /Assets, "[a] name tree of
embedded file specification dictionaries as detailed in 7.11.3", is walked by `rich_media::assets`,
and /Configurations and /Views stay on the clause 13 entry; test
`table_341s_assets_are_read_and_its_configurations_and_views_give_no_file`. §13.7.2.3's and §13.7.2's
notes name the exception, status unchanged. The rest of the list, and why each stays, is ADR 2410's
table; `doc/todo/65` holds no row that moved.

**The premise that did not hold.** The three calibration rows are `implemented` on HEAD (ADR 2376
moved them), so HEAD cannot list them; the calibration is the past tree. The hypothesis (a dozen
rows, two or three more moving) did not hold: HEAD lists 21 rows by a caption term of reach five
or less, and one moves. Two of the list are miscitations, reported: `pdf-transform/src/archive/
prepare.rs`'s "§14.11.5's Table 400" (Table 401 is the output intent's) and `pdf-font/src/sfnt.rs`'s
"§9.9 Table 126" (spot functions).

**Gates.** `cargo fmt --check -p conformance` exit 0; `rustfmt --check` on `rich_media.rs` exit 0;
`cargo clippy -p conformance --all-targets` (`-D warnings`) exit 0; `-p pdf-model --all-targets` exit
101 on the first run (49 errors in a sibling's `xfa/` files), exit 0 on the rerun; `cargo test -p
conformance --lib inapplicable` 15 passed; `cargo test -p pdf-model --lib rich_media` 4 passed;
`--bin inapplicable`, `--bin cited`, `--bin quotations` exit 0; `tools/state.sh --round 1788 ledger`
exit 0; `cargo test -p viewer-core --test rich_media_assets` 2 passed; `cargo test -p conformance
--no-fail-fast` exit 0, 69 test binaries passing.
