# 1028 — A permission is not a debt, and an optional entry is not a residue

Ledger slot of batch 1026–1031, five siblings in one worktree. Contract: act on `--bin permitted`'s
50 and `--bin owed`'s 130. Gate: `cargo test -p conformance`.

**1. `permitted`'s 49 read; 14 are aggregates (ADR 1035 section 4), 2 of the 35 leaves settled, 33
left.** The flagged optional entry is the row's residue in two cases and is not in 33 — the
instrument's own noise, stated so no later round re-reads it: `permitted` classifies by every table
entry a note **mentions**, not by the one it says is **owed** — §8.6.5.9 lands there for naming
`/Intent` while owing ISO 18619, §12.2 for `/PageLayout` while owing the whole print half.

**2. Four rows to `implemented`, each on a clause sentence.**
- **§14.11.3.** `/MN` is "[a]n arbitrary name identifying the type of printer's mark" and nothing
  says what a reader does with one; `/MarkStyle` is "a text string representing the printer's mark
  in human-readable form and suitable for presentation to the user" — what the string contains, no
  `shall`, `should` or `may` on a processor. `/Colorants` is the same shape, its `shall` a writer's.
  The clause's four prose `shall`s are executed.
- **§14.11**, forced by ADR 1035 section 5's `AggregateWithoutDebt` gate the moment §14.11.3 moved —
  its first firing; the note already said §14.11.3's was what held it, alone.
- **§8.11.4.3.** Residue was `/Configs`, `/Name`, `/Creator`. The clause's sentence is a permission —
  "Configs lists other configurations that may be used under particular circumstances" — beside the
  `shall` this tree obeys for `/D`; the other two label what it would produce. `/Order`'s
  empty-array default was checked in `optional_content.rs`, not assumed.
- **§11.6.4.4** (found through `owed`): four `shall`s, all four executed, and the residue it was read
  as carrying is §11.6.4.3's.

**3. `owed`: seven notes whose debt was a pointer now state it in sentences.** §8.11, §8.11.1,
§8.11.4 and §8.11.4.1 each named `/Configs` as this family's debt and no longer may; what they carry
is written out — §8.11.4.4's `User` and `Language` answered `Recommendation::Unanswerable` rather
than by "otherwise OFF", and §8.11.4.5's "the corresponding dictionaries shall be reapplied". §7.7's
"what the subclauses still name" and §12.4's "what a *window* still owes any of the three" now
enumerate three children apiece.

**4. §12.3.5.2 owes more than it said.** Its three named entries state no processor requirement and
`/Thumb`'s presence is read already. Never named: six bullets restrict a folder name, then "An
interactive PDF processor may choose to support invalid names or not. If not, an appropriate error
message shall be provided." `collection.rs` applies none and reports nothing. Stays `partial`.

**5.** `\uXXXX` escapes on ledger lines 4378 and 4386 were blocking tier 1 for the whole worktree;
replaced with the characters. Left alone: `pdf-model/tests/silent_fonts.rs`'s `zz_probe` fails
`cargo fmt --all --check` and is a sibling's. `partial` 185 → 181; `permitted`'s optional class
49 → 46; `owed`'s list 106 → 103. Conformance green, every new quotation verbatim.
