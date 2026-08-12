# 0288 — A profile inflated once per mark

**Status.** Accepted.
**Context.** `doc/todo/03` §1 asks a round to take a chunk of the 65 944-document population. This
round re-surveyed **all** of it, which re-ranks the demand side, and took what the survey named.

## The survey, re-run whole

145 archives, one process each — `render-cpu` aborts on panic, so one document's abort would take
an archive's verdicts with it — 65 944 distinct documents, 1188 s of process time, 0 failures:

```text
65 944 documents: 173 unopenable, 45 locked, 23 encrypted beyond us, 52 pageless,
                  823 incomplete, 2 slow
```

**823 incomplete against ADR 0269's 1144**, which is the eleven rounds since; the rate is 1.25% of
the web against the pdf.js corpus's 6.7%. The interesting line is the last: **the two slow
documents are not the two ADR 0271 diagnosed.** Those were fixed; these are new to the list because
they were under 30 s before the survey's 24-way load and are over it now.

| | commands | interpretation |
|---|---:|---:|
| `3129278.pdf` | 5 874 | **34 450 ms** |
| `3990833.pdf` | 279 | **24 948 ms** |

Five point nine milliseconds per command, and eighty-nine.

## `3129278.pdf`: one profile, 1053 times

`callgrind`, 380 G instructions:

| | share |
|---|---:|
| `ColourSpace::parse_at` (inclusive) | **95.20%** |
| — of which `Document::decoded_stream_data` → `filter::flate` | 77.76% |
| — and `icc::Profile::parse` | 17.46% |
| `shading::Cache::build` (inclusive) | 2.30% |

The page states **1053 axial shadings, each its own object**, each preceded by its own `cs` naming
one `[/ICCBased 15 0 R]` space. `shading::Cache` could not help — the shadings are distinct — and
nothing cached the *space*, so the profile was inflated and its tables read **1053 times**.

`Interpreter::icc_spaces` remembers it, keyed by the `ObjectId` the `/ColorSpace` resource entry
states. **34 450 ms → about 1 550 ms, 22×.**

Three things about the shape of it, each a decision:

- **Only `[/ICCBased <stream>]` is remembered.** `ColourSpace::parse` is a pure function of the
  object *and* the resource dictionary in force — §8.6.5.1 resolves a name through it, and an
  `Indexed` space's base may be a name — so a space cannot in general be keyed by its object. This
  one can: §8.6.5.5 makes its whole content the stream, and no resource dictionary can change what
  it means. That is `shading::Cache`'s own caveat about named spaces, applied one level up.
- **The table is asked before the shape is tested.** `is_icc_based` resolves the array, which copies
  it, so it runs on the *miss* path only — once per distinct object rather than once per operator.
  The first version tested first and made every `cs` pay a copy; the corpus gate is what showed it.
- **A space answered from the table takes the same path as one parsed on the spot.**
  `take_colour_space` was split out for that: §8.6.8 makes `cs` set the initial colour too, and a
  memo that skipped it would set the space and leave the *previous* space's colour — a wrong picture
  produced by a cache.

`shading::Cache` gained the same table for the same reason, and it is worth 4% on this page rather
than 95%: the shadings' own `/ColorSpace` is read by `kind_of`, which is 2.3% of it.

## `3990833.pdf`: not this, and it is named rather than fixed

`callgrind`, 233 G instructions: `image::convert_channels` 22.2%, `zune_jpeg` about 30%,
`colour::press_at` 9.8%, `zlib` inflate 9.3%. Thirty-eight images on one page, converted sample by
sample through a press. It went 24 948 → about 19 500 ms with this round's change and the rest is
a different population — recorded in `doc/todo/03` as the next candidate the survey offers, with the
profile beside it.

## What this says about `doc/todo/41`

That file prices a decoded-stream cache at **0.7% of interpretation**, measured over the pdf.js
corpus, and its own lesson is "[p]rice an item on the corpus, not on the page the profiler happens
to open". The corpus said 0.7%; this document says **78%**. Both numbers are right about their
population, and the file's rule needs one more clause: **the corpus is not the web either.** A
decoded-stream cache is still not taken here — what was taken is narrower, memoises a *parsed*
profile rather than bytes, and needs no eviction argument — but the price is now recorded against
two populations rather than one.

## Gates

Every verdict identical: corpus 65 incomplete, oracle 905/68/786/1/2/14/18, both text gates,
quorra 915/37/5/17, 1623 tests. The corpus gate is *faster* — 5.0/4.8 s against 5.6/5.4 before,
A/B in one sitting on a machine whose load inflated both.
