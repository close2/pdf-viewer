# 1258 — An attachment goes by what names it

Status: accepted and **built**.
Context: `crates/pdf-transform/src/archive/{sites,prepare,decision,rewrite,report,mod}.rs`,
`crates/pdf-transform/tests/archive.rs`.
Answers: `doc/todo/66`'s **embedded files** family — the `discard` at
`embedded-files/embedded-file-is-itself-pdfa` and
`embedded-files/embedded-file-is-itself-pdfa-in-the-plain-profile`, which
`doc/pdf-a-mitigations.md` section 11 named as the last of four answers and no version carried out.
Builds on: `doc/adr/1099` (the reference is what is removed, and nothing hunts for the object),
`doc/adr/0947` (a decision that cannot be carried out is a refusal rather than a plan),
`doc/rfc/0007` section 4.6 (the four answers, in order of preference).
Clauses: ISO 32000-2 §7.7.4 (Table 31), §7.9.6, §7.11.3 (Table 43), §12.5.6.15 (Table 184),
§14.13.2; ISO 19005-2 section 6.8; ISO 19005-4 section 6.9.

## 1. The last answer of four, and why it was the one left

`doc/rfc/0007` section 4.6 ranks the answers to an embedded file a target will not admit: attach it
unchanged, which PDF/A-4f and -4e do by the rule not binding at all; derive a conforming PDF from
it with a declared tool, built since session 999; keep it by departing, which the owner's XML case
asks for; and remove it. The first three are all *ways of keeping the file*, so the fourth was the
one nobody had a reason to build first — and it is the one a plain PDF/A-4 or PDF/A-2 target needs
when no tool exists and no departure is wanted.

## 2. What is removed is every reference, not the object

ADR 1099's rule, applied a second time. The serializer copies what the converted document reaches,
so a file specification nothing names is one the output does not hold, and the embedded file stream
under it goes the same way. Nothing searches for the object and nothing edits it.

**Two entries rather than one**, and §7.7.4's Table 31 is why the name tree cannot simply be left:

> All File Specification dictionaries referenced from this name tree shall contain an EF key whose
> value is a dictionary which contains either an F or UF key whose value is an embedded file
> stream.

A tree entry left behind would be a defect this conversion wrote. §14.13.2's `/AF` array goes the
same way, and an array with nothing left in it is removed rather than written empty: an object with
no associated files is what an absent entry already says.

## 3. The name tree is two shapes of one key, and both are walked

§7.7.4's name dictionary states `/Names` as a **dictionary** of trees; §7.9.6's leaf states `/Names`
as an **array** of alternating name strings and values. So one key is read two ways and which it is
decides whether the walk descends or prunes. A removed leaf takes the name beside it, because an
odd-length remainder is a tree no reader could read.

§7.9.6 lets a node be an object or be written directly inside its parent, and producers mix the two
freely — the shipped fixture writes the whole tree inside the catalog. `super::sites` therefore
records the **outermost object** on the path to each node holding a removed leaf (or the catalog,
where nothing but the catalog has been crossed), and the rewrite descends from there through
whatever that dictionary states directly. A node that is an object of its own is reached by the
object walk and recorded as itself.

## 4. Two refusals, each a clause rather than a limit

- **A specification a page still reaches.** §12.5.6.15's Table 184 makes a file attachment
  annotation's `/FS` required, and §12.6.4.4's embedded `GoTo` names one the same way. Taking such a
  specification out of the name tree would leave it in the file, reached from a page, with the
  requirement still failing — so the document is refused by name rather than half-converted and
  caught by `doc/adr/0947`'s third stage. Removing that annotation is a different loss, over the
  marks it draws, and it is not this one.
- **A specification the name tree writes in place.** The finding names an object, and a leaf written
  directly inside its parent has no object number for the removal to match on. Guessing which
  name-tree entry the validator meant is the act `doc/adr/0947`'s first rule forbids.

## 5. A derivation that was run and did not answer now refuses rather than falling through

Before this, the two rows were in `REFUSED_BY_NAME`, and a `derive` whose tool failed fell through
to that refusal by accident. With a `Loses` row behind them it would have fallen through to an
*unauthorised loss* instead — offering the operator a discard they did not ask for.
`super::configured` now refuses the site outright where the report holds a derived row whose outcome
is not `Attached`: `doc/rfc/0007` section 4.2 does not trust what comes back, so a tool that failed,
declined or produced something other than what it promised has answered nothing. The behaviour the
old accident produced is now the behaviour an argument produces, at every `derive` site rather than
at these two.
