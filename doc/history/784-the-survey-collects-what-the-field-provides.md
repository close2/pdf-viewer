# 784 — The survey collects what the field provides, and the RFC directory is born

Session 784, 2026-08-28. An owner-commissioned research round, no implementation. ADR: none —
the RFC convention is stated in the new directory's own README, and this round decided
nothing; the decisions are the owner's to take on the RFCs.

Touched: `doc/rfc/README.md` (new — the RFC convention: proposals the owner marks up,
status draft/proposed/accepted/declined, and the owner's directive that an RFC is not bound
by current rules but names them as restrictions with their rationale),
`doc/rfc/0001-the-survey.md` (new — the market and demand survey), this file.

## What the round did

Branched from `28ed2239` deliberately — a merge round was landing on `main` concurrently —
and surveyed, from live product pages, docs and trackers (all URLs in the RFC, fetched this
week): Acrobat Standard/Pro, PDF-XChange Editor, Foxit, Nitro; Stirling-PDF (confirmed as
the owner's "successful recent open source PDF web service" — nothing else matches);
the CLI field (qpdf, pdftk, mutool, poppler-utils, ocrmypdf, ghostscript's pdfwrite) for
operations *and* interface conventions; the issue trackers of Okular, Evince, pdf.js,
SumatraPDF, Zathura and MuPDF for what users actually file; and KDE's prior art for a
`pdf:/` KIO worker (kio_archive, KioFuse, Okular's integration surface).

The survey's headline findings, argued in RFC 0001 §9: the demand centre is document
transforms, and nearly every easy-or-moderate gap is gated on one missing piece — a
whole-file writer; printing is demand-backed pain in both major Linux viewers, and a good
print preview is this tree's existing core competence; every surveyed viewer is growing
into an editor under user pressure, annotations first; KIO/FUSE have no tracker demand and
real integration value, and the RFC says so honestly rather than dressing them in votes.

Mid-round the owner directed that the RFCs must not be limited by current project rules —
naming the authoring exclusion and `pdf_syntax::Document`'s immutability — so the gap
matrix's easy/moderate/hard estimates are for the unconstrained design, with each standing
rule recorded as a current restriction with its original rationale (the oracle's purity
argument, principally) as a data point rather than a constraint. RFC 0001 §8 frames the
exclusion amendment in three tiers for round 785 to argue.

Reserved in the index for the sibling design rounds: RFC 0002 (transform suite and CLI,
round 785), RFC 0003 (KIO/FUSE, round 786), RFC 0004 (print, preview, text editing,
round 786). Placeholders are index rows only — no files, deliberately, so the pointers
sweep has no phantom paths to find.

## What the round can attest

Doc-only change, so per the change→gate map: fmt clean, clippy silent under
`-D warnings`, the workspace suite green under nextest, doctests green, and
`cargo test -p conformance` green with `doc/rfc/` in the tree — the checker walks the new
directory (its `prose::documents` takes every Markdown under `doc/`) and objected to
nothing. The §4 sweeps ran before and after against the pristine baseline; every delta
accounted: the new documents add a handful of governed sentences and quotations, all in
the buckets a non-specification document should land in (unrelated quotations, unrooted
URL pointers), with zero new absent pointers, zero new diverging quotations, and the
attributed counts and table citations unchanged.

Two incidents worth their sentences. `git reset --hard 28ed2239` in the fresh worktree
replaced the `doc/arlington-pdf-model` and `doc/pdf.js` symlinks with empty directories —
the same hole `doc/environment.md` records for `git checkout -- doc`, reached through a
fourth door; repaired by the documented `rmdir` + `ln -s`, and the corpora symlinks
survived under their skip-worktree flag. And `tools/round.sh` reported CI's last run on
`main` as failing at round start; a doc-only round branched off a pinned commit is not
placed to fix `main`, so it is recorded here for whoever merges next.
