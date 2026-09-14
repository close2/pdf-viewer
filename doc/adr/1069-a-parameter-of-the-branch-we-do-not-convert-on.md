# 1069 — A parameter of the branch we do not convert on

Session 1055. Status: **accepted**. One decision about ISO 32000-2 §11.7.5.3's black generation
and undercolour removal: what a document stating Table 57's `/BG`, `/BG2`, `/UCR` or `/UCR2` is
owed by a processor that converts on §10.3's branch.

`§N` is ISO 32000-2 and nothing else.

## 1. The sentence the tree held a debt on, and the question nobody asked

§11.7.5.3, under a lead sentence that restricts where the functions apply — they "shall be
applied only during conversion from DeviceRGB to DeviceCMYK colour spaces":

> When painting an elementary object with a DeviceRGB colour directly into a transparency group
> whose colour space is DeviceCMYK , the functions used shall be the current black-generation and
> undercolour-removal functions in effect in the graphics state at the time of the painting
> operation.

Read as a requirement to *apply* them, that is a `shall` this tree does not execute. It has stood
as a debt in five ledger rows since the four-hundred-and-twenty-sixth session, and it was priced
as a report: `Interpreter::black_generation_stated`, set wherever an `/ExtGState` or a pattern
dictionary states any of the four, withheld §11.4.7's page-group pair, §11.6.6's group-scoped
press and §11.5.3's four-component mask half, and the page was drawn on the device's three
components with the departure named.

The question nobody had asked is what algorithm those functions are parameters **of**. §10.4.2.4
answers it in the line that introduces them:

> The complete conversion from RGB to CMYK shall be as follows, where BG ( k ) and UCR ( k ) are
> invocations of the black-generation and undercolour-removal functions, respectively:

and Table 52 types the graphics state parameter itself the same way — "[a] function that
calculates the level of the black colour component to use when converting RGB colours to CMYK",
pointing at §10.4.2.4. There is no other algorithm in the standard that calls them.

**And the tree had already written this down.** `doc/todo/23-transparency-departures.md`, in the
four-hundred-and-twenty-seventh session: "[t]he bullets that name the black-generation and
undercolour-removal functions are §10.4.2's side of §10.4.2.1's fork; the paragraph above them
chooses a *target* and leaves the algorithm to whichever branch the processor is on." That round
used the reading to justify `rgb_to_ink` and did not carry it one step further, to the refusal
standing on the other side of the same fork. The refusal outlived its own argument by six hundred
sessions — `doc/habits/the-ledger-and-claims-about-this-tree.md`'s shape, with the answer already
in the tree.

## 2. The decision

**§11.7.5.3's black-generation bullets select among functions a conversion uses; they do not
require a processor to use one.** This tree's conversion of a colour into a four-component group
is §10.3's — the profile's own `B2A` table where the press has one, the ink cube's inverse where
it does not (ADRs 0009, 0042, 0263, 0796) — and neither has a black-generation step for a stated
function to replace. So the clause is satisfied vacuously on this branch, and §11.7.5.3 and
§10.4.2.4 are `implemented` rather than `partial`.

The authority is §10.4.2.1, which ranks the two branches outright:

> Although ICC enabled PDF processors should always follow the provisions and recommendations
> provided in 10.3, "CIE-Based colour to device colour", a less-capable PDF processor may choose
> to use the algorithms specified in the following subclauses 10.4.2.2 through 10.4.2.5. These
> algorithms are, however, very simple and as perceived by a human viewer they produce only crude
> approximations of the original colours.

Reading §11.7.5.3's bullets as a requirement to apply the functions would require a conforming
processor to abandon §10.3 for §10.4.2.4 whenever a file states a `/BG` — the branch the same
standard calls a crude approximation, and the one it tells an ICC enabled processor to avoid. The
two readings cannot both be right, and §10.4.2.1 is the clause that decides between them.

§11.7.5.3's third bullet is inapplicable on its face and always was: it conditions on "the native
colour space of the output device" being `DeviceCMYK`, and this device is a screen.

## 3. What follows, and what it is not

**The refusal comes off.** It was pure loss: falling back to the device converts `DeviceRGB` to
sRGB and composites there, which evaluates the stated functions exactly as much as compositing in
ink does — not at all — while additionally giving up the blending space §11.4.7 requires. A
refusal that does not buy the thing it refuses for is not a refusal.

**The report stays**, because the file stated something this processor's conversion has nowhere to
put, and trap 5's rule is that such input stays loud. `Unsupported::BlackGeneration` is raised once
per interpretation by `Interpreter::note_black_generation_departure`, on both halves of the
clause's own condition: a function is stated, **and** this interpretation converted into four
components. It over-reports a page that names a press and paints nothing into it in `DeviceRGB`,
and it cannot under-report.

**The condition narrowed, and that half is Table 57's rather than §11.7.5.3's.** `/BG2` and `/UCR2`
admit a second value — "the name Default, denoting the black-generation function that was in effect
at the start of the page" — which is this device's own function, so a state naming it departs from
nothing; and §10.4.2.4 makes a stated function one "defined as PDF function dictionaries (see 7.10,
"Functions")", so a name is not one. Table 57's own precedence decides which of each pair is in
force. `Interpreter::states_black_generation` is that reading. The two conditions are five-fold
apart on the world: over `doc/pdf.js` and the three web corpora,
`pdf-model --example black_generation_census` counts 8 of 963 and 910 of 39 127 documents stating
one of the four, against 1 and 189 stating a function.

**What this is not** is a claim that the functions are unimplementable or unimportant. If this
program ever grows a §10.4.2.4 branch — a `DeviceCMYK` output device, a separation preview — the
bullets bind there, and they bind with the moments §11.7.5.3 names, which this tree already reads
correctly for the rendering intent (ADR 1054). The decision is about which branch the conversion
is on, not about which parameters matter.

## 4. A claim that was stronger than its own working

Found while reading §10.4.2.4 for this: that row said an RGB colour taken into a `DeviceCMYK`
group and back to grey is §10.4.2.2's grey of the original "whatever the black-generation and
undercolour-removal functions returned", above an algebra that had substituted the nominal `k` for
both. With `BG(k) = B` and `UCR(k) = U` the sum is `0.3c + 0.59m + 0.11y − U + B`: what cancels is
`B = U`, which the nominal pair satisfies and an arbitrary pair does not. Nothing rested on the
difference, because the functions are not evaluated — but the sentence claimed more than it had
shown, and the row now says what it shows.
