# 3583 — Every errata source held, and re-fetched by the train

Session 2220. Status: **accepted**. Context: ADR 3581 (step 2 of the owner's plan of 2026-11-24),
ADR 0252 (`tools/spec-errata`, which reads the errata annotations inside ISO 32000-2's sponsored
PDF and nothing else).

## Decision

`doc/errata/` holds, under CC-BY-4.0 with the attribution in its README:

- the Jekyll source pdf-issues.pdfa.org is built from (`docs/` of `pdf-association/pdf-issues`,
  every Markdown page, the data file and the grammar files, not the figures), at the commit `main`
  named on the fetch — the source rather than the built HTML, because it is what the site is made
  of, it diffs, and its `data-issue` and `data-iso` attributes say which issue and whether ISO
  approved each change;
- every issue of that repository, every state, with its labels (`issues.jsonl`), and every
  comment (`comments.jsonl`), where the working groups record agreement — the second half of ADR
  3581's criterion, which the site does not carry;
- `MANIFEST.tsv`: path, SHA-256, source URL (the commit for the site), fetch date.

The README lists the fifteen documents the site has pages for and which of them `doc/` holds
(ISO 32000-2, 19005-4, 14289-1, TS 32001, 32002, 32003, 32005, WTPDF 1.0, PDF Declarations), and
says that ISO 19005-2 has no page and is reached through the issues.

`tools/fetch-errata.sh` writes it; `--check` fetches into a temporary directory and fails naming
each file whose hash moved. The records are reduced to fixed fields with sorted keys, one a line,
so an unchanged tracker writes the same bytes (checked twice on the day of the fetch). The
post-merge train runs `--check` daily; a failure is a prompt to read what moved and re-fetch,
never something to quieten. Any new comment on any issue moves the hash, deliberately.

## Cost

5.8 MiB in the repository at the first fetch, growing with the tracker; a minute of network per
fetch. Holding only the PDF/A issues would have been smaller and would have missed what ADR 3581
covers: every standard this tree reads.
