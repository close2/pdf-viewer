# 2950 — The pin moves to Rust 1.100

Session 2068. Status: **accepted.** It cites no clause. It moves the toolchain pin and five counted bands in
`doc/checks/launch-path.toml`, and no ledger row. Context: ADR 2910 (what the bump needs, and why it was
held), ADR 2914 (a program found from a test's path under both of Cargo's layouts), ADR 2915 (what 1.100
adds to an open), A875 (the lint form of the owner's audit tests).

## 1. What held the pin, and what released it

ADR 2910 held the pin at 1.97.1 for two reasons: Cargo 1.100 puts a test executable in
`<profile>/build/<package>/<hash>/out/`, where every lookup that searched upwards from a test's path
for a program failed, and 27 of the owner's audit test files needed lint-form edits that Q875 asked
about. ADR 2914 answered the first. Its one rule, `pdf_sandbox::beside`, reads the profile directory
from an executable's path under either layout (`deps/`, `examples/`, or `build/<package>/<hash>/out/`)
and refuses any other path by name; every program lookup a test or a shipped program makes goes through
it. So this round has no lookup to fix: it asks the same question ADR 2914 says, and under 1.100 the
workspace's tests pass with no worker named in the environment. A875 answered the second: a toolchain
bump may make purely syntactic lint-form edits to the owner's audit tests where every assertion's
operands, inputs and expected outcome stay the same, and the round's record lists each edited file.

## 2. The bump

`rust-toolchain.toml` names `1.100.0`; the workspace's `rust-version` and `clippy.toml`'s `msrv` name
`1.100`. Clippy 1.100 over the tree as it stood (`--workspace --all-targets`, the lints the workspace
sets) reported 1056 sites:

| lint | sites | of them in the owner's audit tests | rewritten as |
|---|---|---|---|
| `chunks_exact_to_as_chunks` | 410 | 19 | `as_chunks::<N>()`, whose first half is the same chunks and whose second is the remainder `chunks_exact` dropped |
| `assert_is_empty` | 294 | 13 | `assert_eq!(v, [])` (an array type named where inference needs one), so a failure prints what was there; `assert_ne!` for the negation |
| `map_unwrap_or` | 254 | 1 | `Option::map_or_default`, new under this `msrv` |
| `manual_midpoint` | 45 | 0 | `f32::midpoint`, the same value for every pair that does not overflow |
| `unfulfilled_lint_expectations` | 38 | 5 | the `#[expect]` removed, or its list shortened to the lints that still fire (`float_cmp` no longer fires inside `assert_eq!`) |
| `manual_is_variant_and` | 8 | 0 | `is_some_and` / `is_ok_and` |
| `manual_bit_width` | 4 | 0 | `bit_width()` |
| `needless_nonzero_get`, `needless_range_loop`, `useless_format` | 1 each | 0 | as clippy suggests |

Cargo 1.100's own lints named six workspace dependencies no member uses (`proptest`, `raster`,
`viewer-ffi`, `viewer-gtk`, `viewer-qt`, `zune-png`), removed, and `pdf-model`'s `pdf-spec`, which only
an example reads, now a dev-dependency. The lock file does not change. Round 2047's patch was the guide;
ten further sites that rounds since have added were fixed the same way. The owner's second audit
(`bf59984d`) arrived during the round and brought 39 more sites, 24 of its files among them (23 once main's own fixes took one), and the 8
expectations their rewrites left unfulfilled; they were rewritten in the same forms, with clippy's
fully qualified array types shortened to the names each file already imports.

Every edit to an owner's audit test is of A875's kind: the same operands, the same inputs, the same
expected outcome. An `as_chunks::<N>().0` yields exactly the chunks `chunks_exact(N)` yields, and an
`assert_eq!(v, [])` holds exactly when `v.is_empty()` does. The record lists each edited file.

## 3. The bands

`launch_path`'s counted half under 1.100, from the gate's own release build, put three figures outside
their bands: the two opens ADR 2915 read (WTPDF 10098.1 thousand against 9463 .. 9656, EC3 55619.5
against 52862 .. 53931) and the font-walk row's first frame (`bug1815476`, 63482.1 against 64104 .. 65400).
Four children per document and phase agreed to a tenth of a thousand. Each figure that moved was read
against round 2065's callgrind profiles of the same children under 1.97.1, by function:

| figure | 1.97.1 | 1.100 | where |
|---|---|---|---|
| WTPDF open | 9559.4 | 10098.1 | zlib-rs `inflate_fast_help_avx2` (ADR 2915) |
| EC3 open | 53396.4 | 55619.4 | the same loop, and the cross-reference entry closure (ADR 2915) |
| EC3 first frame | 157676.8 | 159060.6 | the open's two, less 1.56 million in `image::unpack::sample_rgba` |
| `bug1815476` first frame | 64752.1 | 63482.1 | 1.09 million fewer in zune-jpeg's `ycbcr_to_rgba_unsafe`; 1522 commands in both |
| `xfa_filled_imm1344e` first frame | 3236.4 | 3266.7 | `pdf_render::geom::Path::bounds`, a third more, its source unchanged |

Every one is code generation for unchanged source: none of the functions named was edited, and
the lint rewrites reach none of them. Those five rows, each past half the rule's one per cent margin, are
restated by the band file's rule over the four children (the observed range widened by one per cent,
rounded outwards), and each comment names the compiler and this ADR. The other seven counted figures
moved by under half a per cent and keep their bands. The clocks were judged beside them, as ADR 2915
section 2 asks, in two runs; the record has their figures.

A falling count is the habit's case for a reading against the build that last held the band; here that
build is the same source under the other compiler, and the function diff is that reading.
