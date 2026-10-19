# 1569 — A form's font is loaded once for its layout and its `Tf`, and a script not run is said once

Slot 6 of batch ninety, 2026-10-15, a script round. `doc/todo/70` items 1, 2 and 5; ADRs 1974, 1975;
no question; no ledger row moves (§12.7.4.3, §12.6.3 and §12.6.4.17 gain the new tests).

**Premise.** Held for items 1 and 2: the interpreter's `Tf` loaded the `/DR` font the layout had
loaded (6 `LoadedFont::load` calls at three ticks of the framebuffer witness), and a host decided
Tier 0's sentence from `ScriptsRan::handed` alone. Not for item 3: none of ADR 1957 section 2's
eight property rows is a method in Adobe's reference (ADR 1975 section 2), so nothing moved; the
XFA page's own `typeof xfa == "object"` example, which throws here, is left in `doc/todo/70`.
**Hypothesis.** Held: no pixel moved, the loads halved (6 → 3), and the double sentence is one field.

**Built.** (2) `LayoutFonts::sharing`, the interpreter's: a `/DR` font named by reference is asked
of and kept in the interpreter's `FontCache`, so the appearance's `Tf` finds it, and an open
document's cache answers later interpretations without a load (ADR 1974). Instructions 1 085.3 M →
1 079.6 M (−0.53 %): a non-embedded Courier loads in about 1.9 M. (1) `ScriptsRan::said`, counted by
`report_unread` and `before_open`, and `ScriptsRan::answered`, on which the three host sites drop the
action path's refusal (ADR 1975). The two new test files fail with each change put back (trap 13).

**Measured.** Callgrind over `script_cost --ticks 3`, HEAD's export against this round's, outside
the clock hold. The clock hold: `turn_path` alone, load 1.5–1.6, 92 s, the round's scratch.

**Files.** `crates/pdf-model/src/{variable_text.rs,content.rs,content/font.rs}`,
`crates/pdf-model/src/view/{script_sites,script_timers}.rs`, `crates/pdf-model/tests/
form_font_once.rs`; `crates/viewer-core/src/{viewer,interact}.rs` (one expression each site),
`crates/viewer-core/tests/script_said_once.rs`; ADRs 1974, 1975; `doc/todo/{70,README}.md`,
`doc/state-of-play.md`, `doc/conformance/ledger.toml`; this record.

**Gates.** rustfmt clean on all nine Rust files. clippy `-D warnings` `-p pdf-model -p viewer-core
--all-targets` exit 0. nextest `-p pdf-model -p viewer-core` exit 0, 2 501 passed, 23 skipped. Six
arms of this round's export against HEAD's arms: 0 of 5 799 page digests moved, every arm
`test result: ok`. t2 `pdf-model --test script_corpus` exit 0 (356 held, 0 moved); t2 `pdf-script
--features engine --test script_corpus` exit 0, 3 passed, 0 over a budget. `turn_path` exit 0, 33
of 33 rows inside their bands. `cargo test -p conformance` exit 0. `tools/state.sh docs` exit 0.
Drive: `quorra` under Xvfb on doom.pdf at `--scripts on` says the page's 6 415 756-byte script once,
with the bound, and no Tier 0 sentence; at `off` only Tier 0's; the framebuffer witness draws its
200 rows. The toolkit windows were not driven: no host code of theirs changed.
