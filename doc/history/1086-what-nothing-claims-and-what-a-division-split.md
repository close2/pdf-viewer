# 1086 — What nothing claims, and what a division split

§14.8.2.2.1, §14.8.2.2.2 and §14.8.2.3 close, and §14.8.2 and §14.8 with them. Each had been read
right for hundreds of sessions and each was waiting on a computation.

## The census first, and it moved the design twice
`examples/unreached_content_census` splits a tagged page's readback four ways from
`Interpretation`'s public spans and the files' own dictionaries — a second route to the same
question. 1660 paths, 191 with a `/StructTreeRoot`, 4654 pages, 10 727 789 characters: 9 418 765
reached, 465 173 declared `/Artifact`, 1755 in an unresolved `/MCID`, 841 998 in no sequence at all.
**§14.8.1's claim is the gate**: 831 599 of the 843 753 unreached — 98.6% — sit in 37 documents that
have a tree and never state `/MarkInfo` `/Marked true`, and this clause is addressed to "tagged PDF
files". **The `/MCID` decides, not the parent tree**: `issue15340.pdf` and `issue20516.pdf` state
identifiers `/K` names while their page's index resolves to nothing — 98 characters of body text the
index route would have called our artifact. ISO 32000-2's own PDF refutes the expectation it was
given, declaring 189 976 artifact characters and leaving 2039 to absence: it tags its running heads.
The two ISO 14289 parts, WTPDF 1.0 and the Best Practice Guide leave **zero** unreached over 205
pages and 445 305 characters, 41 241 of them declared.

## The computation, and the decision §14.8.2.3 leaves
A run is an artifact by absence when it lies in no `/MCID` sequence, no `/Artifact` sequence, and no
appearance stream of an annotation whose `/StructParent` makes it a content item (§14.7.5.3, NOTE
2's "as well as annotations") — worth 765 characters, all widget and choice appearances.
`structure::artifacts_by_absence` is the complement, `content::ArtifactSource` says which of the
clause's two sentences produced a span, white-space-only runs are dropped (they can be
`separate_text`'s own inference). **10 021 of 8 443 205** over the 154 tagged documents — the
census's 10 786 less those 765, agreement to the character. Consumers unchanged:
`pdf-retrieve --no-artifacts` subtracts both kinds, the accessibility tree never had a node for it,
search still finds it. §14.8.2.3 states a determination and no use for it, and §14.8.2.2.1 NOTE 3
says that silence is deliberate — so the use is ours and documented as a choice: the readback keeps
U+00AD and `select::find` skips it with the break after it; the needle is not folded, U+002D is
untouched, nothing joins two separate words. ADR 1100.

## Rows and gates
`14.8.2.2.1`, `14.8.2.2.2`, `14.8.2.3` `partial` → `implemented`; `14.8.2` and **`14.8` itself**
follow, all 59 subclause rows settled — its note had named debts its children had discharged four
separate times and now states a rule instead of a list.
`-p conformance` 7 passed, `fmt --all --check`, `--workspace --doc` and both `fuzz/` lines all exit
0. `text_extraction` exit 0, 11096/11131 words in bounds (99.69%), both ratchets slack 0.
`selection_census` exit 0. `accessibility_census` exit 0, **all 30 ratchets slack 0** — the tree is
built from the structure tree, so nothing no element reaches was ever spoken. **No ratchet moved.**
`nextest --workspace` 4907/4908 and the workspace clippy line are red only in `transparency_groups`
and `viewer-core/src/notes.rs`, a sibling's mid-edit files; this round's nine crate files lint clean
under `-D warnings`.
