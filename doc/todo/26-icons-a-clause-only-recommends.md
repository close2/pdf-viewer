# Icons for `Stamp`, `FileAttachment` and `Sound`

Status: not drawn, deliberately, and the verb is why.
Priority: 26
Corpus: 1 document
Clauses: §12.5.6.4 and its three neighbouring tables
Code: `crates/pdf-model/src/icon.rs`

§12.5.6.4 says a processor **shall** provide predefined icon appearances for the `Text`
annotation's standard names, and draws none of them — so `icon.rs` is this processor's own
artwork, and it says so: it is the one module in the tree that is pure invention (ADR 0109).

The three tables covering `Stamp`, `FileAttachment` and `Sound` say **should**. That is the whole
difference, and it is the reason those three are not drawn: reading a silence is not reading the
sentence it sits in, and four tables were read as one silence for a hundred and nineteen sessions
before anybody checked the modal verbs.

Taking this means inventing three more sets of artwork on a *recommendation* rather than a
requirement, and saying so as loudly as `icon.rs` already does. Worth doing for the one corpus
document only if the artwork can be argued from the clause's own descriptions.
