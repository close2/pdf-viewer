# The Ghent PDF Output Suite 5.0 — where it comes from, and why none of it is here

Status: **standing** — the provenance of a test corpus this tree uses and does not carry (ADR 2728).
The results are [`gwg-output-suite.md`](gwg-output-suite.md).

## What it is

The Ghent Workgroup's PDF Output Suite, release 5.0: single-feature test *patches* for a PDF output
workflow (overprint, spot and DeviceN colour, transparency, soft masks, ICC colour, fonts, optional
content, 16-bit and JPEG 2000 / JBIG2 images), each with a README stating its intent and its method
of evaluation, and combined *test pages* imposing them. Each patch is built to be self-revealing: an
`X`, a half `X` or a word appears where the processing is wrong. It targets PDF/X-1a, -3 and -4
output.

## Where it was fetched, and what was fetched

From <https://gwg.org/resources/ghent-pdf-output-suite/>, on 2026-11-06, two files:

| file on the server | SHA-256 |
|---|---|
| `https://gwg.org/wp-content/uploads/2026/05/Ghent_PDF_Output_Suite_V50_Testpages.zip` | `52d271ddd97dcb5778d1da2bdb3b86a66b96083b7f57ee3fd8ed2b5e5a5d7d29` |
| `https://gwg.org/wp-content/uploads/2026/05/Ghent_PDF_Output_Suite_V50_Patches-2.zip` | `109625cc2f77c3fec703edd19e28b3676323526517c9f65dc9e45b752fa2fb47` |

`find <suite> -name 'GWG*.pdf' -path '*/Patches/*' | wc -l` counts the patches and
`find <suite> -name 'Ghent_PDF-Output-Test-V50_*.pdf'` names the test-page files.

## The licence, read

The suite's legal notice (in `Ghent_PDF-Output-Suite-V50_ReadMeFirst.pdf`, © 2016 Ghent Workgroup;
each patch README repeats it) grants permission to use the suite and its documentation as its
documentation describes, under conditions, paraphrased here:

1. the legal notice must be included in every copy holding the whole suite or a substantial part;
2. without the Ghent Workgroup's written permission, the suite may be used for nothing but its
   intended purpose, testing workflow setup;
3. it may not be sold, nor used in any commercial context, without that permission;
4. it is provided as is, with no warranty and no liability.

**What follows for this repository.** Rendering the patches to see whether this program processes
them correctly is the purpose the suite was made for, so the use is within condition 2. This
repository is public and Apache-licensed, and a copy committed here would travel without the
notice's control under a licence that permits commercial use, which conditions 1 and 3 do not allow.
So **no byte of the suite is tracked here**: no PDF, no README, no rendered image of a patch. The
tree holds this document, the fetch script, the harness, and a results table written in this
project's own words.

## How a checkout gets it — the equivalent of a submodule

The other test corpora are git submodules under `doc/corpora/`. This one has no repository and may
not be redistributed, so it is a pinned fetch instead:

- `tools/fetch-gwg.sh` downloads the two zips (or reuses a verified copy in the download cache,
  `$PDF_VIEWER_GWG_CACHE`, default `~/scratch/gwg`), **refuses on a SHA-256 that differs from the
  pins above**, unpacks them without `__MACOSX`, and writes `NOTICE.txt` beside the suite pointing at
  the legal notice that travels inside it.
- It unpacks into `doc/corpora/gwg-output-suite/`, which `.gitignore` names. A worktree's
  `doc/corpora/` entries are links into the main checkout's, and the script follows such a link, so
  every tree reads one copy.
- `tools/places.sh` names the place `PDF_VIEWER_GWG`, defaulting to the tree's
  `doc/corpora/gwg-output-suite` and overridable by the environment like every other place.
- `crates/pdf-model/tests/gwg_output_suite.rs` skips with a sentence where the suite is absent —
  CI's case — and checks the patches a pixel comparison decides where it is present;
  `tools/gwg-render.sh <out-dir>` renders every page in both views for a person to judge, and refuses
  an output directory inside the tree.
