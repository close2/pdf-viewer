# 1302 — The fuzz lock tracked, todo 12 cut to its header, and what a merge cannot carry

Instruments slot of batch forty-six. ADRs 1439, 1440. No ledger row moved; no question.

**The lock (ADR 1439).** `fuzz/Cargo.lock` was ignored and copied from the main checkout, whose copy
no merge updates: it pinned `hybrid-array` 0.4.13 and `wnaf` 0.14.0 against 0.4.15/0.14.1 and lacked
`pdf-ccitt`. Tracked now, aligned by `cargo update --precise`, and `tests/fuzz_workspace.rs` holds
every shared package to a root version (137 locked, 134 shared, 0 apart). `cargo +nightly fuzz build
-O -s none` exit 0. The same test holds each target to a `[[bin]]`, a `doc/verify.md` line and a
`fuzz/seeds.sh` arm; `seeds.sh` is one arm per target, and `fuzz/seed_streams.py` seeds six targets
with the object each reads (xmp 390, sfnt 402, cmap 20, ccitt 160, crypt 25, variable_text 51 from
pdf.js; INITED cov against the grown corpora 689/1584 … crypt 1064/837).

**Todo 12.** Both re-run commands moved to `doc/verify.md`, ADRs 0575/0616/0617 named, the file cut to
its header; 29 citing sentences in `oracle.rs`, `image.rs`, `image_reuse.rs`, `pdfref`, `oracle-and-corpus.md`
(the "the work" line) and `todo/00` point at ADRs 0243/0771/0774/0776.

**Overtaken notes.** Six, not three: ADR 1419's three and ADR 1421's three. Each read against its ADR;
no sentence false (1419's pages are atlas history, 1421's are GPU tiling cost; the notes argue CPU
verdicts, a font report and a device refusal). `READ` updated, `INCOMPLETE` added: 6 → 0.

**Documents.** state-of-play: ADR 1421's stroke, one fuzzing paragraph. PLAN: the fuzz bullet, the
gate, superlatives. crate-map: the gate. HANDOVER: rows for fuzzing, RFC 0008 (proposed, Q193 open)
and the owner's disk. Navigation 13 absent/26 undefined before and after; history grep 0.

**todo 65.** Membership re-derived (14 rows, unchanged). `cargo search`: no `bp512`; `bp512-nestler`
is PolyForm-Noncommercial; `ed448` 0.5.0 is types; `ed448-goldilocks-plus` waits on Q192. T.4 Annex E
read: XYZ for D50 only — §7.4.9's note edited in that one sentence (1301's file, named).

**The owner's disk (ADR 1440).** `doc/environment.md` *After a merge*, and `tools/state.sh
main-checkout`: `.gitignore` in the way (owner's `/doc/qpdf` edit), lock 2 apart, 3 read artefacts,
the 2 unread page timeouts re-run under `-timeout=60` (67 ms, 3.5 s) and named, 5 targets unseeded.

**Found:** the batch's `conformance` rlib had main's `CARGO_MANIFEST_DIR` (shared target dir, mtime
freshness) — siblings' runs read main's ledger until a touch rebuilt it. `batch.sh open` now names a
build directory of its own.

Left: `pdf-model` clippy blocked by a sibling's `label.rs` mid-edit.
