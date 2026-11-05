# 1945 — The operator names their CMYK press once, in their configuration

Session 1945, a single archive round. ADRs 2704, 2705; no question; A784's `Owes` cleared.

**Where "once" lives (ADR 2704).** `[profile] press = "<file.icc>"` in the operator's `--config`
file, a relative path taken from that file's directory; with `base = "as-if-printed"` it is three
lines. Not an XDG file: it would be invisible at the call site (RFC 0007 section 4.7.3), make a run
depend on the account running it, and reach censuses and tests on a developer's machine. A window
offering operator profiles reads the same key (`viewer-host`'s `configured_press`); the confined
route fences the resolved file.

**What it replaces (ADR 2705).** GRACoL only, wherever `press-assumed` is taken: the CMYK intent of
ADR 2701 and part 4's `/DefaultCMYK` of ADR 2518. The assumed inks stay for the other profiles; the
loss stays where the shipped press costs it. Ranking: `--output-intent-profile`, then the
configured press, then GRACoL. The report names it `configured`, "the press your configuration
names", with its `desc` and `cprt`; the intent's sentence and the `/DefaultCMYK`'s history entry
name it as the operator's.

**Validation.** `archive::configured_press`: an ICC header; `prtr` over `CMYK`; versions 2, 4.0,
4.1, 4.3 at part 2 (ISO 19005-2 section 6.2.4.2, PROVENANCE's reading) and 2 to 4.3 at part 4
(§8.6.5.5 Table 66). A refusal names the configuration, the resolved file and the reason; a file
that cannot be read is refused the same way. A configuration naming a press while taking no
`press-assumed` is refused by its line (`PressNeverAssumed`), and no shipped profile names one.

**Tests.** `tests/archive/configured_press.rs` (9): the press with no flag on both routes, the
flag outranking it, the inks kept, the relative path from another working directory, each refusal
by its sentence, report and history naming it, `as-if-printed` with and without it; `press.rs` (2);
the window test gains a press and a missing one. Planting the press ignored fails 3 of 9; planting
it over the assumed inks with the edition check off fails 2.

**Outputs.** `issue9940` and `postscript_type4_many_outputs` under a company profile naming the
assumed-inks profile as its press, at 2b and 4: all four written, the report naming the press and
its copyright, and veraPDF 1.30.3 passes all four.

**Census.** Shipped, CJK pack, against round 1943's: 5105 of 5105 same, no NEW, GONE or CHANGED.

**Gates.** fmt, clippy `-D warnings --all-targets` (`pdf-transform`, `viewer-host`): exit 0.
nextest `pdf-transform`+`pdf-archive` 1412 passed, `viewer-host` archiving 8: exit 0. Archive tier:
`corpus` 1 passed, `archive_corpus` 8 ignored (the census is one): exit 0. veraPDF: 4 of 4.
`cargo test -p conformance` after `git add -N`, `questions` too: exit 0. `state.sh docs`: exit 0.
