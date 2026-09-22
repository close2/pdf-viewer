# 1187 — An encryption cannot be carried into an archive, and its claim can

Status: accepted and **built**. Session 1175.
Context: `crates/pdf-transform/src/archive/protection.rs` (new),
`crates/pdf-transform/src/archive/{decision,prepare,report,rewrite,mod}.rs`,
`crates/pdf-transform/tests/archive.rs`.
Answers: `doc/todo/66`'s *File structure and encryption* family and `doc/pdf-a-mitigations.md`
section 2, which this moves from *catalogued* to *built* for `file-structure/no-encryption` and
`file-structure/crypt-filter-is-identity`.
Builds: `doc/adr/0947` (three stages, and the rule that nothing changes which no failed requirement
asked to change), `doc/adr/1007` (the signature statement this is the second instance of),
`doc/adr/1161` (§7.6 on the way *out*, which is the direction this one is not).
Clauses: ISO 32000-2 §7.6.1, §7.6.2, §7.6.4.1, §7.6.4.2 (Tables 20, 21, 22), §7.4.10;
ISO 19005-2 sections 6.1.3 and 6.1.7.2; ISO 19005-4 sections 6.1.3 and 6.1.6.2.

## 1. What was refused, and why it was the smallest refusal in the catalogue

Two requirements carried one sentence saying the removal "is not built". RFC 0006 section 5.4 calls
decrypting on the way out the easiest requirement in the whole conversion, and the measurement
agrees: the act was already performed and nobody had named it.

§7.6.2 makes encryption a property of the **file** rather than of anything in the object graph —
strings and stream data are enciphered on their way to disk and deciphered on their way back. So a
document this tree has opened is already a decrypted object graph, and the conversion's own
serializer writes a new file from those objects with a fresh trailer that holds no `/Encrypt`. The
two requirements were failing on a source and passing on an output that already existed.

What was missing was a **decision**: `doc/pdf-a-conversion-limits.md` section 3.5 classes this *Ask*
and never a default, so an act nobody had to authorise would have been a loss taken in silence.
`Loss::Encryption` (the word `encryption`) and `Rewrite::EncryptionRemoved` are what name it.

## 2. What is actually lost, which is not a byte of the document

Nothing in the document changes. Not a mark, not a string, not a byte of any stream's decoded data;
the output holds exactly what the source held. **What is lost is the file's requirement of a key**,
and with it §7.6.4.2's Table 22 flag word — the producer's *no printing*, *no extraction*, *no
modification*.

Both parts forbid encryption outright and neither offers any way to keep it, so no predicate
rescues the requirement: `doc/pdf-a-mitigations.md` section 2 already ranked its departure **C** on
the argument that an archive nobody can decrypt is not an archive. That ranking stands and this
build does not touch it.

## 3. The decision this ADR exists for: the enforcement goes and the assertion stays

The catalogue names a `preserve` beside the `discard` and this build is it. Table 22's flags are a
*statement* about the reader — that is the whole of what an unencrypted file cannot make — and a
statement survives where an enforcement cannot. So `SourceProtection` reads the source's `/Encrypt`
before the conversion writes anything, and puts what it found in two places:

- the conversion report, in one line per document naming the handler (§7.6.1's `/Filter`), Table
  20's `/V`, Table 21's `/R` and every operation Table 22's word withheld;
- the output's own `xmpMM:History`, under `PERMISSIONS_NO_LONGER_ASSERTED` — *this document's
  permission flags are recorded, not enforced* — because a report is something somebody may not
  have kept and the archive is the thing that is meant to outlive them.

Three things this deliberately does **not** do, each for the same reason:

- **It does not restate the flags in a scheme of its own.** The words are Table 22's operations and
  the bit numbers are printed beside them, so a person reading the report twenty years from now can
  check it against the clause rather than against this program.
- **It does not read the reserved positions.** Table 22 says a reader "shall ignore this bit" of
  position 10, so a file's value there asserts nothing and nothing is reported about it. Which of
  the rest are meaningful depends on the handler's revision, and that reading has one home already
  (`pdf_syntax::crypt::Permissions`), so none of it is made twice.
- **It does not enforce anything.** §7.6.4.2's flags are the *document's* restrictions over its
  reader, which `CLAUDE.md`'s own section makes the reader's to switch off; a converter that
  refused to archive a document because its producer said *no extraction* would be enforcing
  against the only person who asked for the archive.

## 4. Why `crypt-filter-is-identity` is answered by the same act and not by a rewrite

ISO 19005-2 section 6.1.7.2 and ISO 19005-4 section 6.1.6.2 forbid the `Crypt` filter unless its
decode parameters name `Identity`. §7.4.10 makes that filter the place a stream states which crypt
filter enciphers it — a statement that means nothing in a file with no encryption dictionary, and
one whose *conforming* spelling (`Identity`) the requirement permits to stay. It is the per-stream
half of the same act and has no separate answer, which is exactly what the catalogue said: a
separate key for it would let an operator authorise half a decryption, and that is not a state a
file can be in.

## 5. What a password still costs, and what it does not

A source whose user password is not to hand cannot be *read*, so it never reaches this decision —
`doc/pdf-a-conversion-limits.md` section 2.4, unchanged. A configuration cannot carry the password
because it is per document; the command line's `--password` is where it goes. Nothing here changes
either fact.

## 6. What this leaves

`file-structure/permissions-dictionary-keys` was already `Mechanical` (ADR 1007) and stays so: a
key outside Table 263 names a permission handler ISO 32000 does not define, which is a different
act from this one and remains a separate row.
