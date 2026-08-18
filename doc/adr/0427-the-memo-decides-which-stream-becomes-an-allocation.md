# ADR 0427 — The memo decides which stream becomes an allocation

Status: accepted, 2026-08-18. Session 592. Finishes the first half of `doc/todo/14`'s remainder:
the four content streams §7.8.2 names beside a page's `/Contents` are no longer decoded whole
whatever their size. ADR 0365 is the same road for the page's own stream; ADR 0359 is what put
these four under the same clause in the first place. Amends §7.3.8, §7.3.8.2, §7.4 and §7.8.2's
ledger rows.

## What changed, in one line

**A decompression bomb hidden in a form XObject, a Type 3 glyph description or an annotation
appearance stops being an allocation.** The same 1.85 MB
of file `doc/todo/10` §2 describes, moved out of `/Contents` and into an `/XObject`, costs
**10.7 MB** of peak resident memory where it cost **1032 MB** — and it now reports
`MAX_OPERATIONS`, the bound that is actually about it, where it used to report
`undecodable form /Fx`, which is a bound of ours wearing a sentence about the file.

## The hole ADR 0365 left, and why it was left

That round windowed a page's `/Contents` and said plainly what it did not do:

> A page's `/Contents` is read through the window. The four other content streams §7.8.2 names
> — form XObjects, patterns, Type 3 glyph descriptions, annotation appearances — are decoded
> whole exactly as before. […] What follows honestly: **a bomb hidden in a form XObject still
> costs its gibibyte.**

The reason was not laziness and it is still true. `doc/todo/14`'s criterion for the good case is
a stream read **once, forwards**, and none of the four is:

- §11.6.6's paired runs interpret the *same* form two and sometimes three times —
  `group_commands` for the subtractive half, again for the black half, `rerun_on_device` for a
  third.
- A tiling pattern's cell runs once per cell, and `MAX_TILES` allows four thousand of them.
- A Type 3 glyph description runs **once per character drawn with it**.
- An annotation's appearance is read twice by construction: once by
  `annotation::appearance_damage`, which has to answer §7.4.1's damage question where the stream
  is rather than where it is drawn (ADR 0359), and once by `draw_appearance`.

A window that re-inflated the stream for each of those would trade an allocation for unbounded
work, which is not the trade `doc/todo/14` is about. So that file asked for one of two things:

> Whoever takes this needs a route that streams the *first* read and remembers the bytes for the
> second, or a measurement saying the re-inflation is cheaper than the memo.

## The answer is neither of those two, and it is not a new number

**The decoded-stream memo already draws exactly the line that was wanted, and it draws it for
the same reason.** `DecodedStreams::put` declines any decode it cannot hold beside its own
encoded bytes:

```rust
if data.len() > self.allowance(encoded) { return; }
```

— `allowance` being `DECODED_BUDGET` less the encoded length, four mebibytes less the encoded
length, ADR 0317's figure. So the population splits in two and each half wants the opposite
route:

|  | today | after |
|---|---|---|
| decode the memo **keeps** | decoded whole, held, second read is a cache hit | **unchanged** |
| decode the memo **declines** | decoded whole, **not** held, second read decodes again | pumped through a 64 KiB window, second read pumps again |

The right-hand column of the second row is the finding. **A stream the memo declines is already
re-decoded on every read** — that is what declining means — so windowing it costs nothing that
was not already being paid, and it removes the allocation. And every decompression bomb is in
that half by construction, because a bomb is precisely a stream whose decode dwarfs anything a
cache would keep.

`doc/todo/10` §6 requires that "[n]othing arbitrary may be replaced by something equally
arbitrary". Nothing is: the split is `DecodedStreams::put`'s own condition, asked before the
decode instead of after it, and the two are one function — `allowance` — so they cannot drift.

## The premise has one exception, and the fuzzer found it

"A stream the memo declines is already re-decoded on every read" is a claim about *who reads*,
and it is true of three of the four: a form asks `content_stream` afresh at every `Do`, a Type 3
glyph description at every character, an annotation appearance at every draw. **It is false of a
tiling pattern**, because `Tiling` holds the decoded bytes for the whole tiling — the pattern is
its own memo, with exactly the lifetime the window would have to replace. Windowing it inflates
the cell again for *every cell painted*, and `MAX_TILES` allows four thousand of them: road D's
trade backwards, unbounded work in place of an allocation something else already bounds.

