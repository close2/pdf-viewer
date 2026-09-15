# 1104 — §12.8.2's second step reaches a reader, and Table 258's rights rank beside Table 257's

Session 1090. Status: **accepted**. Closes the debt ADR 1043 §4 named and ADR 1049 §3 repeated —
"[n]othing in the *program* reports any of it" — by putting §12.8.2.2.2's and §12.8.2.3's second
step into `viewer_core::notes::about`, and builds the second ranking that debt sat beside: Table
258's rights, through `revision::Comparison::against_usage_rights`. Moves five §12.8.2 ledger rows, and takes a
descriptor out of `FileBytes::prefix`.

`§N` is ISO 32000-2 and nothing else.

## 1. Two clauses, one shape, and the half that was computed and never said

§12.8.2.2.2 and §12.8.2.3 each state validation as two steps, and each states them in the same
words: verify the byte range digest, then examine what changed against the transform parameters.
`Signature::integrity` has been the first step since ADR 0215 and `pdf_signature::revision` has been
the second since ADR 1043 — reached, in that ADR's own sentence, "by `pdf-signature`'s
tests and by no host".

That is the fifth shape `doc/habits/the-ledger-and-claims-about-this-tree.md` warns of: a capability
that reached the crate and never reached the program. What closes it is not a channel — `notes` has
carried §12.8's sentences since the three-hundred-and-seventy-seventh session and `Event::Reported`
has crossed the confinement and the C ABI since ADR 0445 — it is a round writing the sentences.

**So the boundary does not move.** No `Command`, no `Event`, no `Query`, no ABI entry point,
`QUORRA_EVENT_KIND_COUNT` and `QUORRA_ABI_VERSION` both where they were. `doc/ui-boundary.md`'s own
preference is that a report be said in the vocabulary that exists, and here the counts a C caller
could want are inside the sentences it already reads through `quorra_reports_len` and
`quorra_report`. A struct for them would have been three entry points and a second way to be told
one thing.

**And the launch path does not move either**, because ADR 1044 already decided where a claim about
the *file* is answered: `notes::about` sits behind `Command::Report`, held by `Open::about`'s
`OnceCell`, and a host asks for it after the frame it presented. A signature decides nothing page
one draws, and neither does a comparison of two revisions.

## 2. What fires, and on whose condition

Trap 11's rule, and every condition here is a clause's rather than a taste:

- **The comparison** runs where §12.8.1's signed range stops short of the file — which is where an
  update was appended after signing and a second state exists — or where §12.8.6's permissions
  dictionary names the signature in `/DocMDP` or `/UR3`, because a transform stating what may change
  is owed an answer whether or not anything did.
- **Table 257's levels** are asked of the `/DocMDP` signature alone and **Table 258's rights** of the
  `/UR3` one alone, which is where §12.8.6's Table 263 puts each. Ranking an approval signature
  against somebody else's `/DocMDP` would be a report firing on a condition no clause states.
- **The rights a `/UR3` grants that this comparison cannot recognise are named**, on the file's own
  statement rather than on the ranking's outcome. A reader told a change is inside a `/UR3`'s rights
  is owed the list of rights the answer could not have been about; the condition is what the
  document granted, so a `/UR3` naming only recognised rights says nothing.

**No sentence reaches the word *valid*.** `revision::Judgement` and `revision::RightsJudgement` have
no variant for it, ADR 1076 gave the word a constructor only an `Anchored` proof can reach, and the
paragraph `notes` has closed with since ADR 1039 is unchanged.

## 3. Table 258 is a different ranking, and the classifier says so twice

Round 1082 found §12.8.2.3's residue and named it: the clause states a ranking "against Table 258's
rights", and Table 257's three levels are not it. Table 257 states three levels each permitting a set
of operations; Table 258 states twenty-three named rights in five arrays and the file picks a subset.

