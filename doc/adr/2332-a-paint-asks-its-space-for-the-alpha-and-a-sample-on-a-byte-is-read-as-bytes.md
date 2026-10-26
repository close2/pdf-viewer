# 2332 — A paint asks its space for the alpha, and a sample on a byte is read as bytes

Status: accepted. Session 1749, 2026-10-25, the measure slot of batch one hundred and fourteen.
Context: ISO 32000-2 §7.10.2, §8.6.6.4, §8.6.6.5, §8.7.4.5.5 to §8.7.4.5.8, §11.4.7; ADRs 1644, 1645
(this page's interpretation priced before), 1969 (when a band moves), 2258 (the instrument); trap 6;
habit 164; principle 2's rule that an optimisation carries its benchmark and its comment. Moves no
ledger row; moves one row's two bands (section 6).

## 1. The question, and the hypothesis that did not hold

`bug1721218_reduced.pdf` page 1 is the slowest page the turn gate holds, and its interpretation is
the largest term after the scene. The brief's hypothesis was a font program parsed per turn. **It
did not hold**: the page names no font. Its content is one `DeviceCMYK` group form, 2.8 MB inflated,
whose 3 490 `sh` operators each paint under a clip of their own, run once on each of §11.4.7's two
process planes (ADR 1645).

## 2. Where the interpretation goes, counted

Callgrind over `pdf-model`'s `examples/callgrind_pages -- <n> <file> 1 kept-revisit` on an export of
HEAD, a run of two interpretations less a run of one, threads apart: one steady interpretation is 1
122 M instructions, **965.6 M of them on the interpreting thread**. Of that thread: the content's
tokens about 290 M and the operators' dispatch 144 M; `shading::ramp` 216 M, 83 ramps of 256
samples; clips 72 M; the forms' inflation 57 M. Inside the ramps, `Compositing::paint` is 121 M over
21 248 samples, and **67 M of it is a conversion to RGB whose colour is thrown away**: on every
branch but `Device` the paint reads only the conversion's alpha, and on a `Separation` space that
conversion runs the tint transform, a type 4 function here, a second time a sample.

The comment beside the conversion priced it as paid "per *distinct* colour rather than per sample".
That was true of the solid paints and images it was written for; a shading's ramp calls the paint
once a sample, and that is the price that decayed.

A first interpretation in a process also pays the press's ink table, 1 194 M on the pool
(`Press::table`, built once a process). The gate keeps the quickest of five rounds, so it does not
see it; a person opening the document does, once.

## 3. The decisions

- **`ColourSpace::rgb_alpha` is the conversion's alpha without the conversion.** The alpha is a
  function of the space, never of the components: transparent for §8.6.6.4's `/None` and §8.6.6.5's
  `DeviceN` of nothing else, reached through the three indirections `to_rgb_at` follows under its
  depth bound, and opaque everywhere else. Its match names every space, so a new space is decided
  there when it is decided in the conversion, and `a_spaces_alpha_is_its_conversions_alpha` holds
  the two to each other over every family and each indirection; it fails with `/None` made opaque,
  and with the indirections' depth not advanced (trap 13). **`Compositing::paint` converts only on
  `Device`**, and every other branch asks `rgb_alpha` — trap 6's one conversion read for its alpha,
  not a second conversion. Its ceiling first (habit 164): the subtractive branch with its alpha set
  to one and no conversion, a probe, read 901.0 M on the interpreting thread, and the change reads
  901.0 M.
- **A read that starts on a byte and takes whole bytes is read as bytes.** The 41 sampled functions
  (§7.10.2) the page's ramps parse a turn, about 4 096 eight-bit samples each, were read a bit at a
  time; read as bytes, `Function::parse_within`'s own instructions fall from 43.8 M to 12.8 M, about
  190 a sample. `BitReader::read` takes an aligned read of 8, 16, 24 or 32 bits from the bytes, and
  every other read, and an aligned one that would pass the end, a bit at a time as before, so the
  value and the position each read leaves are unchanged; the meshes of §8.7.4.5.5 to §8.7.4.5.8 are
  read by the same reader. `a_read_taken_as_bytes_is_the_read_a_bit_at_a_time` compares the two at
  every width the two clauses admit, from seven alignments, to past the end, and fails with the
  bytes taken little-end first or the position left a bit short.

The display list's `--digest` is `850915a4…` on HEAD, the probe, and each change.

## 4. The figures

**Instructions**, one steady interpretation (a run of two less a run of one), the display list's
digest unchanged on each page:

| page | HEAD | the alpha alone | both |
|---|---|---|---|
| `bug1721218_reduced.pdf` p1, the interpreting thread | 965.6 M | 901.0 M | 869.9 M, −9.9 % |
| the same, all threads | 1 122 M | 1 057 M | 999 M |
| `personwithdog.pdf` p1, §8.7.4.5.7's patch meshes, all threads | 105.6 M | — | 24.8 M, −76 % |

`personwithdog.pdf` is the second reader of `BitReader`: its meshes state 32-bit coordinates and
8-bit flags and components, every one an aligned read that took a bit at a time.

**The clock**, one `--clock` hold on 2026-10-26, HEAD's export interleaved with this round's, pinned
to the four Zen 5 cores and their siblings, load 1.2 to 2.2. `frame_budget` on
`bug1721218_reduced.pdf`, three runs of five rounds each way, ms:

| row | HEAD | this round | per interleaved pair |
|---|---|---|---|
| turn | 132.59, 132.77, 132.40 | 129.48, 130.88, 128.06 | −3.1, −1.9, −4.3 |
| its `interp` | 52.39, 53.69, 52.75 | 48.89, 49.85, 48.76 | −3.5, −3.8, −4.0 |
| seventh | 133.48, 132.12, 132.28 | 129.03, 130.58, 127.58 | −4.5, −1.5, −4.7 |
| step, which interprets nothing | 83.42, 83.71, 83.52 | 83.01, 83.97, 82.93 | inside the spread |

In the same hold the turn gate read `bug1721218_reduced.pdf`'s turn at 135.84 ms on HEAD (interp
54.82) and 132.00 on this round (50.87), inside its band; and `personwithdog.pdf`'s at 9.38 ms
(interp 3.72) and **6.97 (1.41)**, under the band's floor of 7.30 — the one figure of 33 outside,
which is the gate seeing the change. The launch gate judged 59 figures on each export and none was
outside.

