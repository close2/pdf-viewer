# 1048 — Which half of a contradiction a sweep can settle, and which half it may not silence

Session 1031. Status: **accepted**. Changes four of `tools/conformance`'s sweeps — `blockers`,
`tables`, `counts`, `overstated` — and one ledger row. Establishes the rule a round resolving a
sweep's contradictions works by.

`§N` is ISO 32000-2 and nothing else.

## 1. What the round found

Session 1025 woke twenty dormant instruments and gave four of them the verdict *act* on the
strength of a contradiction each reported inside this tree's own claims: 30 blocker sentences
expired by the ledger's own account, 7 denials a table contradicts, 5 places counting one family
twice, 2 unmarked overstatements. Read one by one, **42 of the 44 were the instrument's, not the
tree's.** One was a live defect (§4) and one is left standing (§5), which also names the reading
that keeps §4's own pair printed.

That ratio is the finding, and it is not an argument for switching anything off. Each of the four
instruments was reporting a population that is *mostly* legitimate prose, which is ADR 0249's
ratio argument and the reason none of them is a gate. What had gone wrong is narrower and is what
this ADR fixes: **the headline number a round is asked to move was measuring a grammar rather than
a claim.**

## 2. The rule

> **Resolve a contradiction by finding which half is false. Where neither is, the instrument's
> rule is what is false — and it is corrected by making it see a distinction, never by making it
> quieter.**

Three consequences bind any round that touches these sweeps:

- **A demotion is arithmetic or it is not a demotion.** A hit may be taken out of the headline
  number only where the *program* can say why it is not a contradiction — a rung whose definition
  is "the denial is about another term", a clause the ledger holds no rows below. A demotion that
  rests on a reading belongs in the reading, beside the hit, printed.
- **Nothing is filtered.** Every hit this round demoted is still printed, with the mark that
  demoted it. A sweep that stops printing a population stops being calibratable against it.
- **Every rule change carries a calibrating pair.** The test that asserts the new rule fires also
  asserts that the sentence one word away still does *not* — a plant on each side (trap 13). Four
  such pairs were added.

## 3. What each instrument could not see, and can now

**`blockers` — `while` and `until` are conjunctions.** `CLAIMS` mixes five phrases that assert a
dependency (`needs §`, `waits on`, `blocked on`, …) with two that merely join two statements. 27
of the 30 "expired" sentences matched only a conjunction, and every one of them was contrasting
what two clauses say — "§9.9.1 says outright … while §9.2.4 is an inference". The other three were
past tense. `CONJUNCTIONS` counts the two apart and `PAST` marks the tense where the auxiliary is
adjacent to the phrase, which the module documentation had said no grep could see; it can see
*these*, because `were blocked on` is three characters of lookbehind. **The reading list is
0 in both populations** — every present-tense dependency blocker in this tree names a clause that
still owes something.

**`tables` — a negation with a noun of its own.** `denies` counted a negation anywhere in the
reach before the key. "Table 185 states no such ordering for `/InkList`" denies an *ordering* and
says in the same breath that the table states the entry. Requiring the negation to be the word
immediately before the key separates that from "Table 119 gives a Type 0 dictionary no
`/FontDescriptor`", which is the form every real denial in this tree takes. 7 → 6.

**`counts` — a clause with no family cannot have one counted twice.** Four of the five pairs were
under leaf rows, where the sweep already reports each number on its own as `Childless` — it cannot
count what they count, and two things it cannot count cannot disagree *about this ledger*. The
second numbers were a raster row, a row of one of the standard's own tables, and an erratum's row.
5 → 1.

**`overstated` — two by construction, one by ellipsis.** `Rung::Elsewhere` *is* the sweep saying
the denial names another table or another entry, so it demotes itself; that was already true and
was not counted. And §7.3.8 says "Table 5's entries are read where they are used — `/Length` … by
the parser, `/DL` nowhere": the part asserts as a whole and excepts one term by an ellipsis its own
verb supplies, so the row asserted the entry it excepted and its child agreeing read as a
contradiction. `excepted` takes it out of the assertion.

## 4. The one live defect: `/Configs` is Table 98's

§8.11.4.3's row opened "Table 99, all of it but two entries" and then said "`partial` for one
entry now: `/Configs` is unread", naming three entries in the sentence after it. Table 99 —
*Entries in an optional content configuration dictionary* — states eleven entries and `/Configs`
is not among them: it is **Table 98's**, the optional content properties dictionary's, and
§8.11.4.2's row owns that table. The debt is owed under §8.11.4.3 all the same, because this
clause's own opening sentence is what says what those configurations are for — "Configs lists
other configurations that may be used under particular circumstances" — but the row had it inside
an inventory of Table 99, which is the ninth sweep's standing shape: an entry filed under the
table its value points at. Corrected, and the count with it.

`--bin tables` could not see this one, because the row never writes the two nouns adjacently;
`--bin overstated` found it by pairing a parent's assertion about Table 99 against a child's
denial of an entry, which is the pairing that sweep exists for.

## 5. What is left standing, and what would settle it

- **§9.7 counted as 16 and as 3.** 16 is the family's descendants and is right; 3 is "[t]hree rows
  in this family and §9.3.7 carried one or other half of that pair", a *subset*, which is not a
  claim about the family's size at all. Settling it in the program needs the sweep to tell a
  cardinality from a restrictive predicate — a count followed by a finite verb with an object —
  and that is grammar this checker has no parser for. Left, named.
- **§8.11 against §8.11.4.3, after §4's correction** — the hit that found the defect, and it
  stays. The parent reads Table 99's `/Locked` and the
  child says `/Name` and `/Creator` are unread: a table read in part, both true. The mark cannot
  fire because the parent's asserting part enumerates no entries of the table, which is the
  "capability read in part with no table to divide it" the module documentation already leaves to
  the reader deliberately.
- **`--bin inapplicable`'s 253.** This is a *sorted reading list* by design — the module's own
  argument for rarity over a stop-list is correct and is not touched here. The 27 terms at three
  witnessing files or fewer were read against their cousins and every one resolves as two rows
  about different sentences: `LineHeight` under a font's metrics, `Ruby` the element against
  `RubyAlign` the attribute, `PrintField` named by §14.8.5.4.1 only in order to *exclude* it.
  Nothing was changed and nothing should be.
