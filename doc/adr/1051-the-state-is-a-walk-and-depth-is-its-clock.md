# 1051 — §12.5.6.3's state is a walk over the page, and depth is its clock

Session 1034. Status: **accepted**. Adds `pdf_model::annotation_state` and
`pdf-model/examples/annotation_state_census.rs`; reads Table 175's `/State` and `/StateModel`,
which nothing in this tree read before. No pixel moves.

`§N` is ISO 32000-2 and nothing else.

## 1. Where the state is not

§12.5.6.3's second sentence is the whole difficulty:

> The state is not specified in the annotation itself but in a separate text annotation that
> refers to the original annotation by means of its IRT ("in reply to") entry

So "what is the state of this annotation" cannot be answered from its dictionary. It is a question
about the *other* annotations on its page, and answering it is a walk. Table 172 bounds that walk
— "Both annotations shall be on the same page of the document" — so the population is the page's
own `/Annots` and no second page is opened.

## 2. Three decisions, each because the clause states no alternative

**Depth is the chronology.** "Additional state changes shall be made by adding text annotations in
reply to the previous reply for a given user", and the clause names no other order. So the walk is
breadth-first from the original and a deeper state supersedes a shallower one *for the same user
and the same model*. The tempting alternative — order by Table 166's `/M` or Table 172's
`/CreationDate` — is a reader trusting a producer's clock about a fact the standard has already
told it how to order, and neither entry is required. 664 of the 1752 state changes in ISO 32000-2's
own PDF reply to another state change, so this is the rule that decides, not a corner.

**A `/State` with no `/StateModel` still states a state.** Table 175 makes the second "Required if
State is present", and a file that omits it has departed from the table — but Table 174 puts every
state name under exactly one model, so the model is *read from the table* rather than invented, and
the state the file did write is kept. Refusing the whole state change over the missing entry would
discard a fact that was stated.

**A name outside Table 174 is carried as the file spells it.** Both entries are `text string`s with
no enumeration constraining them. `StateModel::Other` and `State::Other` keep what the producer
wrote, because dropping it is the silent swallowing principle 1 forbids and a panel saying "this
file says `Approval`" is telling the truth.

## 3. What this does not decide

Nothing is drawn, and the clause asks for nothing to be drawn: the states are handed over the way
`popup::Popup` hands over a window. §12.5.6.2's other `shall` — "[i]nteractive PDF processors shall
not display replies to an annotation individually but together in the form of threaded comments" —
is untouched and still owed, and that row carries it.
