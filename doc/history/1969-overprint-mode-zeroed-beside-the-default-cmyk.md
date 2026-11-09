# 1969 — Overprint mode 1 under a `DeviceCMYK` mark is written 0 beside the `/DefaultCMYK`, as the named loss

Session 1969, a single round, PDF/A lane's worktree. R05's last nine (ADR 2749 built as ADR 2750).

**The premise.** Two of three. Each of the nine stops at part 4 on `OVERPRINT_MODE_ONE` under every
profile; under `as-if-printed` it is each one's only refusal, under `as-shown` it is the only one
of six (`363_Risk…`, `E3_Mayhew_et_al_2009`, `me450f12project10_report` also stop at R09's fonts).
But two blend their governed marks in a `DeviceCMYK` group, where ADR 1978 refuses by name (each
converts with the group's space written `DeviceRGB`, the control), and `digitally_signed_3D_Portfolio`
then fails the row in the output's own verdict, as it already did at 2b (a `D-defect`).

**Built (ADR 2750).** `overprint::under_a_default` names the states setting the mode for each
`DeviceCMYK` mark overprinting under it (objects stating `/OPM`, a whole walk, no four-component
blending), apart from those the overprint row already zeroes; `supplied_cmyk::prepare` takes them
only at part 4 and only where `overprint-mode` is authorised, and refuses otherwise as before. The
`DeviceCMYK` row's decision carries `Loss::OverprintMode` beside the press's own; its `CONDITIONAL`
entry does not list it, so no `discard` there authorises it. History: `default_history`, one entry.

**Freed** at 4: `as-if-printed` `E3`, `LLMImplicationsforScience`, `NeilJairath_3`, `carp2010-2`,
`freeculture`, `me450…`; `as-shown` `LLM…`, `NeilJairath_3`, `carp2010-2`, `freeculture`.
**Pixels** (CPU, 72 dpi, 1 080 pages): each output with its `/OPM 0` written 1 again draws
identically (difference 0 everywhere). Against the source with `/OutputIntents` renamed: `as-shown`
at most 1 (ADR 2700's bound); `as-if-printed` up to 50 through the shipped press (ADR 2518), and
`me450…` 185 through its embedded machine faces (R09).

**Tests** (`archive/overprint_beside_a_default.rs`, four): both shipped profiles write the default
with the state zeroed, the decision's losses and the history entry; `no-data-loss` keeps the
refusal; a mark in a `DeviceCMYK` page group keeps it; part 2 keeps the `DeviceN` default and the
mode beside a supplied press. Plants: no `/OPM 0` written, no loss carried, no history entry each
fail the first; no authorisation check fails the second; no four-component check the third; no
part 4 check the fourth.

**Census** (against round 1968): shipped 10 NEW (the ten above), 0 GONE, 0 CHANGED; stating 0 NEW,
0 GONE, 0 CHANGED (no stating profile accepts the loss). R05 is 2 at 4; `D-defect` 7 / 7 under
`as-shown`. veraPDF 1.30.3 passes the ten NEW census outputs and the ten CLI outputs.

**Gates.** fmt 0 and clippy `-D warnings --all-targets` 0 on pdf-transform; nextest pdf-transform
and pdf-archive 0 (1447 passed); archive tier 0; both censuses 0; `cargo test -p conformance` 0
after `git add -N`; `tools/state.sh --round 1969 docs` 0.
