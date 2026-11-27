# 3791 — An element in an optional content group that is off is still one "to be rendered"

Session 2272. Status: **accepted**. Context: ADR 3790, review 2271's not-filed reading; ISO 32000-2
Annex Q.1, Q.2, §8.11.2.1, §8.11.4.3.

## Context

Q.2's population is "each graphical element to be rendered on the page", and the survey reads every
element, whatever optional content group it belongs to. Review 2271 asked whether an element in a
group that is off should leave the population.

## Decision

It stays in. §8.11.2.1 defines a group as "a collection of graphics that can be made visible or
invisible dynamically by PDF processors", and §8.11.4.3 gives `/D` only "the initial state of the
optional content groups when a document is opened", with `/Configs` "other configurations that may
be used under particular circumstances". Whether a group is off is a state of a processor at a moment,
not of the page; Q.1 asks whether "a given page contains any graphical elements" in transparency,
which is the page's. So "to be rendered" is read as *renderable under some state the document
allows*, and an element that a person or a usage application can turn on is one. This is a reading,
not a sentence of the standard: Annex Q does not name optional content at all.

## Consequences

- No code changes. The cost is a blending space written for a page whose transparent marks start
  hidden; the reading that dropped them would leave such a page without the space its marks need the
  moment a reader turns the group on, which is the conformance failure the space exists to prevent.
