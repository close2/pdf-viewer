# ADR 1079 — A URI is resolved against a location only a host knows

## Status

Accepted, 2026-09-15. Session 1065. Extends ADR 1062's shape to §12.6.4.8.

## Context

ISO 32000-2 §12.6.4.8 makes two statements this tree had answered unevenly.

The first is the action's own: "[a] URI action causes a URI to be resolved", of a string the
clause introduces as one that "identifies (resolves to) a resource on the Internet". Deciding
*what the URI is* has been `pdf_model`'s since ADR 0070 — Table 210's `/URI` against Table 211's
`/Base` by RFC 3986 section 5's reference transformation, with `/IsMap`'s cursor appended by
EXAMPLE 1's arithmetic — and reaching the resource has been nobody's. Four windows each wrote
their own sentence about that: three printed `link: {uri}` and the confined one printed "this
window does not open links".

The second is Table 211's, and it is the one nothing was doing:

> If no base URI is specified, such partial URIs shall be interpreted relative to the location of
> the document itself.

`Uri::relative` said so and stopped, on the reason that "this crate does not know" the location.
That is true of `pdf-model` and of `viewer-core`, whose rule 2 keeps paths out of the core — the
host "opened the file and chose how to hold it" and the core "reads through what it was handed,
never a path". It is not true of any window in this tree. All four open the document by path and
all four hold it.

So a `shall` was going uncarried-out because the clause's own condition — *the location of the
document itself* — is a fact about this machine, and every layer that knew it had decided the
question belonged to a layer that did not.

## Decision

**The crate decides what the URI says; a host decides what it means here and whether to follow
it.** Three functions in `viewer_host::policy`, which is where the five decisions ADR 1062, ADR
0604, ADR 0431 and ADR 0310 put there already live:

- `resolve_uri(document, uri)` carries out Table 211's sentence. An absolute reference is returned
  untouched; a partial one is resolved by `pdf_model::uri::resolve` against the `file` URL of the
  path the host opened.
- `may_open_uri(uri)` is the policy, asked once.
- `uri_note(uri, refused)` is the sentence, shaped like `submission_note`.

Three consequences, and each is why this is an ADR rather than a commit:

1. **The refusal can become an ask.** `CLAUDE.md` principle 3 says "[a] refusal that cannot become
   an 'ask' is the thing to avoid", and four `println!`s in four windows is the canonical shape of
   one that cannot. A host that does open links — or `doc/todo/38`'s *ask* and *warn* levels, which
   is what a person would actually want in front of this decision — is now a change in one function.

2. **The base is percent-encoded, and that is load-bearing rather than tidy.** A path is not a URI:
   a `#` in a file name left as itself makes the *base* carry a fragment, so RFC 3986 section 5.3's
   merge takes the directory from the wrong place and the link resolves to the wrong neighbour. Every
   byte outside RFC 3986's unreserved set is encoded and the separator is not, which is the
   conservative choice — encoding a character that need not be is harmless, and the other direction
   is not. A path that is not UTF-8 yields no base at all and `may_open_uri` says the reference is
   still relative, rather than a guessed encoding for somebody's directory name.

3. **What is not done is named.** A reference nothing could resolve and a resolved one this machine
   declines are two different failures with two different sentences, because "declined" on its own
   does not tell a person whether the link was broken or the reader was.

**This is not a network.** Nothing here fetches anything, and `CLAUDE.md` principle 3 still gives
the process that parses the file neither a socket nor any way to acquire one. What changed is where
the decision lives, and that one half of the clause is now executed instead of deferred to a layer
that could not execute it.

## Alternatives

**Teach `viewer-core` the document's path.** Rejected on rule 2, which exists so that `interpret`
stays a function of the bytes, the viewer state and the gesture — the property the cross-backend
oracle rests on. It would also have to cross the FFI and the confined protocol to reach the two
windows that need it most.

**Have the core hand out `relative` beside the URI so a host knows to resolve it.** Unnecessary:
whether a reference is absolute is a function of the string, `pdf_model::uri::is_absolute` answers
it, and `viewer-host` already depends on `pdf-model`. An event field would have cost the ABI a
change for a fact already in the event.

**Open the link with the platform's handler and be done.** Rejected as a decision nobody has taken:
starting another program on a string the *document* chose is exactly the kind of act
`CLAUDE.md` principle 3 wants asked rather than assumed, and the owner has not been asked. The shape
that makes asking cheap is what this ADR buys; the answer stays *no* until somebody says otherwise.

## Consequences

`viewer-host` gains three public functions and one private `file_url`; `pdf-model`'s `action`
module and `Uri::relative` name where the remainder of their clause is now carried out; the four
windows lose a sentence each and gain a call. Nothing crosses the ABI and no event changes, so
`QUORRA_EVENT_KIND_COUNT` and `QUORRA_ABI_VERSION` are untouched.

§12.6.4.8's ledger row stays `partial`, and the remainder it names is now one act rather than two:
the fetch. The half about the document's own location is executed and tested
(`host_mappings.rs::a_partial_uri_resolves_against_the_documents_own_location`).
