# ADR 0307 — The initial backdrop a knockout group hands out

Date: 2026-08-13 (session 472)
Status: accepted

## Context

`doc/todo/23` carried two open items. One wants a *second colour space* — a group inside the
page that introduces one, which needs `Command::Group` to carry a blending space and a second
command list and three backends to resolve the pair. The other reads "a knockout group whose
elements blend", and named three corpus documents: `issue18032.pdf`,
`knockout_blend_multiply.pdf` and `knockout_inner_backdrop.pdf`.

Reading §11.4.6 against those three found that **two of the three are not that item at all**.
Neither of them needs a construction this tree does not have; both were losing to a condition
that named the wrong backdrop. This round is those two, and the third — a knockout group whose
elements genuinely blend against a backdrop that is not transparent — is left standing with its
price unchanged and its population correspondingly smaller.

## What §11.4.6 says a knockout group's elements composite onto

> In a knockout group, each individual element shall be composited with the group's initial
> backdrop rather than with the stack of preceding elements in the group.

Two sentences later the clause makes isolation an independent question:

> A knockout group may be isolated or non-isolated; that is, isolated and knockout are
> independent attributes. A nonisolated knockout group composites its topmost enclosing element
> with the group's backdrop.

So there are two backdrops in play — the *initial* one, which every element of a knockout group
gets, and the *immediate* one, which is the group's accumulated result so far — and the clause
tells you when they are the same thing and when they are not. This tree had one answer for the
whole of Table 145's `/K true`, and it was the wrong answer twice.

### 1. Where the knockout rule can change no pixel, the two backdrops are the same

The initial backdrop and the immediate one differ only *after* something has been composited
into the group. Where no element that composites covers an element painted before it, the
accumulated result equals the initial backdrop at every point any element marks — so §11.4.6's
recurrence and §11.4.4's are the same recurrence, term by term, and the group is §11.4.4's
group exactly.

That condition already existed in this tree and had been derived from this clause: it is
`knockout_can_show`, which decides whether the knockout departure is worth *reporting*. What it
did not do was decide anything about the picture. `Interpreter::run_transparency_group` read
Table 145's `/K` alone and forced §11.4.5's transparent backdrop onto the group:

```rust
let isolated = group.isolated || group.knockout || … ;
```

`knockout_blend_multiply.pdf` is one non-isolated knockout group holding **one** element — a
cyan rectangle under `/BM /Multiply` over a yellow page. One element has nothing to knock out.
The clause asks for §11.3.5.2's Multiply against the page, `(1,1,0) × (0,1,1)`, which is green;
this tree gave the element a transparent backdrop to blend against, which under Multiply is the
source, and drew it cyan. Two channels of 255 apart, and the page was *reported* — for the
non-isolated group whose backdrop had been excluded — which is a report that named a real
substitution made for no reason.

The condition is now `group.knockout && knockout_can_show(&commands)`, asked of the file's own
elements before the rewrite that turns some of them into `Command::Shaped`. Where it is false
the group takes §11.4.4's route, which ADR 0237 built and which `render-cpu` and
`render-quorra` both draw.

**One term is stated rather than derived**, and it is worth the line: `|| knockout`, the flag
that says the two staged draws *are* being emitted. Those draw `P' = (1 − f) × P + S` on the
transparent start §11.4.5 gives, and seeding `P` from the page would put the backdrop in twice.
Every group that reaches the relaxed condition satisfies this one already — a knockout group is
drawn only when it is isolated or when nothing in it blends — but `command_blends` answers
`true` for a `Command::Shaped`, so the last disjunct cannot be relied on to carry it.

### 2. NOTE 6, which gives a nested group the *outer* group's initial backdrop

> When a non-isolated group is nested within a knockout group, the initial backdrop of the inner
> group is the same as that of the outer group; it is not the immediate backdrop of the inner
> group. This behaviour, although perhaps unexpected, is a consequence of the group compositing
> formulas when b = 0.

Where the enclosing knockout group is isolated, its initial backdrop is §11.4.5's transparent
one — and so, by that sentence, is the inner group's. Which makes the inner group an isolated
group by §11.4.5's own definition, "one whose elements shall be composited onto a fully
transparent initial backdrop", whatever its own `/I` says.

