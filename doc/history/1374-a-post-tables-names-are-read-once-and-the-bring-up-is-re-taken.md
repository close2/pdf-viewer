# 1374: a `post` table's names are read once, and the bring-up is re-taken on a quiet machine

This was the performance slot of batch fifty-eight. It wrote ADRs 1584 and 1585 and the question
Q270. It moved no ledger row.

**`post` (ADR 1584).** `pdf_font`'s new `PostNames` resolves a face's format 2.0 names in one pass
over the strings and inverts them once into a randomly keyed `HashMap`. It serves §9.6.5.4's
lookup and §9.10.2's readback. Round 1368's three units are `0c5ebcab…`, `2b9f6ddf…` and
`e788c7ce…`, not the ones its size suggested. Page 1 of the first is 14 487.3 → 131.6 M
instructions. `issue14297.pdf`'s font load is 338.0 → 23.0 M and `bug1727053.pdf`'s 72.2 → 22.9 M.
`S2.pdf` pays 1.2 M more, stated in the ADR. `tests/post_table_names.rs` runs at 5125 and 65 535
glyphs. The old code took 6.98 s on the smaller case and the new code 0.02 s on both.

**Audit.** Callgrind over 58 pdf.js pages, counted first. `/Differences` names were searched for
in about 1400 static strings each, 28% of `issue4550.pdf`'s page, and are now owned:
`encoding_names` is 1.28 → 0.11 M. The `cmap` 4 search, `loca` and `hmtx` have no such shape. The
CFF encoding's charset walk is linear and was left. `/W` expansion has no total bound and is
priced in `doc/todo/49`.

**Bring-up (ADR 1585).** Runs 4, 5 and 6 of the gate, at 13:17–13:21 on 2026-10-06: load 0.42 to
0.78, 27 GiB free, Tctl 48–51 °C. `bring_up_ms` read 17.08, 16.87 and 16.05, and the band is
13.6 .. 20.6 (was 14.5 .. 22.2). Every `first_page_ms` band moved by the file's rule, and
`xfa_filled_imm1344e.pdf`'s was derived, 20.2 .. 31.0. Runs 1 to 3 were at loads up to 2.8 and are
not used; run 3 failed two warm opens. Run 4 failed `bug1815476.pdf`'s warm open, 0.598 against
0.35, at load 0.89, and that band is not this round's. Run 7, on the new bands, exited 0 with 43
judged.

**Lavapipe.** It is about 3.5 ms of the bring-up, mostly in the adapter check, and 11.2 M of the
child's 118.8 M instructions. Its share of the 45.7 M enumeration is 1.4 M. `wgpu` has no driver
filter. Direct driver loading needs `unsafe`, `set_var` is `unsafe`, and the loader's settings file
is vkconfig's single file. Q270 asks whether a launcher may set `VK_LOADER_DRIVERS_DISABLE=*lvp*`.

**Gates.** rustfmt `--check` on the six `pdf-font` files: 0. `RUSTFLAGS=-D warnings` clippy
`-p pdf-font --all-targets`: 0. `cargo nextest run -p pdf-font`: 0, 251 passed.
`cargo test -p conformance`: 0, 383 passed. An earlier run failed `names.rs` on a sibling's
in-flight `crates/pdf-model/src/aform/number.rs`.
`launch_path` behind the lock: runs 1, 2, 5, 6 and 7 exit 0, runs 3 and 4 exit 101.