## 5. What is left, priced and not built

Of the 869.9 M the interpreting thread now spends, about 435 M is the content itself — its tokens
read (290 M) and its operators dispatched (144 M) — and the group's 2.8 MB is read twice, once on
each process plane, because ADR 1645 makes the black list a second run of the content and not a
function of the chromatic one. Keeping the first run's tokens for the second would save at most the
second reading and its inflation, about 170 M, a fifth of the thread, since the operators still run
on each plane; and it would hold some 600 000 operands for the whole of the group, tens of megabytes
the content budget would have to charge, on the one page in the gate that has the most of them. That
is a change to the run's memory, argued on its own, and it is not made here.

## 6. The band that moved

`personwithdog.pdf`'s turn moved past its floor and its seventh frame to within half a millisecond
of its own, so both move by ADR 1969's rule over three runs of the change — the hold's and two in a
second hold, one of HEAD's between them: turn 6.97, 7.07, 7.09 ms, `turn_ms` 7.3 .. 10.8 → **5.9 ..
8.6**; seventh 7.29, 7.25, 7.26, `seventh_ms` 6.8 .. 13.6 → **6.1 .. 8.8**. HEAD's 9.38 and 9.40 lie
past the new ceiling, so taking this ADR back fails the gate. The row's busy end (ADR 1556) is a
second mode, judged against HEAD and not banded (ADR 1969 section 3). Its step, which reads no mesh
data, read 10.17 to 10.45 ms against HEAD's 10.06 and 10.33 and keeps its band, and so does every
`bug1721218_reduced.pdf` row: the gate read its turn at 132.00 to 136.39 ms on the change against
HEAD's 135.84 and 136.61, inside the band it has.
