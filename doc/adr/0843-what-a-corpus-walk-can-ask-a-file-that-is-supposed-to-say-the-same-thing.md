# 0843 — What a corpus walk can ask a file that is supposed to say the same thing

Session 900. Status: **accepted**. The fifteenth decision record of RFC 0002's implementation,
and the instrument beside ADR 0842's verb.

## Context

ADR 0835 opened with the observation that RFC 0002 section 9's four layers are about
*appearance*, so none of them can see a structure tree: "a merge that dropped every structure
element would draw bit-identically and pass every assertion the three walks had."

**`optimize` has the mirror-image problem, and it is worse.** Every pass in ADR 0842 is
*supposed* to be invisible to all four layers. A rewrite that pruned an object it should have
kept, packed a member into a carrier at the wrong offset, or re-encoded a stream into something
that decodes differently will still draw page 1 correctly whenever the damage is not on page 1 —
which, over a corpus of mostly multi-page documents, is most of the time. So a raster comparison
is necessary here and nowhere near sufficient, and a walk that stopped there would be a walk
that could not fail.

Three questions therefore have to be asked of the *file*, and RFC section 9 already names two of
them: the second layer's self read-back, and the property gate — "`optimize` is idempotent — its
own output, optimized again, is byte-identical" — which session 888 could not take because the
verb did not exist.

## Decision

### 1. The content comparison is of **decoded** bytes, and that is this walk's own question

`tests/split_corpus.rs` compares each page's `/Contents` *encoded*, and its comment says why: "a
comparison of decoded content would pass on a piece whose streams had been re-encoded, which
RFC 0002 section 11.1 does not permit". That is right for `split`, which promises pass-through.

It is exactly wrong for `optimize`, which promises `CLAUDE.md`'s *other* arm — "carried byte for
byte **or recompressed without reinterpretation**". Here the encoded bytes are *expected* to
change and what must not change is what they decode to. §7.8.2 makes the comparison well formed
over an array of content streams — "the division between streams may occur only at the
boundaries between lexical tokens" — so the concatenation of a page's decoded streams is the
page's marks whatever shape the producer stored them in.

Two walks over one construct asking opposite questions is the point rather than an inconsistency:
each asks what its verb promised.

### 2. Idempotence is the gate that can see what nothing else can

Every pass in ADR 0842 is a function of the input's object graph and nothing else. A pass that
was not — one whose result depended on an iteration order, a hash seed, or on some object that
survived only because a *previous* pass left it behind — would show up here and in no other
assertion the suite has. It caught real design constraints before it was ever run: the walk's
`/Length` rule and the recompressor's refusal to touch an indirect `/Filter` are both there
because without them the first rewrite leaves an orphan that the second one prunes.

The walk therefore rewrites every corpus document, rewrites *the output*, and requires the two to
be byte-identical — beside the determinism assertion the other walks make, which is a different
claim (same input twice) and does not imply this one.

### 3. The output's own closure is asked, and §7.5.7 excuses the only exceptions

After pruning, every object the file holds should be one some path from §7.5.5's `/Root` or
§14.3.3's `/Info` arrives at. §7.5.7 states the one exception in as many words: an object stream
is an indirect object "although there might not be any references to it (of the form 243 0 R)",
and §7.5.8's cross-reference stream is the same. Both are recognised by their own `/Type` rather
than by anything the writer recorded, which is what keeps the check a question about the file.

`support::check_optimized` then reads Table 16 back off each carrier: `/N` against the number of
pairs in the header, `/First` against where the pairs end, "[t]he byte offsets shall be in
increasing order", every member's start inside the decoded stream, no member that is a stream
object, and `/Extends` naming another carrier without a cycle. Trap 8's rule holds throughout:
the walk asks the *output* whether it conforms, never whether it matches what `optimize`
intended — a writer that was consistently wrong would satisfy the second kind of check.

### 4. The savings are attributed in the walk, not in a script somebody ran once

Principle 2 asks an optimisation to carry the benchmark that justifies it. The walk rewrites
each document four ways — nothing, pruning, pruning and recompression, and all four passes — and
prints the totals with each pass's contribution. That puts the number ADR 0842's table states
under the same gate as the correctness assertions, over the whole population rather than a
sample, and it is what will notice the day a pass stops earning its place.

The counts are **printed and not ratcheted**, on `doc/todo/05`'s standing rule and ADR 0835 §2's:
they are a function of the corpus and of what this walk asks, so a changed plan moves them for a
reason that is not a regression.

### 5. What fails, and what is held

The assertions bind from the first run: no panic, determinism, idempotence, every page read
back, decoded content unchanged, no §7.5.5 or §7.5.7 fault, nothing unreachable surviving
pruning, no §14.7 tree lost, and the corpus smaller than it started. A **refusal is not a
failure** — a document this verb declines by name is the document's, counted by reason and
printed (trap 11) — and a raster difference nobody has diagnosed fails the run, with `HELD` the
place a diagnosis goes. `HELD` is empty, which is the state to keep.

## Consequences

- `crates/pdf-transform/tests/optimize_corpus.rs`, the fifth corpus walk of the suite, added to
  `doc/todo/02` §2's sequence and to its change-map row; it calls `require_the_sandbox()` for
  trap 10's reason, like the four before it.
- `crates/pdf-transform/tests/support/mod.rs` gains `check_optimized` and `OptimizedCheck`,
  beside `check_structure` and for the same reason: the structural question belongs in the module
  written independently of the crate, not in the verb's own tests.
- `crates/pdf-transform/tests/optimize.rs` holds the per-property tests over committed fixtures
  and `qpdf --check` as the foreign evidence, in `tests/split.rs`'s shape.
- The §14.7 check is a *comparison* here rather than an absolute one: a rewrite copies objects,
  so a tree the source stated is a tree the output states, element for element, and a fault the
  source already had is not the rewrite's. That is the one place this walk asks less than ADR
  0835's does, and it asks it of both sides instead. **The comparison is of counts and not of
  the sentences**, and the first run of the walk is why: a fault names the object it is about,
  a rewrite renumbers, and comparing the strings reported 103 carried faults as new ones. The
  103 are real and are the corpus's — parent-tree entries for a *page* that state an indirect
  reference where §14.7.5.4 makes a content stream's value "an array of indirect references" —
  and the count is now printed as a fact about the population.
