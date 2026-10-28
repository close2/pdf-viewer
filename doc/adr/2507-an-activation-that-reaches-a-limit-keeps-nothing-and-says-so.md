# 2507 — An activation that reaches a limit keeps nothing and says so

Session 1836, a clause round in batch 124. Status: accepted. Follows ADR 2506.

## Context

Page 380 recommends that a processing application have some means of identifying and ending the
execution of seemingly infinite loops (cited by page, as the RFC's preamble asks). Two calculations that each read the other and add one never settle, and a template can hold
a hundred thousand of them. The limits below are principle 3's: a decompression bomb has its
counterpart in a template built to run for ever.

## Decision

1. **Four limits** (`Limits`, default 10 000 runs, 10 000 000 steps, 1 000 000 names and 64 MiB of
   assigned text), each tested at the place the thing grows: a run is counted before it starts,
   the discovery run of each calculation included; the steps every script takes between them are
   charged after each run (a script handed what is left, and one that took more than it was handed
   ends the activation); the names the calculations hold between them are counted when a
   calculation's set is recorded; and the bytes of text are charged in `Form::set`, every assignment
   at the length of its text whether or not it overwrites one, with the discovery runs' copy of the
   form charged to the form the activation keeps. An assignment past the room is refused with an
   error the script sees and ends the activation as `Budget("bytes")`. A script's own `Budget`
   holds a value to a megabyte, but a template of N fields each assigned that value would hold N
   megabytes with two runs and no step count to see it, so the bytes are a limit of their own.
   The defaults are well past what a form needs: the census found a few dozen calculation scripts in
   five documents (ADR 2482), and the limits are a fraction of a second of work.
2. **Reaching one ends the whole activation, and its edits are dropped.** A half-settled cascade is
   a form whose values do not agree with each other, and writing it into `/V` and the `datasets`
   packet would make the file say something no complete run of the calculations says. This is the
   rule `pdf-script` already holds for a script that does not finish: "A run that does not finish
   hands back no edit and leaves the realm's table as it was" (ADR 1603 section 3). The validations
   of an unsettled form are not run either.
3. **The sentence is kept.** The report says which limit and after how many runs, and says that
   nothing calculated is kept; that sentence is kept past the report's cap of 256 sentences, after
   which the rest are only counted, in one more (`and N more sentences`). A sentence is cut at 512
   characters where it is kept, and the set that removes duplicates stops growing when the report is
   full, so the report holds at most 258 sentences of 512 characters whatever the template, a
   quarter of a megabyte at most.
4. **A hostile script and a hostile template are both bounded.** A script is bounded by its own
   `Budget` as ADR 2482 decided, and by what the activation has left. The work list is a loop over a
   set of at most the form's calculations, with no recursion, so the call stack does not grow with
   the template; dependency discovery adds one run per calculation, counted in the same limit; the
   memory an activation adds to the form is the assigned text within the fourth limit, the
   dependency names within the third and the report within item 3.

## Consequences

- Tests at the bound: the cycle at 1, 2, 50, 51 runs and the default; the step bound; the names
  bound; the bytes bound, discovery's copy included; the report cap and the cut sentence (`crates/pdf-formcalc/tests/cascade.rs`). The fuzz target `formcalc_cascade`
  runs the activation under small limits and asserts the drop, the count and the cap.
- A form that legitimately needs more than the defaults gets no cascade at all rather than a
  partial one; the limits are public so that a host can raise them for a form it trusts.
