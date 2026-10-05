# 1533 — A page list found by scanning is said out loud, and the pageless readings stand

Session 1349. Status: accepted. Amends ADR 0097 (its page-tree half). Code: `Pages::found_by_scanning`
in `crates/pdf-model/src/page.rs`; the note in `crates/viewer-core/src/notes.rs`'s `about`;
`FOUND_BY_SCANNING` in `crates/pdf-model/tests/corpus.rs`; five fixtures at the end of
`crates/pdf-model/tests/page_tree_nodes.rs`.

## 1. What the brief asked and what the tree already had

The brief asked for a page tree rebuilt from the `/Type /Page` objects in object-number order where
`/Root` or `/Pages` is missing or broken. That recovery exists: ADR 0097 built it, ADR 0782 fixed its
guard, ADRs 0784 and 0786 extended it to dictionaries that stop part-way. What it lacked was a
voice. A page found by scanning a whole dictionary reported nothing. `issue9418.pdf`'s newest
trailer names as `/Root` an object stating `/CreationDate`, `/Creator` and `/Producer` and no
`/Pages`, and its page was shown exactly as if a tree had stated it. §7.7.3.1's tree
"defines the ordering of pages in the document"; the scan's order (ascending object number) and its
membership (every object that says `/Type /Page`) belong to this reader. Trap 5's failure is a
recovery that shows something plausible and says nothing.

## 2. The decision

The statement belongs to the document, as the cross-reference rebuild's does, and is not a page
report. Every mark on such a page is the file's, so an `Unsupported` would take a correctly drawn
page off the oracle's judged set and report nothing about the page itself. So `Pages` answers
`found_by_scanning() -> Option<usize>`. The host's open notes say the count and the order (one
sentence, off the open path, like `was_recovered`'s). The corpus gate holds the population by name,
so a document whose tree stops being walked is a named arrival, not a silent one.

## 3. The thirteen documents, read against their bytes

- `bug1020226.pdf`, `REDHAT-1531897-0.pdf`, `poppler-937-0-fuzzed.pdf` hold no `/Type /Page` in their
  bytes (`grep -a`). The scan has nothing to find. They stay `PAGELESS` / `NoFirstPage`.
- `poppler-85140-0.pdf` does hold one, inside `3 18446744073709551616 obj`. §7.5.4 states "The
  maximum generation number is 65,535", so that header names no object a reference `3 0 R` can
  reach. Its `/Contents` is `4 233245 obj`, past the same bound, so taking it would draw a blank
  sheet 2 147 483 647 points tall. That is a plausible nothing, and it stays refused.
- `Brotli-Prototype-FileA.pdf`: its page tree is inside `/BrotliDecode` object streams, and Table 6
  names no such filter. It is refused with the filter's name. That is right by the clause, and it
  is kept.
- `Pages-tree-refs.pdf` page 2 is a `/Kids` cycle. Its page 1 draws from the tree, so the scan
  correctly does not run.
- `PDFBOX-4352-0.pdf`: object 6 begins `E<` where §7.3.7 puts `<<`, so the `/Encrypt` resolves to
  null. No entries are whole before the damage, so ADR 0784's prefix has nothing to take, and §7.6.2
  forbids reading the ciphertext as plain. It stays refused.
- `encrypted-attachment.pdf` is ADR 1534's.
- The three `ONE_REFERENCE_REBUILT_THE_FILE` pages: only `issue9418.pdf` is ours by scanning, and
  the one reference that draws it, `gs`, agrees at 3.15 of 255; the note now says how we got there.
  `poppler-67295-0.pdf` walks its tree (its `/Count` is not believed). `bug1980958.pdf` walks a
  rebuilt table. No reference invented content on any of them, and none moves.

## 4. What would reopen it

A scanned page whose order is evidence of a stated one (a `/PageLabels` tree, an outline's `/Dest`
indices) would let the order be recovered rather than chosen. No corpus document offers one.
