# 1150 — the line's direction, and the comb that was never an exception

2026-09-16. ADR 1130. Contract: §12.7.4.3's `/DA` `Tm` under rotation, skew and mirror; the comb; `/MK` `/R`.

**Census first (trap 8), and in the right instrument.** §12.5.6.6 sends a free text annotation's
`/DA` here too, so the eight-class classification went into `examples/variable_text_census`, whose
population is already every object this clause lays text out for, rather than `field_flag_census`,
whose population is fields. **Curated 1452 documents: 1370 objects state a `/DA`, 3 state a `Tm`,
all three the identity. Crawl 65 720: 41 488 state a `/DA`, 3 state a `Tm`, all three the identity.**
No witness anywhere for a scale, mirror, quarter turn, shear, off-axis turn or singular part.
`field_flag_census` keeps the two questions that are a field's: **97 curated and 1074 crawled comb
widgets, each with the `/MaxLen` bit 25's own condition requires**, and Table 192's `/R` — **33
curated and 953 crawled widgets turned**. Calibrated on ten planted files, one per class plus a comb
and a turned widget, against an identity twin and a `Tm`-less free text (trap 13).

**§12.7.4.3, built (ADR 1130).** ADR 1114's `Scale` becomes `Frame` over a general 2×2, and the rule
is *where the linear part sends the line*: `b` zero leaves it on the appearance's x-axis, `a` zero on
the y. A **scale**, a **mirror**, a **half turn**, a **quarter turn** and a **shear** are all carried
out by the one construction — `shrink` carries the box back, `place` carries each position forward,
`UPRIGHT` is what a `/DA` with no `Tm` runs under. A shear's whole effect is that the line's origin
slides with the baseline (`Frame::drift`): the advances are untouched and the glyphs lean. **The comb
was never an exception** — bit 25 decides where a character sits, §12.7.4.3 what space it sits in —
so the cells are divided out in the space the matrix maps from and each cell's `Tm` carries the
`/DA`'s linear part. `Owed::TransformedTextMatrix` is one case instead of four: a line off *both* box
axes, or a singular part, neither leaving a length the box states to measure a line against; the
matrix still reaches the stream there (`Frame::unmeasured`), because dropping it would depart from
the clause further than the mispositioning the report names. **`LaidOut::selection` answers four
corners**, a sheared rectangle being a parallelogram. **Table 192's `/R` was already built** and the
brief was stale: `appearance::Rotation` has read and applied it since the hundred-and-fifth session,
and the entry is in Table **192**, not 230.

**Trap 1 paid by eye at 4×**: the quarter turn reads bottom-to-top and ends at the top of the box
under `/Q 2`; the mirror puts the right-quadded line at the *left* edge, each glyph reversed; the
left-quadded shear starts at the box's left edge and leans; the comb's six cells are the same six at
twice the glyph; the 45° turn draws on the diagonal. Six planted defects each fail one test, by name.

Gates. rustfmt --check 0 and `batch.sh check` fmt clean; clippy -p pdf-model -D warnings 0 in mine,
2 siblings'; nextest -p pdf-model 1425 passed 16 skipped; --test variable_text 72; --doc 0;
conformance 5 passed 2 failed, every named site a sibling's. Under the lock: --test corpus ok with
every ratchet at its ceiling, raster_golden **held 974 moved 0**, oracle 3 passed.
