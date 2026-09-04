# 0905 — A search is a selection, and the census that judges selections now asks for one

Session 932. Status: **accepted**.

## Context

Session 929 measured a row of ADR 0895 §3's survey and found it false:

> **`selection_census`'s readback cache is never asked.** Measured: forty caret queries over the
> annex leave `hits: 0, misses: 0`. `Readbacks::get` is reached only from the search step, and the
> census issues no `Command::Find`.

That reads as one of two defects — a census weaker than it should be, or a cache in the wrong
place — and this session was asked which. **It is neither, and the third answer is worth stating
because it is the one a future round would otherwise have to find again.**

## What each of the three things actually is

- **`crate::readback::Readbacks` is a search cache and it is live.** `Viewer::readback` is its
  only reader and `Viewer::find_step` is that function's only caller, so the cache is consulted
  exactly when a person searches. It is *written* from two places — the search step, and
  `settle`'s interpretation of a page placed on the screen, which primes it so that a find bar
  opened on the page showing costs nothing.
- **Selection does not go near it.** `Command::Select(Selection::All)` and a pointer drag both
  read `Open::interpreted()`, the on-screen record, and `Viewer::selected` borrows its `text`
  directly — which is not an optimisation but the identity the census asserts.
- **`tests/selection_census.rs` is a *selection* instrument**, and its three properties — the
  drag, the readback, the caret — are all about the pointer. Nothing in it searched, correctly.

So the cache is not dead code, and the census was not weak at what it claimed. What was wrong is
the survey row that put the two together: they were never on one path, and no measurement said so
until 929 took one.

**What was genuinely missing is a third thing neither answer names.** The cache's own rules are
held by `tests/headless.rs` on a five-page fixture — a second sweep answers what the first did
without interpreting a page, an edit forgets every entry, the budget evicts — and that is a good
test. But **no instrument reached the search path over the corpus at all**, while a census that
opens every corpus document at a fitted viewport was sitting one function away from doing it.

## Decision

**The census asks for a search, because a search *is* a selection.** ISO 32000-2 §O.2.2's
`search`:

> Open the document and search for one or more words, selecting the first matching word in the
> document.

"Selecting" is the verb, and the one thing this crate has that means it is the range
`Query::Selection` answers with — `find_step` sets `Open::pending_selection` and `settle` makes it
the selection. The find bar's loop therefore ends exactly where the drag's ends, on the same
question, which is what makes this census its home rather than a fourth file.

`search_for_the_reference` is property 4. For each word `drag_across_the_reference` already found
— longest first, unique in poppler's answer and in ours — it issues `Command::Find(Find::Start)`,
pumps to the answer, and asks three things:

1. the search answers on page one, and `Query::Selection`'s text **is that word**, byte for byte;
2. the cache's **miss** counter does not move across the document's searches, which is the claim
   `viewer.rs`'s priming `put` makes in a comment and nothing measured: *the page a person is
   looking at is not interpreted twice*;
3. the cache's **hit** counter moves at least once per search, which is the sentence 929 could not
   write.

**The needle is filtered a second time and that is deliberate.** The drag's words are unique in
the *whitespace-stripped* readback, because §9.3's spacing is each extractor's own and the text
gates' subject; a search runs over the readback as it stands. So a word is searched for only where
it occurs exactly once in the untouched string, and a word the two extractors break differently
leaves the population rather than becoming a failure. The count of searches is printed beside the
count of documents, which is trap 11's arithmetic.

## Why the cost half is a count

`doc/todo/02` §2's rule is that a cost property stated as *how many times the expensive call ran*
needs no band, no calibration and no re-run on a quiet machine, and that a duration is what is
left when it cannot be. This one can: `ReadbackCache::misses` **is** the number of pages
interpreted for a search, so "the page showing is not interpreted twice" is an equality between
two integers. A neighbouring round's load cannot move it by one. It is the same construction ADR
0894 gave `pdf-vfs` and ADR 0898 gave `pdfref`, in the third crate that had a cache and no floor.

## What was declined

**Moving or removing the cache.** It is reached by the shipped search path and held by
`headless.rs`; removing it would cost a repeated full-document sweep of ISO 32000-2 the 5.45 s
ADR 0256 measured. The instruction that raised this said dead code that looks like a cache is
worse than no cache — that is right, and the finding is that this one is not dead.

**Widening the census's stated subject.** It is still what its first line says it is: what a
person gets when they act on a page and ask what is selected. A find bar is that gesture with a
keyboard instead of a pointer.

**Adding a corpus-scale search of *whole documents*.** `FIND_STEPS` is 2. A needle that occurs
once on page one is answered by the first step, and sweeping a thousand-page document to prove it
again would buy repetition at the price of the census's whole run time — the same argument
`WORDS_PER_DOCUMENT` already makes for the drag.
