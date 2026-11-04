# 1931 — A property list nothing defines is a membership dictionary; a width veraPDF misreads

Session 1931, a single archive round. ADRs 2683 and 2684; no question, trap or habit.

**Why.** Round 1929 left three outputs our checker passed and veraPDF 1.30.3 failed:
`AIAA-2005-6173-712` at 4 under both stating profiles (6.2.10.5-1, and 6.2.11.5-1 at 2b), and
`issue14438` under `state-every-loss` at 4 (6.2.2-3).

**AIAA (ADR 2683).** The failing code is 45 of `Helvetica` (object 324). Its direct encoding sends
45 to `/minus`, and `/Widths` says 584. The program is the substituted Liberation Sans, shared
through one `/FontFile2` with font 213, whose `/WinAnsiEncoding` makes 45 `hyphen` (333). By
§9.6.5.4 each font gets its own glyph, and both widths agree. veraPDF caches one mapping per program
for every direct encoding (`FontProgramIDGenerator` gives each the key `direct`). With object 213's
encoding made indirect, the meaning is unchanged and the failure disappears. Verdict: veraPDF's
error. No converter change; a checker test pins the reading.

**issue14438 (ADR 2684).** Annotation 19's appearance (object 49) opens `/OC /MC0 BDC` with no
`/Properties`, so veraPDF is right. Our survey read a list name with `peek`, so neither 6.2.2 row saw
it. It now uses `look_up`, and ADR 2500's remedy defines the name as `<< /Type /OCMD >>`, which
Table 97 makes hide nothing, as the reader drew it.

**Wording.** The statement sentence counts each kind of entry by name (removed objects, omitted
characters, removed keys, halftone entries, font, run and recovery records); JSON: `entries`, `listed`.

**Tests.** `a_property_list_name_nothing_defines_is_reported_undefined` (survey),
`a_property_list_nothing_defines_is_a_membership_dictionary_naming_no_group` (converter), and
`one_program_under_two_encodings_is_judged_by_each_fonts_own_encoding` (checker). Wording asserts are
in three test files. With `peek` planted back, the first two failed; without the converter arm, the
second failed.

**Census.** ADR 2669's command was compared with round 1930's run. There are 4500 outputs before and
after, none new or gone, and the summary lines are unchanged. 4129 match once masked, and 365 differ
only in the sentence's wording. Six gain `/Properties`: `issue14438` (2b, 4) and `issue17679` (both
profiles, 2b, 4). Before the change, veraPDF failed `issue17679` at 4 on 6.2.2-3.

**Gates.** fmt and `clippy -D warnings --all-targets` (`pdf-archive`, `pdf-transform`): exit 0.
nextest of both: 1364 passed, exit 0. Archive tier (`corpus`, `archive_corpus`, `witnesses`): exit
0 each. `cargo test -p conformance`: exit 0 after one quotation was cited under §8.11.2.2.
`state.sh docs`: exit 0. veraPDF sweep of the 153 outputs new since round 1928: 151 pass, 2 fail
(AIAA at 4, ADR 2683). The 6 changed outputs and 18 sampled wording-only outputs: all pass.
