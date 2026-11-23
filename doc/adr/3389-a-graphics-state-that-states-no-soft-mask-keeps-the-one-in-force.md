# 3389 — A graphics state that states no `/SMask` keeps the soft mask in force

Session 2172. Status: accepted and built (`pdf-model`: `soft_mask.rs`'s `SoftMaskEntry::Unsaid`,
`content/ext_gstate.rs`). Answers finding 2171-B01 of `doc/reviews/2171-specification-audit-11.md`.

## The clauses

§8.4.5: "All entries need not be present for every invocation of the gs operator … The results of
gs shall be cumulative; parameter values established in previous invocations persist until
explicitly overridden." §11.6.4.3 gives the removal of a mask to one value: "The name None may be
specified in place of a soft-mask dictionary, denoting the absence of a soft mask. It shall also
mean that any existing mask shall be removed from the current graphics state."

## Decision

`entry_with_output_intent` answered an absent `/SMask` (read as `Null`) with `SoftMaskEntry::None`,
and `apply_ext_gstate` cleared `state.soft_mask` for it, so `/M gs /A gs` drew unmasked wherever `/A`
stated only `/ca`. The entry now has a third answer, `Unsaid`, for an absent key or a `null` value
(§7.3.9 makes the two the same), and the state is left alone. `/None` clears; an unusable value
still clears and is reported, as before.

The audit's habit — "an absent entry is a third answer beside its two values" — was asked of every
other entry `apply_ext_gstate` reads. None reads absence as a value: `/ca`, `/CA`, `/LW`, `/LC`,
`/LJ`, `/ML`, `/SM`, `/OPM` take a number or integer, `/SA`, `/OP`, `/op`, `/TK`, `/AIS` a boolean,
`/D`, `/Font` an array, `/RI`, `/UseBlackPtComp` a name, `/BM` a name or array, each only where it
is present; `/TR`/`/TR2` have `Stated::Unsaid`, `/BG`/`/UCR` `Said::Unsaid`, and `/HT` returns
`None` for `Null`.

## Cost

A page that sets a mask and then a second state without `/SMask` now draws masked where it drew
unmasked. That is the specification's picture; any golden page carrying the old one moves.
