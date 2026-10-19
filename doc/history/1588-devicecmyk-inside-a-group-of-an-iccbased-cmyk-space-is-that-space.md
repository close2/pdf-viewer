# 1588 — DeviceCMYK inside a group of an ICCBased CMYK space is that space

Slot 4 of batch ninety-three, 2026-10-19, a clause round. ADR 2012; ADR 2013 and Q483 unused.

**Premise.** It held: with the redefinition planted away, a `k` on a page group of one profile under
another's output intent is the intent's colour converted into the group's, and keeps no overprint
mode (ADR 1996 §4). Read anew, §11.7.2 says it three times — the redefinition "within the
transparency group", the "nearest such ancestor" for a device colour, and "the full semantics of
DeviceCMYK shall still apply, including overprinting"; §14.11.5's intent is "for informational
purposes only"; §11.6.6 confines it to isolated groups; Table 145 adds the `/DefaultCMYK` route.
**The hypothesis did not hold**: no corpus page moves, at any scale or lane.

**Built.** `Reading::within_cmyk_group` and `Conversion`'s twin carry the space; `device_family`
ranks default, redefinition, intent, device. `Interpreter::cmyk_group` is a scope: the page
group's (`page_cmyk_group`), an isolated group's across every run of its content in
`group_commands`, a soft mask's group's own `/CS` in `enter_mask_compositing`;
`stated_cmyk_group` says which entries state one and `may_state_four_components` keeps an RGB or
grey profile from being read for it. Images, shadings and meshes parse through the new
`Conversion::parse_space`; `icc_spaces` is keyed by the redefinition beside the object, since an
unparsed profile's `/Alternate` can be redefined. Seven tests in `tests/overprint.rs`, each an
equality with a named colour; planted back, the four positive ones fail with the redefinition
gone, the non-isolated one with isolation ignored, the default one with the ranking swapped, the
mask one with the mask's scope emptied.

**Rows.** §11.7.2 stays `departed` (the press budget), its note gains the redefinition and seven
tests; §14.11.5 stays `implemented`, its ranking sentence and parse route corrected; §11.6.6 stays
`departed`, one test added. `doc/state-of-play.md`: one sentence.

**Found, not built.** `content.rs`'s checkpoint `#[expect]` says the interpreter has "fifty-nine"
fields; it had 79 at HEAD (80 now) — a counted fact in a reason, stale before this round.

**Gates.** rustfmt clean on my ten Rust files. `RUSTFLAGS=-D warnings clippy -p pdf-colour -p
pdf-model --all-targets`: exit 0. `nextest -p pdf-colour -p pdf-model`: 2245 passed, 0 failed.
`cargo test -p conformance`: exit 0.
`--bin quotations`: no divergence in my files or rows. Six arms in one `--tree 12` hold of 1480 s:
each exit 0, 968 pages at 1× and 965 at 4× per lane, 0 different from HEAD's arms
(corpus SHA-256 `b8ae9c79…` against HEAD's `2b692928…`). `t2-raster_golden` in the same hold: held
974, moved 0. `t3-oracle` under `--clock`: exit 0, 1970 pages, 47 contradicted, 0 held as ours.
Duration: 10 800 s.
