# 1087 — A prefix is content in exactly one place, and that place is a spelling

Status: accepted. Session 1073.
Context: `crates/pdf-model/src/xmp.rs` (`respell`, `RequiredPrefix`,
`WriteError::PrefixMeansSomethingElse`), `crates/pdf-archive/src/table/metadata.rs`
(`ContainerField`, `extension_container_fields`, `REQUIRED_PREFIXES`),
`crates/pdf-transform/src/archive/{decision,prepare,rewrite}.rs`,
`crates/pdf-transform/tests/archive.rs`.
Builds: `doc/rfc/0007` section 2's vocabulary, at `Decision::Mechanical`.
Clauses: ISO 19005-2 sections 6.6.2.2 and 6.6.2.3.3 (Tables 3–6); ISO 16684-1 sections 6.2 and 7.5;
ISO 32000-2 §14.3.2.

## 1. What was measured, and what was taken

`crates/pdf-transform/tests/archive_corpus.rs` ranks the requirements that stop a conversion by how
many corpus documents each stops. Its top row at PDF/A-2b — the largest of the six targets — was
`metadata/extension-schema-container-fields`, refused with `Because::NotBuiltYet`, which is a gap
named rather than an argument closed. That is what this round took.

Reading the nineteen witnesses one by one is what made the answer obvious, and a count could not
have: **they are two different faults wearing one requirement's name.** Fifteen state a container
that leaves a field out — no `pdfaSchema:schema`, no `pdfaProperty:category`. Four state *every*
field each table names, in the field namespace that table gives it, with the producer's own value,
and spell them `nonpdfaSchema:`, `nonpdfaProperty:`, `nonpdfaType:`, `nonpdfaField:`.

## 2. Why the second is `Mechanical` and not a loss, an ask or a refusal

This module's own opening says a prefix is not a name: ISO 16684-1 section 6.2 makes an XMP name a
namespace URI and a local name, and a file may bind any prefix it likes. ISO 19005-2 section 6.6.2.2
states the exception in as many words — a prefix means nothing **except** where one is identified as
required — and section 6.6.2.3.3's four tables each identify one. So in this one place the spelling
is part of what the clause requires, and a packet can be wrong about it while meaning exactly the
right thing.

Which makes the correction a *structural fact*, RFC 0007 section 2's test for `Mechanical`: the
namespace URI, the local name and the value are all already in the file, and what changes is the
prefix token. `xmp::respell` moves those tokens in the producer's own bytes — the declaration that
bound the old prefix along with the names that used it — so the packet gains no attribute, loses
none, and every other byte crosses unchanged. Nothing is invented, so nothing is asked; nothing is
lost, so no authorisation exists to withhold. The diff on a corpus witness is five prefix tokens and
one `xmlns:` name.

**Three refusals keep it honest**, each a way the move would change meaning rather than spelling:
a required prefix the packet already binds to another namespace (taking it over would move every
name spelled with it), a required namespace carried by a *default* declaration (there is no token to
move, and writing one would be an edit of a different kind), and a packet this writer cannot
tokenise. The move is read back out of the packet afterwards and a container still faulting refuses
the document, which is the rule the property removal beside it already follows.

## 3. Why the first is still refused, and what it is waiting for

A field the packet does not state is content nothing in the file holds. `doc/pdf-a-mitigations.md`'s
entry for this requirement says what is missing — a name for a schema, a description of what a
property means, or the category saying whether a value came from the document or from outside it —
and the authoring exclusion is what forbids supplying any of them. Its two honest answers are an
operator's own `supply` (the configuration states the field once, for a schema the archive owns) and
a `discard` that loses the container and, with it, the properties it described. Neither is built,
and the refusal names the field rather than the subclause so that a reader can tell the two faults
apart.

**The split is `pdf_archive`'s to make, not the converter's.** `ContainerField::misspelled` is
decided by the same predicate the requirement's own row is, which is why the converter cannot
respell a name that row would have passed — `properties_outside_their_schema`'s construction, for
its reason.

## 4. Four writers over one packet now

`super::prepare::Edited` carried three and carries four: section 6.6.2.1's header attributes come
off first, section 6.6.2.3.1's misused properties come out of what that left, section 6.6.2.3.3's
prefixes move in what *that* left, and section 6.6.4's identification schema is restated into the
last of them. Each starts from what the one before it left, because a writer reading the producer's
original would silently undo its predecessor.

## 5. What it did, measured

Over the veraPDF corpus at PDF/A-2b: the requirement stopped nineteen documents and now stops
fifteen; four convert to a written file that did not, and they convert **with nothing authorised**,
which is what `Mechanical` means. Each was re-opened and held to the target again before being
written. No other target moves: neither subclause binds under part 4.
