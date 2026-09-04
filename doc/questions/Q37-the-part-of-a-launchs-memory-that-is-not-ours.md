# Q37 — The part of a launch's memory that is not ours

Asked by round 935, which was sent to resolve the launch gate's memory figure and did — by
narrowing what that figure claims. This question is the narrowing, put to the owner, because
`CLAUDE.md` principle 2 names the figure and the round has changed what the project measures under
that name.

`Q29` is the clock half of the same instrument and is untouched by this; the two are neighbours
rather than duplicates.

## What the round found

The gate banded `VmHWM` — the resident high-water of the process that draws page one. Measured
(ADR 0910): **nine tenths of that number is resident pages of mapped shared objects**, 52 MiB of
`libLLVM.so` and 26 MiB of `libgallium.so` above all, and how many of those pages the kernel keeps
resident is the page cache's business rather than this program's. Evicting those two libraries and
changing nothing else moved the figure 25% and moved what this program had *allocated* by 30 KiB.
That is why the figure fell away from its band three times with no code change and no clock in it.

## What the round did

Banded the **anonymous** high-water instead — what the allocator asked the kernel for — which held
to 4% over eleven samples where the whole-process figure moved by a factor of two, and which
separates the four documents (27, 31, 32 and 44 MiB) where the old figure barely did. The
whole-process figure is still measured and printed on every run, beside the driver's share of it,
and is banded nowhere.

## The question

**Is that the right thing to gate, and does the whole-process figure need an instrument of its
own?**

What a user's machine actually pays to have this program's first page on screen is 110 to 180
MiB — three to six times what the program allocates — and after this round nothing holds that
number to anything. Three positions, and the round has no view on which the project wants:

1. **Accept it.** The gate guards what we own, the rest is the graphics stack's, and a number that
   moves 25% when a neighbour reads a file is not a regression signal. The printed line keeps it
   visible without pretending it is ours.
2. **Gate it too, by pinning the input.** The variance has a known cause, so it can be removed
   rather than tolerated: warm the driver's shared objects into the page cache before each child —
   the gate already controls the disk in the other direction, dropping a document's cache for the
   cold-open arm. Cost: the band would then be a claim about a machine whose page cache is fully
   warm, which is not the machine a user launches on either, and the warming has to be derived
   from a child's own mappings rather than from a written-down list of libraries (trap 25).
3. **Treat the driver's footprint as a thing to reduce.** 100 MiB of Mesa mapped and ~50 MiB of
   `libLLVM` touched is most of a launch's memory, and this round measured that it is **not** the
   software rasteriser being enumerated: restricting the Vulkan loader to the AMD driver alone
   changes the whole figure by 2.5 MiB. So there is no cheap switch here, and pursuing it means
   asking what the graphics stack is asked for at bring-up — quorra's question as much as ours.

## What this supersedes, so that `main` does not get two half-answers

Round 932 met the same red lines on its own branch (`b93ffaac`, not merged) and asked
[`Q32`](Q32-a-memory-band-whose-floor-nothing-controls.md) — *should the launch path's memory
high-water be a ceiling rather than a band?* — after lowering each floor to a few per cent under
the lowest reading it had seen, ceilings untouched. **`Q37` supersedes `Q32`'s recommendation, and
`Q32`'s measurements are evidence for it.**

- **Its evidence fits the mechanism exactly.** Two runs of one binary ten minutes apart read 108.9
  and 99.3 MiB for the same row, and seven consecutive runs then sat within a megabyte of one
  another: a figure that is stable *within* a plateau and steps *between* them is what a page cache
  whose state holds and then changes produces. Its finding that `main` is below all four floors
  with the round's own change taken back out is the same finding session 933 reported and the same
  one this round explains.
- **Its recommendation — take the floor off `peak_mib` and put one on `open_peak_mib`** — is half
  right and stops short. The half that is right is that the floor is a claim nothing in this
  program controls; what this round measured is that **the ceiling is not a claim about this
  program either**. The whole-process figure was seen at 92 MiB and at 180 MiB on one binary in one
  afternoon, so a ceiling of 231 admits an eighty-mebibyte leak on a cold cache and fires on a warm
  one. A one-sided band on that figure would be a guard that cannot cry wolf *and* cannot see a
  wolf.
- **The other half of its recommendation is already true**: `open_peak_mib` has had a two-sided
  band since session 922, and it is one of the figures judged on any machine.
- **Its floor edit is superseded rather than merged.** `peak_mib` no longer exists in
  `doc/checks/launch-path.toml`, and the harness rejects an unknown row key rather than skipping
  it, so a merge that took `peak_mib = 95 .. 209` from that branch **fails the gate loudly on the
  first run** instead of leaving two answers standing quietly. Whoever merges the two takes
  `peak_anon_mib` and drops the four `peak_mib` lines; `Q32`'s file and the paragraph it added to
  the check file's header are the record of how the question was asked, and the header paragraph
  is the one thing worth carrying across, as a fourth recurrence in the list this round rewrote.

## What is *not* being asked

Whether to widen the old band. Three rounds declined to and this one agrees with them: a band is a
claim about a machine, and the old figure's own range on one afternoon was 92 to 180 MiB.
