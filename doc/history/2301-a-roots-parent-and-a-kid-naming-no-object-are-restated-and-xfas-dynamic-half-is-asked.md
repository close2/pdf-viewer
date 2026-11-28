# 2301 — A root's `/Parent` and a kid naming no object are restated, and XFA's dynamic half is asked

Lane `gwg`. This round took `doc/todo/72`'s top site and asked the owner one question. ADR 3905, Q1122.

- **The census** (`archive_unbuilt` over `doc/pdf.js/test/pdfs` and `doc/corpora`, base `4c2d361bb`):
  the top site marked `not-built-yet` at all six targets was `file-structure/page-tree-kids-name-dictionaries`
  with 9 documents, and `…-root-states-no-parent` had 1. The census counts documents the validator fails,
  but every one of these 9 had already been declined before stage 1. It converted only behind
  `recovered-structure`, as a rebuilt tree, and the row's `NotBuiltYet` meant only a node written directly.
- **The remedy** (ADR 3905): the root's `/Parent` is removed, and each `/Kids` entry naming no object (§7.3.10's
  null) is removed from its array. Both happen in the producer's own tree, as the `Mechanical` rewrite
  `page-tree-restated`, so no loss is asked for. Two shapes still decline the source: a child the file places
  and does not hold (`page-not-held`), and a null entry beside a sibling that is neither a page nor a node.
  One classifier, `page_tree::child`, serves the walk, `children_not_held` and the restatement.
- **Probed** at 2b and 4, with no profile and under `as-shown`, each output checked by veraPDF 1.30.3
  (`quorra-transform archive --in-process`, then `verapdf --flavour`). With no profile, 0 of 9 converted before and
  3 of 9 after (both `PDF-HUL-29` files and `PDFBOX-6040-nodeloop`), all veraPDF PASS. Under `as-shown`, 5 of 9
  converted both before and after: the same three, now with no loss, and `T02-02_003` and `T02-02_004`,
  still through the rebuild. The other four stop at `page-not-held`, as `as-shown` says they should.
- **The first probe found a regression and the exception fixed it.** Without the sibling rule, `T02-02_003`
  (whose second kid is the page's resource dictionary) was refused at `conformance/adheres-to-the-base-standard`
  with a substituted media box, where the rebuild had converted it. The gates were stopped, the source
  changed, and the gates restarted.
- **Q1122** asks whether `doc/todo/78` may start while §12.10.4 waits on Q1114 for a document. The ledger has
  883 rows, with 2 `partial` (§12.10 and §12.10.4, one gap). The recommendation is to start now.
- **The census after the change** lists neither row at any target: 17 sites not built at 2b become 15, and
  10 at part 4 become 8. Every site still marked `not-built-yet` is reached by 0 documents. On the base, the
  veraPDF corpus census likewise reached none.
- Left: a node needing either restatement that is written inside its holder (no witness yet). Next is
  `metadata/xmp-character-data-only-in-simple-values`, which no document reaches.

**Gates.** All over `-p pdf-archive -p pdf-transform -p pdf-model -p conformance`: fmt 0, clippy
`-D warnings --all-targets` 0, `cargo build -p pdf-sandbox --bins` 0, rustdoc `-D warnings
--document-private-items` of `pdf-transform` 0, `cd fuzz && cargo check --bins` 0 (`Rewrite` gained a variant).
nextest `--lib --bins --tests`: 5506 of 5508, and the 2 failures were this record's own two record checks,
fixed and then passing in `cargo test -p conformance --no-fail-fast`, which exits 0.
