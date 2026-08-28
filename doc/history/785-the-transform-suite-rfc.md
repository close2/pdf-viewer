# 785 — The transform suite, designed on paper before anything writes a file

2026-08-28. No ADR: an RFC is a proposal, not a decision, and the reserved 0721 was not needed.
Files: `doc/rfc/0002-the-transform-suite.md` (new, and the round's whole product), this file.

An owner-commissioned design round, docs only. The RFC designs split, merge, page assembly,
image extraction, page rasterisation and optimisation as one CLI binary (`pdf-transform`) over
one library crate whose public API — the *transform seam* — is what round 786's KIO/FUSE/UI
consumer designs sit on: pure plans, caller-supplied sinks, a host-supplied restrictions policy,
explicit budgets, no filesystem or clock inside. Under the owner's directive that current
project rules do not bind the proposal, it records the immutable-`Document` rule, the authoring
exclusion and ADR 0121's incremental-only writer as restrictions with their rationales, then
recommends: a structure-preserving whole-file serializer in `pdf-syntax` (the qpdf shape;
re-distillation rejected on fidelity grounds), with `pdf_syntax::Document` staying immutable
because a transform builds a new file from immutable sources rather than mutating one — the
oracle-purity argument survives the whole suite. §11 of the RFC is the ratifiable amendment:
authoring *content* stays excluded, assembling documents from existing documents comes in, the
fence drawn at "does the operation invent marks" (which keeps overlay/watermark out), and the
ledger's `writer-side` status narrows the day the serializer lands.

Two findings worth a line each. The ledger already carries a `writer-side` status whose header
definition — a generator's clause, "this program writes only §7.5.6's updates" — is exactly the
sentence the serializer falsifies, so the amendment has a mechanical worklist
(`grep -n 'writer-side' doc/conformance/ledger.toml`). And the conformance *gate* does not read
`doc/rfc/` — `citation::rust_sources` walks `.rs` files and the ledger — while three *sweeps* do:
pointers, quotations and tables each moved by exactly the new file's contents (zero absent
pointers, zero diverging quotations, one Table 22 `/P` citation in the same reported bucket
CLAUDE.md's own Table 22 sentence sits in).

Gates: fmt, clippy under `-D warnings`, nextest, doctests, the fuzz `check` and
`cargo test -p conformance` all green in this worktree; §4 sweeps diffed against a pristine
baseline taken before the file existed, deltas accounted above. Prior-art interface conventions
(qpdf, pdftk-java, mutool, poppler-utils, Ghostscript pdfwrite, Stirling-PDF, cpdf) were taken
from the tools' current documentation and cited by URL in the RFC — imitating interfaces is a
different register from principle 5, and the RFC keeps the two apart explicitly.
