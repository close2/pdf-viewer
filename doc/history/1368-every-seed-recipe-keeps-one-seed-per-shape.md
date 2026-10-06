# 1368: every seed recipe keeps one seed per shape, and the oracle holds no page as ours

This was the robustness slot of batch fifty-seven. It wrote ADRs 1571 and 1572 and moved no
ledger row. It wrote no question.

**Seeders (ADR 1571).** Every recipe that reads the 126 GB of `documents` now keeps the smallest
seed of each shape its target branches on. The shape is stated per target, with its reason, in
`seed_page.py`, `seed_streams.py`, `seed_der.py` and the new `seed_codecs.py`; `seed_shape.py` is
new and holds the selection and the memory map. A document is read only when its map holds the
name its target needs. `object` now takes object bodies, because its target stops at a whole
document's first `obj`. `--every` writes the population whole. Each shape was refined against
the old recipe over 300 documents, by `INITED cov`, until the loss fell inside the 2% margin. On
those samples `jpx` went from 20 506 seeds and 400 MB to 353 seeds and 8 MB, and `ccitt` from
142 MB to 17 MB. `jbig2` is 2.4% short, because its decoder's coverage follows its data. The
four whole-document targets still come to about 1 GB each, which is what holding the margin
costs. `check` now builds `-s none`, because AddressSanitizer's shadow map is refused under
`tools/bounded.sh`.

**The thirteen, and `object`.** These are stale: `cmap` (368 against 435), `crypt` (837 against
1164), `variable_text` (6594 against 6734) and `object` (462 against 506). `jbig2`, `jpx` and
`linearize` are stale because they have no corpus on disk. `sfnt`, `xmp`, `ccitt`, `revocation`,
`x509`, `confined_wire` and `cms` are current.

**`issue4436r.pdf` (ADR 1572).** Against today's references the clause's own form, with row 25
unpainted, differs from `poppler` in 8.06% of channels, the same as ours. The bound is 7.34%. Only
the references' black row clears it, at 6.71%. So `CONTRADICTED_SUBPIXEL_IMAGE` is now
`Whose::References`, `held` prints 0 / 14 / 33, and no contradicted page is held as ours. The
note now calls the mask an inline image (§8.9.7). No pixel moved.

**Triage.** `tools/state.sh fuzz` printed the same before and after. Both crashes have their
tests and exit 0, and neither is Q227's. The 70 `page` slow units all exit 0 and fall into six
shapes. One of them is a quadratic in `pdf-font`'s `post_glyph`, and no budget governs it. It is
priced in `doc/todo/49` and left to that crate to fix.

**Gates.** rustfmt on `oracle.rs` 0; `bash -n` and `py_compile` 0. `RUSTFLAGS=-D warnings` clippy
`-p pdf-model --all-targets` 0. `cargo nextest run -p pdf-model` 0, 1785 passed.
`cargo test -p conformance` 0, 382 passed. The fourteen `seeds.sh check`s ran one at a time
behind the lock, from 69 s to 1459 s each.
