# 1887 — a font is judged by where it stands, and an unreached image worker is not a budget

Session 1887, batch 129 slot 7 (parsers, robustness), in three sessions (a handover, a death at
the account's limit, the close). ADRs 2608 and 2609 used; Q763 unused.

**What changed.** (1) `pdf-archive`'s font population is every `/Type /Font` dictionary *and* every
dictionary a `/Font` resource or a `/DescendantFonts` array names (Table 34, Table 119), found by
the one walk ADR 2413 added; a new row `fonts/font-dictionary-states-type-font` (6.2.11.2 /
6.2.10.2) fails a font whose `/Type` is not `Font`, and the converter writes `/Type /Font`
(`Rewrite::FontTypeStated`) in the font's object, a direct font's holder, or a substitute's (ADR
2608). (2) `ImageError::DecoderUnreachable` carries `pdf_sandbox`'s `WorkerMissing`, `Spawn` and
`WorkerMismatch` through one `From<SandboxError>`, and the JPEG 2000 transcode gives it its own
sentence instead of the ordinary decoder's budget (ADR 2609).

**The premise that did not hold.** (1) `issue7544.pdf`'s page resource names a typed Type 0 font;
the font without `/Type` is its descendant, so the position `/DescendantFonts` had to be read too.
(2) `PDF_SANDBOX_WORKER=/nonexistent` does not reach a CLI conversion: `quorra-transform-worker`
decodes in process; shown through the library under `Isolation::Sandboxed` instead.

**What it moves.** Refused, the row took seven documents out of "converted" (pdf.js `freetexts`,
`issue16633`; corpora `T02-04-01_003`, `_004`, `AcroFormsBasicFields`, `transitions_test`,
`page_tree_multiple_levels`), each of whose HEAD 2b outputs veraPDF fails on 6.2.11.2-1 alone. With
the remedy the census without the pack is HEAD's line for line, and veraPDF passes all fourteen
outputs (2b and 4). Written first, the entry hid four direct fonts from the substitution's
by-value search and they failed `font-programs-embedded`; the walk now leaves those to `gains`.

**Plants.** The position walk switched off: the fixture failed (left 0, right 1). The unreached
arm folded into the budget's: the transcode test failed showing the budget sentence. The
substitution's skip switched off: the direct-font test's document did not convert.

**Measured.** `getrusage` high-water, 50 000 fonts on a page, HEAD beside this: direct untyped
239 056 / 268 052 kB, direct typed 271 892 / 271 464 kB, indirect untyped 173 164 / 120 680 kB.

**Gates.** export: `cargo fmt --check` three crates 0; `--test cross_check --test witnesses --
--ignored` 0 (both read); census without the pack 0 twice (the second identical to HEAD's);
veraPDF 14 PASS, plant FAIL; `cargo check --tests` 0. worktree: clippy `--no-deps -D warnings -p
pdf-archive -p pdf-model -p pdf-transform --all-targets` 0 after seven `arithmetic_side_effects`
fixes in `sites.rs`; `cargo test -p pdf-archive` 0, `-p pdf-model` 0, `-p pdf-transform --test
archive --test profiles` 0; `--bin cited` 0; `--bin quotations` 0; `cargo test -p conformance
--no-fail-fast` 0 (69 targets, 470 tests).
