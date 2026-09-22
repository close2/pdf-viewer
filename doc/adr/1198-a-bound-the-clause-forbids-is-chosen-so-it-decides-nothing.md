# 1198 — A bound the clause forbids is chosen so that it decides nothing, and the flag beside it is checked where its sentence is about

Status: **accepted**.
Context: `crates/pdf-model/src/appearance.rs` (`MAX_FIELD_ANCESTRY`, `Field::read`),
`crates/pdf-model/src/submission.rs` (`chosen`),
`crates/pdf-model/examples/field_flag_census.rs` (`parent_links`),
`crates/pdf-model/tests/variable_text.rs`, `crates/pdf-model/tests/submission.rs`,
`doc/conformance/ledger.toml` §12.7.4.1.
Builds: ADR 1119 (the word `departed`), ADR 1189 (a departure re-grounded on a measurement),
ADR 1062 (where a submission's decision is taken), ADR 0120.
Clauses: ISO 32000-2 §12.7.4.1 (Tables 226, 227), §12.7.6.2, §7.7.3.4.

## 1. Two requirements of one subclause, and neither was where the row said

§12.7.4.1's row read `partial` and named one thing: a depth bound on the `/Parent` walk that the
clause forbids in so many words. Reading the subclause found a second, and re-measuring the first
found that its recorded reason was wrong about this disk.

## 2. The bound: forbidden by the clause, required by principle 3, and its number was costing

> An interactive PDF processor shall not limit the range of inheritance for field dictionaries

The sentence admits of no reading under which a bound is conforming. A bound exists anyway, and
its reason is not convenience: a `/Parent` chain in a hostile file can be a cycle, and
`CLAUDE.md` principle 3 makes an explicit resource budget the answer to pathological input rather
than an optimisation. So this is `departed` in ADR 1119's sense — a requirement decided against
with its cost recorded — and what makes it honest is that reaching the bound is *named*
(`Refusal::NotDerivable`, "its field's /Parent chain is longer than this crate follows") rather
than read as a field that states nothing.

**What was not recorded was the cost, and the cost was not nought.** The bound was 32, on the
stated reason that no legitimate form comes near it. `examples/field_flag_census` now walks the
chain to a bound of its own and prints the deepest one seen: over the 90 763 documents reachable
from `corpus-cache`, `doc/corpora` and `doc/pdf.js/test/pdfs`, the deepest chain is **32 links**
and **25 widgets over two documents reach it** — `PDFBOX-2261-0.pdf` and `TIKA-1590-1.pdf`, whose
fully qualified names carry thirty-odd components each and are hierarchies a producer wrote rather
than cycles. At 32 the bound was refusing real fields.

**So the number is chosen so that it decides nothing about a legitimate file**, which is what a
budget owes a requirement the clause forbids: 256, eight times the deepest measurement, with a
cycle still costing 256 dictionary lookups once per field and still refused by name. The
departure is unchanged in kind — a bound remains, and the clause forbids any bound — and its cost
on every document on this disk is now zero.

The census walks to a bound of its own (1024) rather than to this one, deliberately: an
instrument that moved with the quantity it measures could not have found this.

## 3. The flag: Table 227 bit 2, checked at the moment its sentence names

The row said Table 227's `Required` and `NoExport` were "carried only, because §12.7.6.2's
submission is excluded". Submission is built, and `NoExport` has been obeyed in
`submission::chosen` since it was. `Required` was left behind — the shape
`doc/habits/the-ledger-and-claims-about-this-tree.md` calls a reason that names a capability the
tree has since acquired.

> If set, the field shall have a value at the time it is exported by a submit-form action (see
> 12.7.6.2, "Submit-form action").

The sentence is about one moment, and `submission::chosen` is the only code in this tree that is
at it. A selected field with the flag set and no value is now named in what a host is handed.

**Named and not refused**, on three grounds: the clause tells a processor nothing to do about it;
`viewer_host::policy::may_submit` is where a reader decides whether the submission goes (ADR
1062), and it decides better told; and `CLAUDE.md` principle 3's warning is against a refusal
nobody can turn into a question. A field whose value is a stream that would not decode is still
left out with its own sentence and is not named here, because "this reader does not know the
value" is not "the field has none".

## 4. Consequences

§12.7.4.1's row becomes `departed`: every requirement of the subclause is executed except the
bound, which is decided against with the measurement above. No public type changes. The deepest
chain in the tracked 974 is **4 links**, so nothing there was ever near either number, and the
two documents that reach 32 are in `corpus-cache`, which nothing tracked walks — no gate moves
either way. Four fixtures, each planted back (trap 13): a chain one link past the bound, its
control one link inside, and a `Required` field with no value beside one with the flag clear and
one with a value.

## 5. What this does not decide

Whether a cycle detector should replace the bound. It would meet the clause exactly — no limit on
range, a refusal only where the chain is not a chain — at the price of a set per field on a path
`CLAUDE.md` principle 2 cares about, and nobody has measured that price. Until somebody does, 256
is the bound and this is its argument. Nor does it decide what a host *shows* for a `Required`
field with no value; the sentence reaches `Submission::owed` and stops there.
