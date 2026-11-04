# 1933 — A simple font states its widths, and the checker fails one that does not

Session 1933, a single archive round. ADRs 2686 and 2687; trap 321; no question.

**The premise.** Swept with `cargo run -p pdf-archive --example one -- <file> <target>`.
The row's first reading took every font dictionary in the file. It failed 23 sources at 2b and
512 at 4, and 1258 outputs that rounds 1931 and 1932 had written as conforming. 1250 of those
were part 4 fonts that only an interactive form's `/DR` names, and veraPDF passes them. Read as
the clause reads, over the fonts the file uses, it fails 23 sources at 2b and 386 at 4, and 64
outputs (7 documents). `0000490`'s outputs at 2b and 4 fail veraPDF at 6.2.11.2-4/5/6 (a TrueType
`Arial` in rendering mode 3 with none of the four entries). So the gap had already let a
non-conforming output through. The other six are part 4 outputs with one of the fourteen in mode 3
and none of the four; veraPDF passes them, and ISO 32000-2 Table 109 does not.

**The row (ADR 2686).** `fonts/simple-font-states-its-widths`, at 6.2.11.2 and 6.2.10.2. A used
`Type1`, `MMType1` or `TrueType` font states all four entries. At part 2 one of the fourteen,
spelled exactly, may leave out all four (Table 111, quoted in its doc comment); at part 4 none may.

**The remedy.** `Rewrite::FontWidthsStated` writes `/FirstChar`, `/LastChar` and `/Widths` as the
reader's laid-out advances, as a stated interpretation. ADR 2685's refusal is gone. The reader
itself changed (ADR 2687): an embedded program under one of the fourteen names, with no
`/Widths`, now reads the published metrics. §9.6.2.1 says a processor provides them, where it
gave 0 before. Still refused by name: a font with no descriptor drawn only in mode 3, and a direct
font or an unloadable one.

**Tests.** `a_simple_font_states_its_widths_unless_a_standard_one_states_none_of_the_four`, plus
two in `type1_widths.rs`: one replaces 1932's refusal test, one is the mode 3 refusal. Planting
the old reader back failed the remedy test; silencing the row failed all three.

**Census.** Against round 1932's runs. Shipped, with the pack: 4 new outputs
(`issue11150_reduced`, both profiles at 2b and 4); 16 gone, all named above (`0000490` at 2b and 4,
the six at 4); the other 5024 are identical. Stating: the same 4 and 16, and 4508 identical.
R10 goes from 11/11 to 11/17. veraPDF 1.30.3 passes all 8 new outputs.

**Gates.** fmt and clippy `-D warnings --all-targets` (`pdf-font`, `pdf-archive`,
`pdf-transform`): exit 0. nextest of the three: 1736 passed, exit 0. Archive tier (`corpus`,
`archive_corpus`, `witnesses`): exit 0 each. `cargo test -p conformance` after `git add -N`: exit 0.
`state.sh docs`: exit 0.
