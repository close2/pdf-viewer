# 990 — A salvage with one rule, and the save that had held for five hundred rounds

Date: 2026-09-12. ADR: 1011. Written at the merge (session 991's) from the ADR and the round's own
gate notes: the session limit cut the round off after its last walk and before this file.

Files: `crates/pdf-syntax/src/lexer.rs`, `crates/pdf-model/tests/numbers_without_digits.rs`,
`crates/pdf-model/tests/save_round_trip.rs`, `tools/state.sh` (`save` section),
`tools/conformance/tests/state_sections.rs` (new), `doc/todo/05` item 2, `doc/conformance/ledger.toml`
(§7.3.3, §7.5.6), `doc/adr/1011`.

Two leads from the direction review's first attempt, both principle-level. **The lexer's
`salvage_number`** carried, as its reason for reading `--5` as −5, that "both Acrobat and pdf.js read
it as −5" — a reference implementation named as the reason for a behaviour, which is principle 5
inverted. Read arm by arm, the salvage has one rule, and it is the ledger's own for §7.3.3: *toward
reading files that exist*. Each arm now carries that reason, as a documented choice about input the
standard does not describe, and the two consecutive sentences that contradicted each other ("is not
ignored" / "terminates it") are one sentence.

**The save round-trip** — §7.5.6's incremental update over the corpus, read back by this tree,
poppler and mupdf — was `#[ignore]`d and in no gate line for four hundred and ninety-one sessions,
on `doc/todo/05`'s standing rule that an instrument's numbers enter §2 only after they hold. They
hold: the counts ratchet on the file's own promise (935 saved, 933 free texts, 80 fields, 80
fillable, 8 and 2 under `Off`), every shrinking population is a set of names held both ways, the
ratchet runs over the 974 tracked documents alone (ADR 0970's rule), and it costs 12.5 s on a loaded
machine with no clock judged. Calibrated twice by breaking it. `tools/state.sh save` runs it, and
the merge added its line to §2 after `xmp`, where the script already had it.

**And the third question answered on the way**: `doc/todo/02` §2 and `tools/state.sh` are two
hand-written copies of the gate sequence, and nothing compared them. `state_sections.rs` derives
both lists and fails on a §2 line the script does not run; a script section §2 does not list is
printed, because an instrument may honestly run ahead of the sequence. Its first parser scraped
every line naming `cargo test`, took the map's table cell mentioning `--test gate` for a command,
and named a gate the script runs as one it does not — the merge restricted it to fenced blocks.

Gates, as the round reported them line by line while they ran: the core through `nextest`
green on its own files (the workspace lint red on two neighbours' mid-edit files); corpus, oracle
(990 agree / 62 contradicted / 835 ambiguous, unchanged), text extraction, both censuses, dates,
xmp, the save round-trip, jpeg2000, fixed documents, the transform gate and its six writer walks,
both vfs walks — all exit 0. The quorra corpus gate's one differing name, `issue18032.pdf` leaving
the refused list, was session 988's construction and not this round's, attributed by the display
list being byte-identical with this round's patch alone.
