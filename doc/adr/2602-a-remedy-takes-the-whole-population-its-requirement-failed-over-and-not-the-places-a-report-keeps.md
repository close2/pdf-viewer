# 2602 — A remedy takes the whole population its requirement failed over, and not the places a report keeps

Session 1884, slot 4 of batch 129, an "archive" round (robustness).
Status: accepted and built in `crates/pdf-transform/src/archive/sites.rs` (`page_resources`),
`crates/pdf-transform/src/archive/fonts/truetype_cmap.rs` and the one call site in `prepare.rs`.
Context: trap 289 and `pdf_archive::Named` (the set a remedy iterates, bounded at `MAX_NAMED`),
ADR 1953 (every page in one walk), ADR 2500 and ADR 2523 (the 32 MiB budget on copies of forms).
ADR 2603 is unused.

## 1. Two sites read the report's prefix as the set

`pdf_archive::Findings` keeps the first thirty-two places of a requirement and counts the rest
(`MAX_KEPT`). That is what a report should hold, and it is a prefix: two converter sites read it as
the whole population.

- `sites::page_resources` gave a page its inherited `/Resources` only where a kept place named that
  page. Thirty-three pages inheriting one dictionary converted thirty-two and the output failed
  `graphics/content-streams-have-an-explicit-resources-dictionary`: the probe, run at HEAD before
  anything changed, was declined with exactly that sentence at both targets.
- `fonts::truetype_cmap` asked `total > places.len()` for the `cmap`, encoding and `.notdef` rows and
  refused when it held — although its fonts were already read from `judgement.named`
  (`sites::objects_named`), which is complete up to 4096 objects and refuses by name past them. Two
  fonts showing twenty-seven codes on glyph 0 each (fifty-four places) were refused at HEAD.

## 2. What each site takes instead

**The pages are the survey's.** The predicate is a filter over the survey's opened streams (a
stream that named a resource and was read against an inherited dictionary), and the page site now
reads the same population with the same condition at a page's own `/Contents` — the one stream the
survey opens with no object beside it. The survey is always built when this rewrite is wanted
(`prepare::SURVEYED`), and checker and converter build it with the same function of the same
document, so the pages are those the requirement failed at, every one. Where the survey stopped
at one of its bounds, the pages past it are unknown, and every page stating no `/Resources` is
given what it inherits: §7.7.3.4 says the value "shall be inherited as-is", so writing it on a page
that names nothing changes no name's resolution, and over-giving is the safe direction.

**The fonts are `Named`'s.** The two prefix tests are removed; `objects_named` stays the bound.

Reading the finding's count and walking the document again was the other construction the brief
offered. It is not taken because the survey *is* the walk, already paid for.

## 3. The bounds the whole population needs

The prefix bounded these sites by accident. Each loop now reached by every page is bounded before
the work:

- **Pages once, nodes once.** `Pages::every` enumerates the tree in one walk (ADR 1953); a
  memo keyed by node gives each page tree node's inherited value once, so a flat tree's root is
  not cloned for each leaf. The chain is followed at most `MAX_INHERITANCE_DEPTH` nodes.
- **Copies of a direct dictionary are budgeted at 32 MiB**, the figure ADR 2523 set for copies of
  forms. A dictionary written directly in a node is no object a page can name, so each page gets a
  copy. A copy's weight is counted (48 bytes a value plus its names' and strings' bytes) by a
  stack walk that stops at the cap; past the budget the row is refused by name. Writing the
  dictionary once as an object every page names would need no budget; it changes HEAD's output for
  every such document, small ones included, and is left for a round that owns that change.

## 4. Evidence

- `tests/archive/whole_population.rs`: a thousand pages inheriting by reference and directly
  convert under `as-if-printed` at 2b and 4 and conform; two fonts with fifty-four `.notdef`
  places are each given the copy at both targets; 140 pages of a 4000-entry direct dictionary
  convert and 170 are refused with the budget's sentence.
- `truetype_cmap`'s test of "one font past the checker's places is refused" stated the defect as
  behaviour; it now holds that thirty-two and thirty-three fonts both convert.
- The plant: `MAX_KEPT` set to 2, the 36 tests of the four affected test modules pass; the same with
  the two sites at HEAD's text fails six of them, the three new ones among them.
- The census against HEAD's, both arms (no pack, CJK pack): seven pdf.js documents are written
  that were refused, at both targets and under both shipped profiles. Six left
  `fonts/non-symbolic-truetype-differences-are-listed-names` (bug1132849, issue13211, issue6068,
  issue6889, issue6894, issue6961), whose row keeps issue13316_reduced alone; issue14117 left
  `fonts/truetype-codes-reach-glyphs-by-the-standard-route`, whose row is gone. Every refused
  figure falls by seven (as-if-printed 2b 83 to 76, 4 74 to 67, refused at both 68 to 61, no
  pack). The page-resources site moved no corpus document: none has 33 inheriting pages.
- veraPDF at 2b and at 4 passes the conversion of each fixture (1 000 pages by reference, 1 000
  pages direct, two fonts on `.notdef`) and of each of the seven freed documents; the plant (the
  header comment's high bytes made ASCII, ISO 19005-2 section 6.1.2) fails.
- `getrusage` beside HEAD: the 1 000-page document inheriting by reference is refused at HEAD
  (exit 4, 16 544 KiB) and converts here (exit 0, 17 288 KiB); the hostile 100 000-page document
  inheriting a 4 000-entry direct dictionary is refused by both (exit 4), at 612 044 KiB and
  260 s of processor time at HEAD and at 393 940 KiB and 110 s here.
- With ADR 2598's remedy for a CID-keyed CFF program beside this one, `issue12963.pdf` (one
  `/CIDFontType0C` program, 199 places on glyph 0) converts at both targets and passes veraPDF at
  both; without it the document keeps the `.notdef` rows' refusal for a program of another kind.
