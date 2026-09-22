# 1272 — A page turn that does not wait for the photograph: priced, and not taken

Session 1217. Status: **accepted as a price; the thing it prices is not built.** Asks nothing of
the code. Companion to [ADR 1271](1271-a-photograph-on-the-way-into-a-page-turn.md), which took
what this round could take inside the decode, and to `doc/questions/Q121`, which puts the one part
of this that is not a design round's to decide.

## 1. The question

Principle 2's, in `doc/todo/36`'s terms: a page turn onto `issue12841_reduced.pdf` page one costs
**122.68 ms** after ADR 1271 — **68.43 ms** of interpretation and **52.89 ms** of transfer, against
the 8.333 ms a 120 Hz refresh allows. Table 87 states the image's `/Width` and `/Height` in the
dictionary, so the display list could carry the *shape* of the photograph without its samples,
present page one at once, and re-present when the decode lands. Is that a round's work?

## 2. What the first frame would show, and why it cannot be a blurred photograph

Nothing. **The cheap approximation this design wants does not exist for a baseline JPEG**, and that
is a fact about the codec rather than about this tree: ISO/IEC 10918-1's entropy-coded data is a
single bit stream in which a block's position is found only by decoding the blocks before it, so a
decode at one eighth of the resolution still pays every Huffman symbol. That is **55%** of what
this page's decode costs (ADR 1271 section 5); what a reduced decode could skip is the IDCT, the
upsampling and the colour conversion, about a third — and `zune-jpeg` offers no scaled decode in
any case. A progressive codestream's first scan would give a real preview, but whether a
codestream is progressive is its producer's choice and not a design this tree can rest on.

So the honest description of the deferred frame is: the page, correct in every mark except that
the photograph's rectangle is empty, for about a tenth of a second.

## 3. What "a frame that says it is stale" already supports, and what it does not

`doc/todo/37`'s machinery stands in for a *late* frame with the pixels already on the screen, moved
to where the new view puts them. It does not reach this case, and `stale.rs` says so in its own
module comment: **a page turn and a `GoTo` have no base at all**, because nothing about the
outgoing page's pixels is true of the incoming one at any placement. The one layer that can fill a
turn is ADR 0443's retained low-resolution page — and producing one of the *incoming* page needs
the same decode this design is trying not to wait for, by section 2.

`pdf-render` does already carry an image whose samples do not yet exist:
`ImageSource::AtDeviceScale(DeferredImage)`, whose `samples(grid)` a backend asks at draw time.
That moves the decode from interpretation into the frame; it does not move it out of the frame.
The state this design needs is a third one — *not yet, ask again* — and with it: a decode running
on a thread of its own, a re-present when it lands (the token in `Command::RenderReady` is the
existing shape for that), and a cache so that the second frame does not redo the first frame's
work.

**And one of those is structural rather than large.** `CLAUDE.md` keeps `interpret` a pure function
of what the file says, the viewer state and what the user did, and the whole cross-backend oracle
rests on the first of those three. A display list whose contents depend on whether a decode had
finished is a function of time, and the comparison that makes this tree's correctness checkable
would quietly stop being a comparison of equals.

## 4. Why this is a question and not a build

`doc/todo/36` records the owner's decision about frames that are not the right ones — *correct
frames wherever possible, reprojection where not* — and ADR 0386 section 3.3 records the one trade
of that kind available without an ask, declined in the owner's own sentence: *"we should still try
to render a correct image every frame"*. This is that trade in another place. It differs in one
respect worth the owner's word, which is why `doc/questions/Q121` exists rather than a paragraph
here: ADR 0386's case had something correct on the screen to keep showing, and a page turn has
**nothing** — the choice is a page with a hole in it for a tenth of a second against the *previous*
page for a tenth of a second longer.

**The recommendation is no**, and the reason is section 2: the hole cannot be filled with anything
resembling the picture, so what the reader gets for the wait is a page that flashes rather than a
page that arrives.

## 5. The item that keeps the owner's sentence, and is not this one

**Decoding a page's images in parallel.** Every image `XObject` a page names is decoded
independently, the pool is already there (ADR 1259 divides a mesh's paint across it), and nothing
incorrect is ever presented — the frame is the right frame, built on more than one core. It is
linear in the number of images on the page and therefore buys *this* witness nothing, because its
page holds one photograph and two commands; it is the answer for the page that holds twelve. It is
a change to how the interpreter walks a content stream rather than to how a codestream is decoded,
so it is not in this round's hands, and it belongs to `doc/todo/36`'s list with the number above
beside it.
