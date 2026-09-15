# 1100 — What nothing claims, and what a division split

Status: accepted. Session 1086.
Context: `crates/pdf-model/src/structure.rs` (`artifacts_by_absence`, `keep`,
`Tree::claims_object`), `crates/pdf-model/src/content.rs` (`Interpreter::tagged`,
`Interpreter::structural_annotations`, `finished`), `crates/pdf-model/src/content/report.rs`
(`ArtifactSource`), `crates/pdf-model/src/content/annotations.rs`,
`crates/viewer-core/src/select.rs` (`matches_at`),
`crates/pdf-model/examples/unreached_content_census.rs`, `tools/pdf-retrieve/src/lib.rs`.
Clauses: ISO 32000-2 §14.8.1, §14.8.2.2.1, §14.8.2.2.2, §14.8.2.3, §14.8.2.6.2, §14.7.5.1.1,
§14.7.5.2, §14.7.5.3, §14.7.5.4, §14.9.4.

Two clauses close here and they have the same shape: each had been *read* correctly for hundreds
of sessions and each was waiting on a computation nobody had made.

## 1. The census came first, and it moved the design twice

`examples/unreached_content_census` splits a tagged page's readback four ways — reached by an
element, declared `/Artifact`, inside a sequence whose `/MCID` the parent tree does not resolve,
and inside no sequence at all — over 1660 paths, 191 with a `/StructTreeRoot`, 4654 pages,
10 727 789 characters. It is built from `Interpretation`'s public spans and the documents' own
dictionaries, so it answers the question a second way and can disagree with the code.

It moved the design twice, which is what measuring first is for (trap 8).

**§14.8.1's claim is the gate.** 831 599 of the 843 753 unreached characters — **98.6%** — are in
37 documents that have a structure tree and never state `/MarkInfo` `/Marked true`. §14.8.1 makes
that entry the claim to be a tagged PDF and §14.8.2.2.2 is addressed to "tagged PDF files", so a
file that never made the claim has not left anything out of a tree it never promised. Without this
gate the rule would have reclassified a third of those documents' text.

**§14.7.5.2's identifier decides inclusion, not §14.7.5.4's index of it.** The parent tree is the
route *back* from a mark to its element, and a file may write the `/K` entries and not the index:
`issue15340.pdf` and `issue20516.pdf` state `/MCID`s some element's `/K` names while their page's
parent-tree entry resolves to nothing, 98 characters of body text between them. Deciding by the
index would call a producer's broken index this reader's artifact. So what is asked is whether a
run carries an `/MCID` at all — which is cheap, local, and costs a tagged page no tree lookups.

## 2. What the computation is, and the one route that is not a mark

A run of the readback is an artifact **by absence** when it lies in no marked-content sequence
carrying an `/MCID`, in no `/Artifact` sequence, and in no appearance stream of an annotation whose
own `/StructParent` makes it a content item. The third is §14.7.5.3's, and NOTE 2 is why it is
here: "[t]he phrase 'any content' above refers to all page content as well as annotations." An
appearance stream carries no `/MCID`, so without it every filled-in field on a tagged form would be
an artifact — 765 characters over the corpus, all of them in `annotation-text-widget.pdf` and
`annotation-choice-widget.pdf`, and 98.7% of one form's page besides.

A run of nothing but white space is dropped. §14.8.2.6.2 NOTE 1 is about a document stating its
word breaks and `separate_text` reconstructs the ones a document leaves to be inferred, so a gap
between two tagged paragraphs can be this reader's own inference; naming it an artifact would be
this reader reporting on itself.

`ArtifactSource` says which of the clause's two sentences produced a span. A consumer subtracting
artifacts wants both and needs no distinction; one reporting on a document's tagging wants the
difference, and this reader's inference must never be presented as the file's statement.

**The result, both instruments:** 10 021 characters of 8 443 205 across the 154 tagged corpus
documents, which is the census's own 10 786 unclaimed less the 765 the annotation route spares —
agreement to the character, by two routes. ISO 32000-2's own PDF is not the witness this was
expected to have: it declares 189 976 characters of `/Artifact` and leaves 2 039 to absence, so a
specification that tags its own running heads was the opposite of the assumption.

## 3. §14.8.2.3 states a determination and no use for it, so the use is ours

The clause requires the writer to "distinguish explicitly between soft and hard hyphens so that a
PDF processor can unambiguously determine which type a given character represents", and says
nothing whatever about what the processor does next. §14.8.2.2.1 NOTE 3 says that silence is
deliberate: tagged PDF's purpose "is not to prescribe what the PDF processor does, but to provide
sufficient declarative and descriptive information to allow it to make appropriate choices". So
this is principle 5's documented choice.

The choice: **the readback keeps U+00AD and the searcher folds it.** `select::find` skips the
character in the text together with the line break after it, so a word an incidental division split
is found by its own spelling. Four things it deliberately does not do: it does not rewrite
`Interpretation::text`, so a copy of the same range is still what §9.10.2 produced; it does not
fold the *needle*, so a person who types the character still means it; it does not touch NOTE 1's
hard hyphen U+002D; and it joins only what a division split, never two separate words. NOTE 2's
warning that "hyphenation can change the spelling of words" is answered where the clause answers
it, by §14.9.4's `/ActualText`, which has already replaced the readback before any of this runs.

The fold is unconditional rather than gated on §14.8.1's claim, and that is the one place these two
decisions differ. The reason: the artifact rule reclassifies content, so it needs the file to have
promised something; U+00AD is a character whose meaning is Unicode's, and a reader that found the
word only in documents that filled in a catalog entry would be answering about the catalog.
