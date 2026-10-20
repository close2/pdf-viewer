# 1633 — The codecs come from the fork, and the extended template decodes

Session 1633, slot 3 of batch ninety-nine, a clause round; 2026-10-20. ADR 2102 (2103 unused).
The owner pushed `quorra/t88-patches` to `close2/hayro` and answered Q514 "Push it."
(`doc/questions/A514`); ADR 2074 section 4's steps were this round's.

**Built.** The `hayro-jbig2`, `hayro-jpeg2000` and `hayro-ccitt` stanzas name the fork at
`7e39af41`; both locks moved those three packages' source and nothing else; `deny.toml`'s
`allow-git` names the fork in upstream's place; the three `doc/patches/hayro-jbig2-*.patch` files,
`decode::extended_template_region` and its refusal are deleted. Its test is now a decode: the
file's own template-0 region recoded on the extended template at Table 5's nominal locations,
held to the ordinary bitmap, which T.88 6.2.5.7 makes the same; planted back on upstream's codec it
fails. Annex K item 7's generic region is held as `Matches`. The fork was read before it was
pinned: three commits under `hayro-jbig2/` only, each equal to its patch line for line, and its
Annex H.2 unit test passes in an unpacked copy.

**Rows.** §7.4.7 `partial` → `implemented`. §7.4 stays `partial`, derived, on §7.4.9.

**The premise that did not hold.** Two: §7.4 does not move, because §7.4.9 owes, and the ledger has
no §7 row; and the patched codec does not refuse the seven symbol dictionaries "at once" — the two
symbol-pixel inputs take about 0.65 s, past the 250 ms deadline the tests gave them. They are held
to the codec's sentence under a 30 s deadline now, and the confined kill, which only they reached,
has an honest slow region of its own (ADR 2102 section 4). The hypothesis's other halves held: the
lock moved three packages, and no corpus digest moved.

**Census.** The arms' 104 JBIG2 documents hold 110 streams, none with EXTTEMPLATE on template 0;
the fuzz seeds' 8 701 none either; the walk names a planted one.

**Gates.** `cargo fmt --check -p pdf-sandbox`, `rustfmt --check` on the five files and `RUSTFLAGS=-D
warnings cargo clippy -p pdf-sandbox --all-targets`: 0. `cargo nextest run -p pdf-sandbox`: 51 of 51
passed; `t88_conformance` with the data: 0. `cargo deny check`: 0, advisories, bans, licenses and
sources ok. The six arms of the export against `/home/AI/arms-1631/` by page digest: each exit 0,
968 and 965 pages, 0 moved, the 104 JBIG2 pages among them. `cargo test -p conformance
--no-fail-fast`: every binary green but `records`, on sibling record 1634. The JBIG2 fuzz target,
600 s unsanitised under `--long`: 0, 24 167 runs, `cov` 3 383 to 4 702, no crash and no timeout (the
sanitised build under `tools/fuzz.sh` could not map its shadow memory inside the bound's
`RLIMIT_DATA`, as `doc/verify.md` says). `t3-foreign_corpus` under `--clock` in the export: 0, in
188 s. `tools/state.sh docs`: 101, on `render-cpu`'s rustdoc, a sibling's crate. Duration: about 3
500 s.
