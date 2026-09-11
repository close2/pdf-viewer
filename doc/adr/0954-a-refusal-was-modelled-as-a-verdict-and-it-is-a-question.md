# 0954 — A refusal was modelled as a verdict, and it is a question

Session 954. Status: **accepted**, and it **amends ADR 0947**'s account of the middle stage,
ADR 0951's `REFUSED_BY_NAME`, and ADR 0952. Commissioned by the project owner on 2026-09-11.

## What the owner said, and why it is a design finding rather than a feature request

> This tool will probably be used in automatic environments, where a refusal would mean that a
> human has to intervene.

Read that against what ADR 0947 built and the mistake is visible. That ADR gave the converter five
decisions and defended `Refused` carefully — a file leaves the verb only if it conforms, a
requirement in no table is refused **by name**, and session 953 was praised for moving two rows out
of `NOT_BUILT_YET` into the fence because "a promise nothing will keep" is worse than a refusal
with a sentence.

All of that is right and none of it is the problem. **The problem is that `Refused` was modelled as
an outcome of the conversion, when it is an outcome of the conversion's *default policy*.** The
converter was not saying "this cannot be done". It was saying "I will not do this without being
told to", and then providing nowhere to be told.

For one person converting one document those are indistinguishable, which is why four sessions
built on the first reading without noticing. For a queue they are opposites: the first is a fact
about the document, and the second is a fact about the configuration — and the second is fixable by
the operator, once.

## The second finding, which the owner supplied and the code had no room for

> The easiest remedy is just losing information. But for instance for embedded images we could
> provide a remedy, that we append the images to the document.

`Decision` had exactly two answers between "do it" and "stop": a loss the user authorises, and a
refusal. **That is a false pair**, and holding it made a whole class of answers unthinkable — the
ones where the information neither survives in place nor is lost, but *moves* to somewhere the
target admits. An embedded image appended as a page is not a loss. A metadata packet prefixed as a
page is not a loss. A signature's signer, time and verdict written onto a page is not a loss, even
though the cryptography cannot survive.

Sorting the answers by **what happens to the content** rather than by what the converter does gives
four kinds, and the sort is what makes a configuration legible to the person writing it:

| | |
|---|---|
| **`stop`** | the conversion refuses. The default, for every site |
| **`discard`** | the information is lost — today's authorised loss |
| **`preserve`** | it survives, somewhere the target admits |
| **`derive`** | a new representation is made from content the document already has |

## The decision

**A refusal becomes a named site with a default of `stop` and a documented menu of remedies, and a
configuration file may answer any site in advance.** `doc/rfc/0007` is the design and carries the
external-tool API, the per-site remedy table and five questions the owner has to rule on before it
is built.

Three parts of that are decided here because they are not really open:

1. **`stop` stays the default, and a site absent from the configuration behaves exactly as it does
   today.** Installing the feature changes no pipeline. A converter that quietly did something else
   to a document nobody asked it about would be a worse failure than the one being fixed.
2. **Every site is enumerable** — `--remedy-sites` prints them from the same table the converter
   decides from — so a site cannot exist undocumented and a configuration naming one that does not
   exist is an error rather than an ignored line. Session 954's coverage census is what makes this
   possible at all; before it, nobody could list the refusals.
3. **`derive` is never reachable without being named**, is reported per document, and is recorded
   in the file's own `xmpMM:History`. A PDF/A file whose movie has become three screenshots is a
   *different document* from the one that went in; the converter may produce it and may not pretend
   otherwise.

## A third finding, from the owner the same day: a remedy belongs to a site *and a target*

> the provided remedies will also differ (slightly) based on the selected target. As we could
> insert an embedded pdfa for variant pdfa/4 but not for pdfa/2 (and pdfa/4f would allow any file
> as embedded file to avoid losing information)

This is not a refinement of the table, it is the table's shape. One site has a different best
answer under each target, and the answers differ **in kind** rather than in degree: under PDF/A-4f
the original file is attached unchanged and *nothing is lost and nothing is derived*; under
PDF/A-4 a PDF/A must be derived from it, so the content survives in another format; under PDF/A-2
neither is available and the honest answers are `discard` or `stop`.

So the menu is a function of the pair. `--remedy-sites` takes a target, and a configuration naming
a remedy the chosen target does not admit is **an error naming both**, never a quiet fall-through
to `stop` — a configuration that silently does less than it says is the failure mode this whole
change exists to remove.

And it yields the cheapest useful thing in the proposal: **the report can say when another target
would have kept what this one loses.** The converter already holds every target's requirement
table, so "the target you asked for cannot hold this; PDF/A-4f can" costs nothing to compute. It
must not act on it — `doc/pdf-a-conversion-limits.md` §9 exists because the owner said switching
levels is not a way to avoid problems — but an operator who learns once that their queue would lose
nothing at 4f can change their own policy, and that is a decision only they can make.

## What this does to `Because`, and the category error inside it

`Because` has four kinds — `TheFence`, `NotBuiltYet`, `NotThisTarget`, `Declined` — and ADR 0951
was right that separating them matters. But they answer **two different questions** and the type
conflates them:

- `NotThisTarget` says *no conforming file exists for the target you asked for*. Nothing an
  operator configures changes that. It is a fact about the document and the target.
- `TheFence`, `NotBuiltYet` and `Declined` say *this converter will not, or cannot yet, do the edit*
  — and every one of those is a sentence about **us**, not about the document.

Only the second group can have a remedy. The first cannot, and a configuration offering one would
be offering a lie. So the type divides: a refusal that is a property of the input keeps its present
shape, and a refusal that is a property of our policy becomes a site with a menu.

`TheFence` is the interesting member of the second group, because it looks like the first. The
fence is `CLAUDE.md`'s authoring exclusion, and it is *this project's* rule — amended twice already,
both times by argument. A remedy that appends rather than invents may well sit inside it; RFC 0007
§7 asks the owner rather than assuming either way.

## What is not decided, and is not this ADR's to decide

Whether `apply` runs the tool or returns a request for the caller to run — the second keeps RFC
0002 §5's purity, which is a claim this project tests rather than asserts. Whether `derive` belongs
in an archival converter at all. Whether a tool should run under this project's own sandbox where
it can. RFC 0007 §7 puts all five to the owner, and nothing is built until they are answered,
because building the wrong one of these is expensive in exactly the way a configuration format is
expensive: it becomes an interface the moment anyone writes a file against it.

## The one thing that changes immediately

Nothing in `crates/`. What changes is that `REFUSED_BY_NAME`'s rows and `decide`'s catch-all stop
being a list of endings and start being a list of **questions with no answer configured yet** —
which is what session 954's census was already counting without a name for it.
