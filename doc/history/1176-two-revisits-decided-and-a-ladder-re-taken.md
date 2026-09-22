# 1176 — Two revisits decided, a ladder re-taken, and a gap that prints its own size

**§10.7.5 — ADR 1189.** The clause's first `shall` conditions its own mechanism — "adjusted **as
necessary** to produce lines of uniform thickness" — and the next sentence states the necessity's
measure, half a device pixel from the requested width; §10.7.1's NOTE is what lets a clause in §10.7
bound an outcome rather than prescribe an algorithm. So the grid fit is a mechanism whose antecedent
is unmet, not a `shall` this tree fails; the second `shall`, the sub-half-pixel promotion, is
performed. ADR 0419 section 6's preference — adjustment on when the document has not asked — is
declined on the clause: Table 58's initial value is `false`, the last paragraph gives the control to
the document, and forcing it would take this device from 0.0039 to 0.4000. Still `departed`, because
the clause never defines *thickness*: under a touched-pixel-count reading no anti-aliasing device
meets the uniformity sentence at all.

ADR 0848's phase ladder re-taken on today's `render-cpu`: 128 rungs, two orientations, four widths,
two scales, eight placements. The axis-aligned half reproduces ADR 0848 to the digit, which validates
the harness. The turned half does not — the row's "worst thickness this device produces", **0.1802**,
was measured on `tiny-skia`'s supersampler, which ADR 1082 replaced for every diagonal, and the same
rung now reads **0.0038**. Worst over all 128 is **0.0078** against a permitted 0.5. Nothing reported
the drift: the gate covering that ladder holds axis-aligned rules only, at eight times the recorded
worst. **And the revisit note's third claim did not hold**: ADR 0028 decides two things, and the
colourant premise belongs to the *overprinting* one; the stroke premise is "a device that quantises
coverage to whole pixels, and this is not such a device" — the rasteriser, not the display. There was
no screen premise to expire. Re-grounded on the clause regardless.

**`quorra-confined` — ADR 1190.** ADR 0713 Decision 2's premise is contradicted by ADRs 0718, 0725
and 0737. The conclusion keeps a new ground: levelness is an instrument needing three
*interchangeable* hosts, and this window holds no `Viewer`, no bytes and no parser. **A host owes the
control for an operation exactly when it can perform it** — today none, and `doc/todo/30` carries
what arrives with each future feature. The confinement is no excuse for a missing control: the
*worker* has no filesystem, the host opens the document itself.

**The instrument.** `--remedy-sites` prints `N of M sites not built yet` from the site predicate,
agreeing exactly with the line count it replaces, and with `--config` the answers a profile does not
carry out. `Configuration::unbuilt` is a function of the profile and the target, so the profile half
needs no corpus — a deliberate departure from the brief's "behind the lock", which assumed a walk
that does not exist.

Owed: clippy on `pdf-transform` never ran (blocked 15 minutes by `pdf-model/src/view.rs`), and
`render-cpu/tests/stroke_width.rs` wants a turned rung and a tolerance under 0.05.
