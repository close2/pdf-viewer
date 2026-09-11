# 959 — A bomb bound that truncated in silence, and a gitignored directory in the oracle's population

Date: 2026-09-11. ADRs: 0961, 0962 — both of which say *Session 945* on their status line, because
`tools/round.sh` told this round it was 945. That is ADR 0963 and session 963's finding.
Files: `crates/pdf-model/src/mesh.rs`, `crates/pdf-model/src/shading.rs`,
`crates/pdf-model/src/interpret.rs`, `crates/pdf-model/tests/oracle.rs`,
`doc/todo/00-ambiguous-bucket.md`.

The viewer stream, and the finding arrived as a false start. An A/B raising `PATCH_STEPS` — a
*finer* tessellation, which should only improve a page — reported one page moving 6.81 of 255 with
limbs missing from the finer arm. Trap 1 caught it: the finer arm had run the same page into
`mesh::MAX_TRIANGLES`, eleven lines down the same file. With the bound lifted the page moves 0.0029.

**The false start is the finding.** `MAX_TRIANGLES` stopped a mesh part-way and the page was drawn
and **reported complete**, from the day it was written. Every neighbouring bound in this crate is
already a `LimitReached`; `mesh::read` simply returned a pair with nowhere for the fact to sit. So
`mesh::read` returns `Mesh { triangles, ramp, truncated }`, `shading::Cache::build` returns
`Shaded { shading, truncated }`, and the interpreter raises
`LimitReached { limit: "max_mesh_triangles" }`. The bound is tested **after** the next vertex has
been read, so the flag says *something was dropped* rather than *a count was reached* — which is
the difference between a report a reader can act on and a report that is true of a page nothing
was lost from. The census off the display list found 40 mesh paints over 1,516 files, the largest
at 23.3% of the bound: nothing in the corpus was being truncated today, which is exactly why this
had survived. A bound with no witness is not a bound nobody needs.

**The second finding cost nothing and was nobody's code.** `ICC.1-2022-05.pdf` appeared newly
contradicted — because `corpus_items` walks `/doc/*.pdf`, which is gitignored, and six
specifications had landed there from this week's downloads. The oracle's population is therefore
partly a function of what a neighbour put on the disk. Nobody's reading of the page is wrong: the
wordmark is a non-embedded `Arial-ItalicMT`, and the two references convicting us **share a
foundry** — ghostscript names `NimbusSans-Italic` and mupdf carries URW compiled in — while we and
poppler sit on the Liberation design, 3,839 device columns against 3,844 at eight times. It joined
its group with the measurement rather than on membership, which is trap 9's rule.
