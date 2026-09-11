# 971 — The box that can go where no value can be written

Date: 2026-09-11. ADRs: 0982, 0992.
Files: `crates/pdf-transform/src/archive/jpeg2000.rs` (new),
`crates/pdf-transform/src/archive/{decision,prepare,rewrite,mod}.rs`,
`crates/pdf-transform/src/bin/quorra-transform.rs`, `crates/pdf-transform/tests/archive.rs`,
`doc/pdf-a-mitigations.md` §4.2, §4.5, §13.1, §13.3, §13.3.1,
`doc/pdf-a-conversion-limits.md` §4.10, `doc/adr/0982`, `doc/adr/0992`, `doc/history/971`.

The PDF/A converter stream, continuing sessions 962 and 966 over the rows those two had moved
*out* of `doc/pdf-a-mitigations.md` §13.3's owed list.

**One built** — the two JPEG 2000 colour-box rows, together, as an authorised `discard`. **One
re-examined and upheld** — the duplicate-profile pair, whose refusal sentence is now sharper in
both halves.

What is worth carrying:

- **"There is no mitigation" and "there is no *lossless* mitigation" are different claims.** §4.5
  was argued through two rounds entirely inside the question *what value may be written into a
  `colr` box* — and every answer to that is a choice, which is true and was never the whole
  question. Removing a box writes no value. The sentence that licenses it is the one immediately
  after the method's, in both parts: a conforming processor shall use only the selected colour
  space and shall ignore all the other colour space specifications. So what the rewrite removes is
  what the target's own subclause directs a processor to ignore.
- **The tell was in the entry's own four answers.** *Mitigation — none* beside *From a
  configuration — nothing* is the shape this mistake takes: it asserts that no operator could
  authorise anything here, which is a much stronger claim than the argument above it supports. An
  entry with those two lines is worth reading again.
- **A clause's neighbours can be in another standard.** ISO 19005 says which specification a
  processor *uses* only where one carries an `APPROX` of `0x01`. For the file that marks none —
  which is the file written to ISO/IEC 15444-1 as part 1 requires, since part 1 reserves that field
  and sets it to zero — the box a reader uses is settled by **part 1's** own rule that a conforming
  JP2 reader ignores every colour specification box after the first. The clause that makes the row
  fail and the clause that makes it fixable are in different documents.
- **It is an Ask, not an owed rewrite, and §7.4.9 is why.** The removed specifications are a
  fallback chain: a processor that cannot use the one that stays must descend to the next, and
  failing that to a device space. Nothing on the page changes for a processor that can use the
  survivor — the corpus witness renders byte for byte identically — so the loss is small, real and
  nameable. `--authorise jpeg2000-colour-fallback`.
- **Two shapes keep their refusal and now have their own sentences**: a file whose only
  specification states a forbidden method, where there is nothing to remove and nothing to fall
  back to; and the second row's second sentence, the selected specification's own ICC profile,
  which is the profile-replacement case.
- **A rewrite that reaches a new *shape* of object does not site a failure reported elsewhere.**
  Session 966's `SpotColorantEntry` was the first rewrite here to edit a colour space array, so the
  duplicate-profile rows' siting refusal was worth asking again. It holds: that rewrite reaches an
  array because **its own finding names that array's object**, and these findings name the content
  stream that *used* the space — `page 1, ICCBased`, no object at all. What sites a rewrite is the
  finding naming the object, and nothing else.
- **And the value half of that refusal turned out stronger than it had been written.** §8.6.5.7
  says the implicit conversion's conditions "cannot be specified in PDF" and that it "plays no part
  in the interpretation of PDF colour spaces" — so whether an `ICCBased` CMYK space behaves as
  `DeviceCMYK`, and therefore whether non-zero overprint mode reaches it, is a decision the base
  standard assigns to the processor. Writing `/DeviceCMYK` takes that decision away from the
  processor rather than restating anything the file said.

Two working traps, in ADR 0992 because this session does not own `doc/habits.md`: adding a `use`
to a file turned seven of a neighbour's fully-qualified lines into `-D warnings` failures on code
the round never touched; and two gate failures named files in crates this session was forbidden to
edit and passed on a later run with nothing done, which is what a sibling mid-edit looks like from
the outside.