`knockout_inner_backdrop.pdf` is exactly that: `/K true /I true` outside, `/K false /I false`
inside, two Multiply fills within. This tree drew it right — a group's elements go onto
transparency here — and told the reader it had not, because the report asked Table 145's `/I`
where the clause asks about the backdrop. Trap 11, in its purest form: **the condition has to
come from the clause, and "non-isolated" is not the clause's condition.** The page cost the
oracle a judged page for a departure that was not there.

`Interpreter::transparent_initial_backdrop` carries the answer. It is set for a knockout
group's own content, to `group.isolated || <what this group was given>`, and cleared everywhere
else — because NOTE 6 reaches a **direct element** and not a descendant, which is what "it is
not the immediate backdrop" distinguishes. A group two levels down composites onto its parent's
accumulated content and NOTE 6 says nothing about it.

Three places ask it: the report, the knockout-drawn condition (a `/I false` knockout group
nested in an isolated one has the transparent backdrop the staged pair needs), and §11.6.7's
implicit pattern-cell group, which is an element of whatever paints it in the same sense.

`build_soft_mask` clears it while the mask's group runs, and the soft-mask call site passes
`false`: a mask is named by an `/ExtGState` rather than being an element of anything, so NOTE 6
does not reach it and a non-isolated mask group's departure is real however it was arrived at.

## What this is not

**It is not the item `doc/todo/23` still carries.** A non-isolated knockout group whose elements
blend *and* whose knockout rule can show is still refused by name, and the arithmetic says why
it is a construction rather than a condition. With `B` the initial backdrop (premultiplied,
alpha β), `P` the accumulated result, `f` the element's shape and `q` its opacity, §11.4.6's two
stages come to

```text
P' = (1 − f) × P + f × [ (1 − q) × B + q × ((1 − β) × Cs + β × B(Cb, Cs)) ]
```

and the bracket is the element composited against **B** rather than against `P`. Unrolling the
recurrence collapses the `B` terms — the group's own contribution is still Destination-Out with
the shape and then Plus — but the Plus half's colour is now the element blended against the
initial backdrop, which a layer that begins transparent cannot produce. It needs the initial
backdrop retained beside the accumulation, per element, which is a second surface-sized buffer
and a per-element scratch on three backends. `issue18032.pdf` is what is left of the population,
and it keeps both of its reports.

## Consequences

- `knockout_blend_multiply.pdf` draws the colour §11.3.5.2 states and reports nothing.
  `knockout_inner_backdrop.pdf` draws what it always drew and reports nothing. The corpus's
  incomplete list falls by two.
- One more shape of display list reaches the backends: `Command::Group { isolated: false }` for
  a group whose `/K` is true. Nothing had to change for it — the command's own `knockout` field
  is `false` there, which is the guarantee `Command::Group`'s documentation already gives — so
  `render-cpu` and `render-quorra` draw it and `render-gpu` refuses it exactly as they do for
  any other non-isolated group.
- `a_non_isolated_group_reports_only_where_the_backdrop_cannot_be_stated` gained an element to
  its `/K true` fixture. The old assertion pinned a one-element knockout group as reported,
  which was the defect written down as a test; the new pair pins both sides of
  `knockout_can_show`.
- `Interpreter::for_page` exists because the new field pushed `interpret_into` one line over
  `clippy::pedantic`'s hundred. Splitting the page's preparation from the run of its content is
  the split the function already had informally.

## Alternatives considered

**Draw the general non-isolated knockout group.** Priced above: a second buffer per element on
three backends, for one corpus document. Not taken, and `doc/todo/23` keeps it with the
arithmetic beside it.

**Substitute §11.4.4's non-knockout model on the right backdrop for the refused case too** —
`isolated: false` even where the knockout rule can show, since the first element is right under
either model and the rest are no worse. Rejected: it is closer without being correct, and this
project does not draw "closer". The refused case keeps the substitute it had, and keeps saying
so.

**Read NOTE 6 as reaching every descendant.** It says the opposite in the same sentence — "it is
not the immediate backdrop of the inner group" is a statement about one level of nesting — and a
flag that stayed set would have suppressed real reports two levels down.
