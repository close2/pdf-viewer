# Round 1131 — §14.7's structure hierarchy, and its pronunciation hint

Contract: the two remaining Clause-14 `partial` rows, §14.7 and §14.7.2. Decide each against the
clause; do not force a move.

## §14.7.2 — implemented (a permission is not this clause's departure)

§14.7.2's own `shall`s are the structure-hierarchy ones — the root located by `/StructTreeRoot`,
"[t]he K entry shall specify the immediate children of the structure tree root, which shall be
structure elements", a no-`/Type` dictionary is an element — and every one is executed
(`structure::Tree`, read on demand, bounded, reported on truncation). Every table entry the
reader does not itself consume is answered in the settled row of the clause that owns it:
`/Namespaces` -> §14.7.4 (a writer-completeness rule per Issues #396/#93), `/AF` -> §14.13,
`/ParentTreeNextKey` a writer entry now written, `/PhoneticAlphabet`/`/Phoneme` -> §14.9.6.

The pronunciation hint is not §14.7.2's departure to record: §14.7.2 only *defines* the two
Table 355 entries; the obligation to process them is §14.9.6's, whose whole normative content is
a permission — "A PDF processor is not required to process pronunciation hints." — already
`implemented` on that permission (§10.7.2 flatness is the precedent). §14.7.2 declines no
own-`shall`, so not `departed` (ADR 1119 needs a declined `shall` inside the clause); it carries
no undecided debt, so not `partial`. -> **implemented**. No ADR 1133 owed — decision lives in §14.9.6.

## §14.7 — implemented (assembly is built, not host-blocked)

ADR 0865 §2's assembly residue is a completed writer capability: the `Host` trait has two real
implementors, `split.rs`'s `Piece` and `merge.rs`'s `Merge`, exercised by the split/pages/merge
tests. The reader is fully built. With §14.7.2 settled, all 18 sub-clauses settle; a `partial`
heading over none-owing children is `AggregateWithoutDebt` (ledger.rs:932). -> **implemented**.

## Census (trap 8) and calibration (trap 13)

Corpus (`doc/pdf.js/test/pdfs` + `doc/` specs, raw scan): **0** documents state `/Phoneme`,
`/PhoneticAlphabet` or `/PronunciationLexicon` (PDF 2.0, absent from the pre-2.0 corpus; 106
state `/StructTreeRoot`). The assembly is a writer path — zero readers exercise it. The silence
being real, the calibration is a fixture,
`structure::tests::a_phoneme_is_read_past_and_the_language_and_unicode_text_answered_in_its_place`:
an element with `/PhoneticAlphabet /ipa /Phoneme (tuh-MAY-toh)` answers its `/Lang` (fr-FR) and
`/Alt` (tomato); the phoneme is none of the answers, and no `/Phoneme` code path exists to return
it. Rows: §14.7 and §14.7.2 partial -> implemented, §14.7.2 note rewritten to what-is.