**It was not reasoned out; it was measured, on an input `cargo fuzz run page` produced.** A
mutated tiling pattern out of this round's own seeds ran 76 seconds under the sanitiser, which
`doc/verify.md` says to check against a release binary before believing — and the release binary
said **0.242 s before this round and 8.99 s after**, on the same profile and the same file. Cut
the cell count from 25 to 4 and the 8.99 falls to 3.95, which is what says the cost is per cell.
With §8.7.3.1 taken back off the routing constructor it is **0.238 s**, and the three that stay on
it keep what they gained: `pdf-retrieve page … 0` on this round's 40 MiB seeds reads 0.135 → 0.051
for a form, 0.250 → 0.102 for a Type 3 glyph and 0.119 → 0.037 for an appearance.

**The exception is a type rather than a comment.** `HeldContent` is what `Tiling` holds and only
`HeldContent::of` produces one, so a later round cannot point the cell at the routing constructor
without the field's type changing under it — which matters here more than usual, because a route
decision is invisible in its output: the bytes are the same bytes, the report is the same report,
and what moves is a clock nobody watches. `NestedContent::windowed` exists for the same reason
`inflate_buffer` exists so a test can read `Vec::capacity` (ADR 0354): the thing being decided has
to be askable.

## How it is asked

`Document::nested_content_source` is the whole rule, and it is four lines of decision:

1. A chain no `Pump` can produce — anything but a single `FlateDecode` with no predictor — comes
   back whole by the route it always took. That is `is_pumpable`, extracted from `stream_source`
   so that the page route and this one cannot answer it differently.
2. Otherwise the stream is decoded **under a bound equal to the allowance**. Succeeds: whole, and
   in the memo, which is where it would have been anyway.
3. Refused for that bound alone: pumped. Every other refusal is the file's and is returned as
   itself.

§8.7.3.1's cell does not ask at all — see the exception above.

**Step 3 has a property worth stating, because it removes a whole class of edge case:** a stream
that reaches the pump has already produced `allowance` bytes under step 2, so it is decodable and
non-empty. "Nothing decoded at all" is therefore still decided before the run, exactly as it was,
and the five callers keep their five sentences for it.

What it costs is a decode of at most four mebibytes thrown away before the pump starts. That is
bounded, it is paid only by streams larger than anything the cache would hold, and it buys the
`Ok`/`Err` answer that keeps the callers' reporting unchanged.

## What the interpreter holds now

**`Interpreter::content_stream` returns a source rather than a buffer**, which is the change that
made the four call sites fall out. `NestedContent` is either the decoded `Arc<[u8]>` or the
*encoded* one plus what a `Pump` needs, and `NestedContent::reader()` makes a `ContentReader` per
run — so §11.6.6's three runs are three readers over one source rather than three borrows of one
decode, and the tiling loop asks for a reader per cell. `Interpreter::run` takes the source.

Two things follow and both are deliberate:

- **The report is the same report, said in the same words, from one of two places.** Where the
  bytes are whole the damage is known at the decode and `content_stream` says it, as it has since
  ADR 0359. Where they are windowed the damage is met mid-run, so `run` takes the reader's issues
  afterwards and `note_nested` translates them. `Unsupported` is a set keyed by the item, so a
  form run three times reports its damage once.
- **`ContentIssue` does not escape.** It is Table 31's noun and every indexed variant of it is
  about a part of a *page's* `/Contents`; a form reached through `/XObject` has no part index and
  putting a zero there would say something untrue. `TokenTooLong` and `TooLarge { part: None }`
  carry no index and cross as themselves; `Damaged` becomes the `DamagedStream` sentence ADR 0359
  wrote.

**And a bound breach is a refusal rather than damage**, which ADR 0343's distinction requires and
which this round had to decide for a second population. A damaged prefix is drawn and reported
because §7.8.2 makes a content stream "a sequence of instructions" and a prefix of one is a
shorter sequence of the same kind — the producer's own bytes. A token longer than `CEILING`, or a
stream that reaches `max_stream_len`, is neither: nothing about the file went wrong, this reader
declined, and the marks that are missing are missing because of a number of ours. So they are
`Unsupported::Content`, with `ContentIssue`'s own vocabulary, and they are never dressed as
damage. The one *behaviour* change in that direction is worth naming: a form whose decode passes
`max_stream_len` used to be `undecodable form /Fx` and nothing drawn, and is now the prefix up to
the bound plus `ContentIssue::TooLarge`. That is louder and more honest — the reader could read
the stream and chose to stop — and it is the same correction ADR 0306 made one layer down.

