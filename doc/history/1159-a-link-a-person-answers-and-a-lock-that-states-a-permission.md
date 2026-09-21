# 1159 — A link a person answers, and a lock that states a permission

Host-UI round of batch twenty-four. ADRs [1155](../adr/1155-a-link-is-a-program-this-machine-starts.md), [1156](../adr/1156-the-permission-a-lock-states-over-the-document.md).

## §12.6.4.8 — the act, at the reader's level

**Measured first (trap 8).** `may_open_uri` answered every call with an `Err`: no window opened a
link, and the refusal sat where the operation is — the shape `CLAUDE.md` principle 3 names. **Built.** `viewer_host::Links` — `refuse`, `ask`, `warn`, `open` — `--links=` in all three windows,
`ask` the default because it is the only level under which nothing reaches another program without a
keypress *and* nobody must configure the viewer first. Not `RESTRICTIONS`'s words: there `off` is
the permissive end and here it would be `open`. `may_open_uri` decides, `open_uri` spawns
`xdg-open` and waits on a thread of its own, `link`/`answered` carry the level out for four faces,
and the prompt is `restriction::Question`'s shape in ADR 1145's dialogue. **Two refusals come before
the level**, neither a thing *open* turns on: a still-relative reference, and any scheme outside
`http`, `https`, `mailto` — `file` among them, because `read_import` answers that hazard with a
directory a *person* supplied. `quorra-confined` is pinned to `refuse`: a window that cannot ask
must not open. **No `Command`, `Event` or `Query` was added.**

## §12.7.5.5 — Table 236's `/P`

The owner's revisit note on ADR 0502 §3.1 was checked claim by claim and **all three held**; the
deciding one is that `RestrictionPolicy::default` is `off` for all six operations, so the "default
refusal on 28 real documents" the deferral was priced against no longer exists. The entry settles
its own two voices: "[t]he access permissions granted for this document", reaching "any incremental
changes to the document following the signature of which this key is part" — §7.5.6's update.
`field_lock_permissions` reads it on `field_locks`' signed condition; `Restriction::LockPermission`
sits beside `Certified` rather than `FieldLocked`; several compose as a minimum, which the entry's
own *less than or equal … otherwise ignored* makes order-independent. Errata #131 changes no
verdict here and is evidence *for* the reading.

**Calibrated (trap 13), four plants, each restored and re-run green**: the `/P` reader answering
`None`, and answering a level unconditionally, each fail the new test; `may_open_uri` skipping the
scheme check, and ignoring the level, each fail one of the two new host tests.

**Gates**: fmt, clippy and `nextest` over the eight crates touched — 0 findings in this round's
files, 2 572 tests green; `conformance` green; tier 2 `save_round_trip` green, every ratchet held.
**`raster_golden` fails and not on this round's account**: 966 of 974 first pages moved "list only",
a neighbour's in-flight `BlendMode::Overprint` in `pdf-render`/`render-cpu`/`pdf-model/src/content`.
Ledger: §12.6.4.8 and §12.7.5.5 both `partial` → `implemented`. `doc/questions/Q67` asks the owner
the one value a round should not settle alone: whether `ask` or `refuse` is the right default.