So a changed object is classified **twice, in one walk, from the same evidence** — into `Kind` for
Table 257 and into `Exercise` for Table 258 — and neither answer is computed from the other. They
disagree in both directions, which is the argument for two:

- a form field **added** is `Kind::Unclassified`, because Table 257 names no such operation and
  therefore permits it at no level, and `Operation::FormFieldAdded`, because Table 258's `/Form`
  `Add` names it outright;
- a page **instantiated from a template** is permitted by levels 2 and 3 with nothing else asked, and
  needs `/Form` `SpawnTemplate` here;
- §12.8.4's **validation material** is carved out of Table 257's question entirely and is refused
  here, because Table 258 states no carve-out and names no right covering one.

**`Operation` has eight variants and `RIGHTS_NOT_RECOGNISED` names the other fifteen**, which is the
whole table between them and a test that holds both to it. Three groups are refused and each for a
reason: `/Document` `FullSave` and `/Form` `SubmitStandalone` are permissions to write rather than
modifications an object records; `/EF`'s four are operations on §7.11.4's named embedded files that
`Kind::Unclassified` already declines to tell apart; and `Copy`, `Import`, `Export`, `Online`,
`SummaryView` and `BarcodePlaintext` name what an interface may do with content and write nothing a
comparison of two revisions could see.

## 4. Where the refusals fall, and why a refusal rather than a verdict

A change Table 258 names no right for — a page deleted, a content stream rewritten — is
`Exercise::Unrecognised` and **not** `NotGranted`, and that is ADR 1049 §2's argument applied to the
other table. Calling it not-granted would be the strict direction rather than the lenient one, and it
would produce a false "this usage rights signature is invalidated" on every document whose update did
something ordinary; refusing says what happened without saying what the clause did not.

The one place the classifier declines where it could have guessed is an object that is a form field
**and** an annotation, added or removed. Table 226's `/FT` makes it the first and Table 166 the
second; `/Form` `Add` and `/Annots` `Create` both describe adding one, and nothing in the file says
which right the producer needed. ADR 1049 §1 resolved the same ambiguity for a *redefinition* by
reading which entries moved, which is an answer an addition does not have.

## 5. The descriptor a prefix no longer duplicates

**The confined sweep found this and nothing else could have.** `viewer-confined --test
awkward_classes` killed five documents' `report` with `SIGSYS` the first time the report ran inside
the confinement, and the reason is ADR 1043's own economy: `FileBytes::prefix` duplicated the open
file's handle, which on Linux is `fcntl(fd, F_DUPFD_CLOEXEC)`, and `pdf_sandbox`'s filter admits
`fcntl(fd, F_GETFD)` and nothing else that call can do. The filter's mismatch action is
`KillProcess`, so the careful `Err` branch could never have run — trap 31 exactly, in a crate that
had no idea it was confined.

**The fix is not a permission and it is not a fallback.** `Held::OnDisk` now carries the handle's
own length beside the `Arc`, so a prefix is the same open file under a shorter number: no system
call, no descriptor, no byte. `doc/todo/61`'s rule is why widening the filter was not considered —
the worker wanting something is not an argument — and the descriptor budget is why the old shape was
wrong even where it was permitted: `DESCRIPTOR_LIMIT` is 8 and three are inherited, so a prefix per
signed document would have spent what a reader's open documents need.

`a_prefix_is_a_file_of_its_own_length_held_either_way` asserts the raw descriptor number is the
file's, which is the thing the kernel would have changed; `pdf-syntax --test on_disk` is what says
the narrowed view still reads every corpus document object for object, from disk and from memory.

## 6. What this does not close

`/UR3` still enables nothing: this program has no feature behind such a gate, which is §12.8.2.3's
row since the hundred-and-ninety-eighth session and is unchanged. And §12.8.2.3's `should` on a
processor that writes in excess of the rights is still answered by `UsageRights::grants` and
`Right`'s three verbs, which is a different question from this one and asked at a different moment —
before a save rather than of a file.