## What it costs, measured, because principle 2 requires the number

Callgrind, `RAYON_NUM_THREADS=1`, `--profile gates`, A/B in one sitting with the round's own
patch applied and reversed (`doc/environment.md` forbids `git stash` here):

| | before | after | |
|---|---|---|---|
| ISO 32000-2 page 101, interpreted 50 times | 1 231 915 218 | 1 233 013 260 | **+0.089%** |
| `prefilled_f1040.pdf` page 1 — 242 widget appearances | 92 274 692 | 92 301 906 | **+0.030%** |
| `alphatrans.pdf` page 1 — transparency groups, so paired runs | 6 461 643 | 6 462 621 | **+0.015%** |

**Which is what the design predicts rather than a pleasant surprise**: the whole ordinary
population takes the route it always took, and what was added to it is one read of the cache's
budget and a `min`. The corpus agrees at the other end of the scale — `display_list_digest` over
all 974 pdf.js documents is 37.2–37.9 s on both arms, three runs each, indistinguishable.

**What it buys**, `VmHWM` from `/proc` because `ru_maxrss` has a floor equal to the spawning
process's own resident set (ADR 0362). The fixture is `doc/todo/10` §2's Bomb B — 1.85 MB of
file, 1029:1, rebuilt from that file's description and coming out at 1 847 511 bytes — put once
in a page's `/Contents` and once inside a form XObject the page's one operator invokes:

| `pdf-retrieve page … 0` | before | after |
|---|---|---|
| the bomb in a **form XObject** | 1032.3–1032.7 MB, 1.23–1.32 s, `undecodable form /Fx` | **10.5–10.7 MB**, 0.11 s, `MAX_OPERATIONS` |
| the same bomb in `/Contents` (the control, ADR 0365's) | 8.4–8.6 MB, 0.10 s | 8.3 MB, 0.10 s |

The control is what says the measurement is of this round: the page route was already windowed
and does not move.

## Output identity, as bytes

`CLAUDE.md`'s rule 1 makes interpretation a pure function of the document and the view state, and
this round changed how the bytes reach it for four of the five streams that carry instructions.
So the artefact is compared rather than the verdict:

**`examples/display_list_digest` over every pdf.js corpus document's page one, 975 lines, is
byte-identical across the change** — `sha256 04a07587…` on both arms, with the same
`pdf-sandbox-worker` on disk for both (that example's own warning).

## The test, and that it fails without the change

`crates/pdf-model/tests/nested_content_window.rs` pins the exception as well, on
`NestedContent::windowed`: the same stream is windowed by the routing constructor and whole by the
held one, which is the difference the fuzzer's input costs 37× on. The rest of it builds one
form's instructions twice — once
`/FlateDecode`d past the allowance, once with no filter at all — and asserts **first** that the
two take different routes and **second** that they draw the same commands. The order matters:
comparing the two pictures alone would pass just as happily if the window were never reached,
which is how a route decision stops being tested without anything going red.

Both halves were confirmed to fail. With the allowance forced to `usize::MAX` — every form whole,
which is the tree as it stood this morning — the route assertion fails. With it forced to zero —
every form windowed — the *memo* test fails, and the picture comparison still passes, which is a
second piece of evidence that the window and the whole decode deliver the same instructions.

Trap 8 is why the fixture is built by hand: no corpus document states a form whose decode
outgrows four mebibytes, and the rule is about the ones that do.

## What this does not do

**The four filters that are not Flate are still not pumped**, and that is the rest of
`doc/todo/14`. LZW, ASCII85, ASCIIHex and RunLength are streaming by construction — §7.3.8.2 says
so itself, "most filters are defined so that the data shall be self-limiting; that is, they use
an encoding scheme in which an explicit end-of-data (EOD) marker delimits the extent of the data"
— and LZW reaches about 1365:1, so it is the sharper bomb of the two. A stream stating one of
them still takes the whole route, in a page's `/Contents` as well as in a form, and
`is_pumpable` is the single place that says so.
