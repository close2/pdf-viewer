# 1100 — the root a host hands over, and the box a matrix shrinks

**Census first (trap 8).** `examples/refused_action_census` gained two columns, for Table 204's `/F`
and for Table 211's `/Base` beside the partial `/URI`s that need one. Over the 1450 curated
documents that open and the 88 890 cached ones: **0 state a `/GoToE` `/F` at all**, against 69
`/GoToE` dictionaries in 9 documents; **64 state a `/Base`** and **1971 a partial `/URI`**, 9 of
those where a `/Base` exists. Calibrated on three planted files, with an `/F`-less `/GoToE` as the
control.

**§12.6.4.4 `partial` → `implemented`.** `/F` is "[t]he root document of the target relative to the
root document of the source", a file on somebody's disk, and principle 3 leaves this process none.
The walk suspends there: `Purpose::TargetRoot` crosses as `Event::NeedsFile` carrying the name the
*document* wrote, and `EmbeddedGoTo::target_from_root` runs the remaining `/T` steps against the
root that came back (ADR 1062's shape, ADR 1079's). Nothing changes across the pause, so it is
*held*: `MAX_TARGET_DEPTH` bounded the path when the action was read, and the root opens under the
source's own `Limits`. Table 204's "Optional if F is present" (EXAMPLE object 5): an empty path
makes the arrived root the target. A host supplying nothing declines **by name** (trap 5).

**§12.6.4.8 stays `partial`, and its own sentence is now false.** It said "the day a host opens a
URL is the day the second caller appears"; that day is this one. A `/F` in §7.11.5's URL form is a
relative URI with somewhere to go, so it is resolved against Table 211's `/Base` — Errata Collection
3 Issue #256 makes that base govern "all relative URIs in a PDF document". What §7.11.2.2 excludes
is *not* resolved, and is checked **before** the resolution, because what it forbids is only
dangerous once something resolves it: `//elsewhere/b.pdf` against a base is a different host.

**§12.7.4.3's scaling edge, bought (ADR 1114).** The clause replaces the *translation* components,
so the rest of the matrix stands and an advance of `w` is `a·w` once drawn; the layout was measuring
a line that was not the one drawn. `Scale` divides the box by the linear part before anything is
measured and multiplies the answers back, so a `/DA` with no `Tm` runs under the identity pair and
every multiplication is by one. **Diagonal and positive only**: a rotation is off this module's one
axis, a skew shears, a negative element mirrors, and `Owed::TransformedTextMatrix` is now those
three and a comb. **Trap 1 paid by eye** at 4×: the scaled field ends at the twin's right edge at
twice the size. Four planted defects each fail the test that should.

fmt and clippy `-D warnings` 0 on this diff; `nextest --workspace` 4957/4957; doc 4; fuzz fmt and clippy
0; conformance 256 + 7 + 11, 0 failures. `raster_golden` **held 974, moved 0**, as predicted — no
corpus document states either construct. `pdf-model --test corpus` 0, every ratchet at its ceiling;
`--test actions` 2 ok; `awkward_classes` killed 0; `viewer-ffi` 18 + 1 + 3 + 2 + 3, with
`QUORRA_EVENT_KIND_COUNT` and `QUORRA_ABI_VERSION` held — a purpose is answered by a call rather
than counted — and `QUORRA_PURPOSE_TARGET_ROOT` the one constant added.
