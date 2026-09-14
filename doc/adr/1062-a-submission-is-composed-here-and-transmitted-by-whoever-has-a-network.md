# ADR 1062 — A submission is composed here and transmitted by whoever has a network

## Status

Accepted, 2026-09-14. Session 1047. Replaces the refusal ADR 0478 recorded for §12.7.6.2.

## Context

ISO 32000-2 §12.7.6.2: "Upon invocation of a submit-form action, an interactive PDF processor
shall transmit the names and values of selected interactive form fields to a specified uniform
resource locator (URL)."

This tree answered that sentence with `Action::Refused` carrying "SubmitForm: §12.7.6.2's
submission, which needs a network", and the reason was true of the *verb*: `CLAUDE.md`
principle 3 gives the process that reads a PDF no network, deliberately, and is not going to.
ADR 0212 re-read it against the round that gave a document's restrictions four levels and
correctly concluded that none of them applies here — this is refused for want of a capability,
not because a policy says no.

What nobody asked was how much of the clause the verb actually is. *Which* fields, under Table
239's `/Fields` and Table 240's Include/Exclude, `IncludeNoValueFields` and `NoExport` rules; in
*which* of the four formats the clause lists; to *which* URL by *which* method; carrying *which*
values, which for a form somebody has filled in are not the file's `/V` at all. Every one of
those is a function of the document and of `ViewState`, both of which this crate holds, and none
of them needs a socket. The refusal was a capability's, and it was applied to a whole clause.

## Decision

**The crate composes the request; a host sends it.** `pdf_model::submission::compose` produces
the whole request — URL, method, media type, body, field count — as a value;
`viewer_core::interact` hands it over as `Event::Submit`; `viewer_host::policy::may_submit`
answers whether this machine transmits it.

Three consequences, and each is the reason this is an ADR rather than a commit:

1. **The policy is asked once, in a place a host can supply.** That is `CLAUDE.md` principle 3's
   own shape — "a refusal that cannot become an 'ask' is the thing to avoid" — and the refusal
   this replaces could not become one, because it was four words inside `action::refused` with no
   caller able to answer differently. A host with a network answers `Ok`; `doc/todo/38`'s *ask*
   and *warn* levels are a change in `policy.rs` and nowhere else.

2. **What is not composed is named, never dropped.** `Submission::owed` carries one sentence per
   flag of Table 240 that the composition does not carry out, and `interact::perform` puts every
   one of them into `Event::Reported`. A submission that quietly lost bit 7's `/Differences` would
   be trap 5's silence inside a feature otherwise built, and it would hide better here than almost
   anywhere: the body would still be a valid FDF.

3. **XFDF stays declined, and the reason is the standard rather than a dependency.** Table 240
   bit 6 wants "XFDF, a version of FDF based on XML as defined by ISO 19444-1"; that document is
   not on this disk, and `CLAUDE.md` principle 5 makes a grammar taken from another reader or from
   sample files not a reading of a specification at all. Session 1035 corrected the same row's
   stale reason on the import side; this is that correction applied to the submission side, and it
   is bought rather than built.

**This is not the authoring exclusion.** An FDF is not a page: it carries the document's own field
names and the values a person entered into that document's own fields, and §12.7.6.2 requires it
by name. Nothing here composes marks. The PDF format is `ViewState::save`, which is §7.5.6's
incremental update the tree already writes.

## Alternatives

**Keep the refusal and close nothing.** Honest, and it was honest for nine hundred sessions — but
it answers a clause of which the unimplementable part is one verb, and it leaves a person who
presses a submit button unable to see where their form was going.

**Give the crate a network behind a feature flag.** Rejected on principle 3: the process that
parses untrusted bytes does not get a socket, and a flag that could grant one is a flag that will
be turned on.

**Compose in `viewer-core` instead.** Rejected because the composition is entirely a reading of
the document and of `ViewState`, both of which are `pdf-model`'s; putting it a layer up would put
Tables 239 and 240 in the crate that knows about windows. What `viewer-core` supplies is the one
thing `pdf-model` cannot see — Table 240 bit 5's mouse click — which is why `compose` takes a
`Click` and reads the widget's own rectangle from it.

## Consequences

`pdf-model` gains `submission.rs` and `action::SubmitForm`; `viewer-core` gains `Event::Submit`
and a click parameter on `interact::trigger`, because a submit button is a widget and Table 240
bit 5 measures from it; every host gains one arm calling one policy. `QUORRA_EVENT_KIND_COUNT`
moves 19 → 20, and `QUORRA_ABI_VERSION` does not, for the standing reason: a new event kind is a
number an old caller has a `default:` arm for.

§12.7.6.2's ledger row moves `reported` → `partial`, which reads backwards and is not: `reported`
meant nothing of the clause was done and it said so, and `partial` means most of it is done and
the remainder says so. What the remainder is, the row now names by flag.
