# 1879 — A test that decodes states its isolation, so two JPX tests no longer depend on order or on a stale worker

Slot 9 of batch 128, 2026-10-29, a "gates" round (robustness, Opus 5.5), one session; ADR 2592, ADR 2593 and Q755
unused. I worked in `scratchpad/r1879/` and one export of this tree built there. No ledger row moved: the slot is a
gate's determinism.

**The premise that did not hold.** The brief's hypothesis was a wall-clock budget in the decoder's `Limits`, reached
under load. There is no `pdf-jpx` crate; the decode is `pdf_sandbox::decode` through `pdf_model::image::jpx_samples`,
and the only clock on it is `REQUEST_TIMEOUT`'s thirty seconds, four orders above an 8×8 decode. Twelve suite runs
at HEAD, alone and beside a release build or CPU burners (load 11 to 30), never failed. The failure came back at once with the
image worker made unreachable (`PDF_SANDBOX_WORKER` naming no file): exactly these two, panicking at
`tests/archive.rs` 12850 and 12925 — the two lines of round 1864's report — while the filter `jpx` passed all
fourteen, its "passing alone". The sentence they print, `NOT_DECODED_WHOLE`, names "the ordinary decoder's budget"
for every `ImageError::Sandboxed`, a missing or mismatched worker included, which is why the red read as a budget.

**The cause (ADR 2592).** The isolation is process-wide and `deep_jpx`'s seven tests set it to in-process, so a test
that leaves the default decodes through the image worker or in this process by which thread ran first; and the worker
is a program `cargo test` does not rebuild, refused by build identity once `Cargo.lock` changes, as batch 127's did.

**What changed.** `tests/archive.rs`: `decode_in_this_process`, its reason written, called first by the two
tests and by the two `keep_everything_transcodes_a_jpx_image_*` tests, order-dependent the same way — the route of production's default conversion, inside `quorra-transform-worker`; no budget touched.

**The plant.** The helper's body emptied: both tests fail in the suite with the worker unreachable, with
`NOT_DECODED_WHOLE`'s sentence (493 passed, 2 failed); restored, byte for byte the merged edit (`cmp`).

**Gates.** In the export: the archive suite ten times beside a release build of the export and sixteen CPU burners
under `tools/bounded.sh --tree 12` (load 41 to 94), each run with the worker present and absent — twenty runs, exit 0,
495 passed each; the two alone with and without the worker, exit 0 each; bound exit 0, peak 3.11 GiB.
`cargo fmt --check -p pdf-transform` exit 0; clippy `-D warnings` `-p pdf-transform --all-targets` exit 0; `cargo
nextest run -p pdf-transform` exit 0 (940 passed); `cargo test -p pdf-sandbox` exit 0 (64 passed; the brief's
`-p pdf-jpx` names no crate). `rustfmt --check --edition 2024` on my file exit 0. In the worktree: `cargo check -p
pdf-transform --tests` red on a neighbour's mid-edit (`Authorisations`' new field, slot 1), none at my lines;
`--bin cited` exit 0; `--bin quotations` exit 0; `cargo test -p conformance --no-fail-fast` exit
101, one red, `records` on round 1875's record, not mine. After the two more: all four `--exact`
without the worker exit 0; the suite without it 495 passed; clippy `--no-deps -D warnings` exit 0.

**Found, not fixed.** `src/archive/transcode.rs` `not_decoded` (~407) gives a budget's sentence to a worker that
could not be reached (trap 11's shape); `ImageError::Sandboxed` carries only a string, so telling the two apart there
needs `pdf-model`'s error widened, which is not this slot's.
