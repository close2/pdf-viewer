# 875 — A grey is the clause's, a holder with a fixture, and the writer over the corpus

2026-09-02. Argued in [ADR 0804](../adr/0804-a-grey-is-the-clauses-a-holder-with-a-fixture-and-the-writer-over-the-corpus.md).
The fifth implementation round of [RFC 0002](../rfc/0002-the-transform-suite.md), on the
long-lived branch `round-867`, which started by merging `main` (round 871's blending spaces)
cleanly.

Touched: `crates/pdf-transform/src/render.rs` (`ImageFormat::Pgm`, `grey_of`, `pgm`),
`src/images.rs` (`ImagesPlan::format`, `Route`, `ImageFile`'s netpbm variants, the mask's
netpbm form), `src/bin/pdf-transform.rs` (`--format` for both verbs, the usage text);
`tests/verbs.rs`, `tests/writer.rs` (the `/Names`-indirect fixture), **`tests/writer_corpus.rs`**
(new); `doc/todo/02-every-round.md` §2 (one line), `tools/state.sh` (`writer`);
`doc/conformance/ledger.toml` (§7.5.6, §7.7.4, §7.11.4, §10.4.2.2, §12.5.6.15);
`doc/state-of-play.md`, `doc/todo/README.md`, `doc/todo/57-…`; `doc/adr/0804-…` (new), this
file.

## 1. What landed

All three items of the round's scope, and the fourth where time allowed:

- **`--format pgm`** for `render` and `images`, on ISO 32000-2 §10.4.2.2's conversion,
  through `pdf_render::Color::grey_level` — the one place the tree states the NTSC weights —
  so a grey file, a luminosity mask and a grey blending space cannot disagree (trap 6). What
  an image in another colour space becomes is a stated choice: the interpreter's own
  conversion to RGB first, which §10.4.2.1 ranks above this clause's family, then the clause;
  a native stream is never converted. A netpbm file has no alpha, so the mask goes beside it
  as a PGM.
- **The `/Names`-indirect holder fixture**, built in `tests/writer.rs` and held to what the
  shape implies: the name dictionary's object rewritten, the catalog's not.
- **The writer over the corpus**: `tests/writer_corpus.rs` attaches a file into every corpus
  document the suite opens, reads it back, removes it and files it on page 1 — exact
  assertions, the writer's refusals by reason, every holder shape counted — on `doc/todo/02`
  §2's sequence and in `tools/state.sh`, run under `tools/bounded.sh`. Its census answered
  `doc/todo/57` §1's question about the holder shape: the corpus has no document of it, so the
  fixture was the right instrument.
- **The thread curve** re-taken at 2, 4 and 24 threads; ADR 0804's last consequence has the
  rows, every one at or under ADR 0801's.

## 2. What was looked at

The walk's own census lines, document by document under each refusal; `qpdf --check` on the
fixture's outputs, which first rejected the *fixture* (a page with no `/Resources`, repaired
with a warning) and then accepted the update; the PGM of page 1 of `PDF20_AN001-BPC.pdf`
against the clause's formula written out in `f64`, byte by byte.

## 3. Gates

`pdf-transform` and two documents the conformance gate reads changed, and `doc/todo/02` §2
gained a line; the whole sequence was run in this worktree, the walking lines under
`tools/bounded.sh` — one walk on the machine at a time, after waiting for a neighbouring
round's survey to finish (the memory rule of 2026-09-02). The results are in the round's
report and not here.

## 4. What the next transform round does first

`doc/todo/57`'s order, as rewritten this round: the serializer and the four verbs on it wait
on RFC 0002 §13 question 1, JPEG output on question 2, and `--password-prompt` on
`doc/stack.md`'s decision on a terminal-mode dependency. What the walk does not see is stated
in that file's §5: a corpus-wide *foreign* readback of the transform's updates.
