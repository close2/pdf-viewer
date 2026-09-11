# 966 — Two owed rewrites, a clause read one sentence too early, and three reasons written for the wrong edition

Date: 2026-09-11. ADR: 0973.
Files: `crates/pdf-transform/src/archive/{decision,prepare,rewrite,sites}.rs`,
`crates/pdf-transform/tests/archive.rs`, `doc/pdf-a-mitigations.md` §4.1, §4.2, §13.3 and §13.3.1,
`doc/adr/0973`, `doc/history/966`.

The PDF/A converter stream, continuing session 962 over what `doc/pdf-a-mitigations.md` §13.3 had
left.

**Two built**, each out of `REFUSED_BY_NAME` — and one of them was not on the owed list at all,
because session 962 had moved it into the fence.

What is worth carrying:

- **`graphics/one-destination-profile-per-output-intents-array` splits on a question the file
  answers.** ISO 19005 6.2.3's own note says where several output intents come from — a file
  conforming to ISO 19005 and to PDF/X or PDF/E at once — and such a file usually carries the *same*
  profile twice. So the profiles are decoded and shared only where they are the same bytes under the
  same stream dictionary: the object that goes was a copy and no entry changes what it refers its
  colours to. Different bytes are two destinations, one of which a rewrite would discard, and that
  half keeps its refusal.
- **`graphics/spot-colourants-appear-in-the-colorants-dictionary` came back out of the fence**, and
  the reason is the one to remember: 6.2.4.4 has a *second* sentence. It requires every `Separation`
  array in a file naming one colourant, expressly including the arrays written inside a `Colorants`
  dictionary, to state the same alternate space and tint transform, compared as PDF objects. So where
  the file states one for the colourant, the missing entry is **determined**: the producer's own
  array, copied. Two sessions had read only the first sentence and reasoned about deriving a
  one-input transform from an N-input one, which is the hard route and is still owed for a file that
  defines the ink nowhere else.
- **A claim that a refusal is a decision decays too.** §13.3 was built on the lesson that "this is
  only unwritten work" is a claim that decays. The reverse is now the standing example: this row was
  argued out of the owed list four rounds ago, correctly given what had been read, and a better
  reading put it back.
- **A clause is not read until its neighbours are.** The general form, and it cost two rounds here.
- **Three rows justified their rewrite with an ISO 32000-2 sentence while binding a part 2 target**,
  whose base standard is ISO 32000-1:2008. None wrote a wrong file — two of the three rules are
  stated identically in both editions, and in the third the ISO 19005 clause was doing the work —
  but the `/CharSet` and `/CIDSet` rows' *only* stated ground for removing rather than recomputing
  was a PDF 2.0 deprecation their one target never sees. ADR 0973 has all three and the model
  already in the tree (`sites::cid_to_gid_maps`, which refuses by target in code).
- **`§7.7.3.3` was cited eight times for a rule that is 7.7.3.4 in both editions.** Nothing behaved
  differently; a reader checking the claim would have been sent to *Page objects* every time.
- **The first rewrite in this verb that reaches an object that is an array.** A `DeviceN` colour
  space is as often its own object as a value inside a resource dictionary, and `Rewriter::rewrite`
  had arms for a dictionary and a stream only.
- **A placement is proved before it is promised.** The colourant preparation runs the rewriter's own
  placement on a copy and refuses unless every colourant it named was placed — without it the verb
  would have written a file still failing the clause it claimed to answer.
