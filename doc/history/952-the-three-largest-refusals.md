# 952 — The three largest refusals, and a number that had changed meaning

Date: 2026-09-10 to 2026-09-11. ADR: 0952.
Files: `crates/pdf-model/src/{xmp,appearance,view}.rs`, `crates/pdf-archive/src/table/`,
`crates/pdf-transform/src/archive/`, `tests/archive_corpus.rs`,
`doc/pdf-a-conversion-limits.md` §3.7, §3.9, §4.4.

**A property its own schema does not define** was the largest single blocker anywhere — 273
documents at PDF/A-2b, 272 on that alone. The reading, written up as §3.9 before any code: three
routes exist and two are closed. Correcting the value invents content, because a schema says what
*shape* a value has and not what this document meant. Declaring it in an extension schema
container misrepresents a predefined schema as an extension, in the file itself. What is left is
removal, which loses what the producer wrote — so §3's kind of answer, refused by default.

The validator's own predicate was factored to answer as a list and exported, so the converter
never re-reads the clause and cannot cut a property the validator would have passed. The editor
had to learn what an `rdf:Description` directly inside the packet's `rdf:RDF` is, or a *field*
sharing a property's name deeper inside somebody else's structured value would have gone with it.
The packet is read back: one still holding such a property leaves the document refused rather than
written half-edited.

**An annotation with no flags is not a mechanical fix**, which is the second row's finding. Table
166 defaults `/F` to 0 and Table 167 makes a clear `Print` bit mean the annotation is never
printed regardless of the screen — so the file *says* it is not printed and the only conforming
value says the opposite. That is a loss.

**A constructed appearance is `Stated`, not invention**, on Table 166's writer obligation plus
each subtype's clause. Four cases refuse rather than write something weaker, including a
construction completable only in part — a partial rendering frozen into an archive is not the
appearance the clauses state. An annotation that legitimately draws nothing gets an *empty*
stream, which is a different statement from no stream.

The work found a bug of its own: `decide` consulted `Prepared::obstacle` for two answers but not
for a general `Stated`, so a `Stamp` was reported as converted and then refused ten lines later by
the stage-three net. **The net worked; the report lied.**

**And the sweep's headline number had quietly changed what it meant.** Moving it to a run that
authorises every loss is right for ranking gaps — a loss nobody authorised is a question put to a
user rather than something the converter cannot do — but it turned 98 into 405 at PDF/A-2b, and
most of that is the question being answered. The sweep now reports **both** runs, because they
answer different questions and one silently replacing the other misstates the converter. What a
person who answers nothing gets is 111 and 129, from 98 and 117.
