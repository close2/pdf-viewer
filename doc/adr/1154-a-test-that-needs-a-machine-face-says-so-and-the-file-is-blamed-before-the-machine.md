# 1154 — A test that needs a machine face says so, and the file is blamed before the machine

Status: accepted. Session 1158.
Context: `crates/pdf-font/src/loading.rs` (`composite_substitute`,
`LoadedFont::machine_offers_a_substitute`, `load_composite`, `corpus_bare_cff_fonts`),
`crates/pdf-model/tests/hostile_budgets.rs`, `crates/pdf-model/tests/composite_fonts.rs`,
`crates/pdf-model/tests/text_state.rs`, `crates/pdf-model/tests/substituted_shapes.rs`,
`crates/pdf-model/tests/silent_fonts.rs`, `crates/pdf-model/tests/variable_text.rs`,
`crates/pdf-model/tests/vertical_forms.rs`, `.github/workflows/ci.yml`,
`doc/habits/tests-gates-and-reports.md`.
Builds: ADR 0152's covering search, ADR 0358's substitute scale, ADR 0433's reading of §9.7.5.2,
and the skip commit 02e4be0e started. No rendering change on any machine that has fonts.
Clauses: ISO 32000-2 §9.7.4.2, §9.7.5.2, §9.10.2, §9.2.4.

## 1. What was measured

The project's CI failed at `pdf-model`'s `hostile_budgets`, and the whole workspace was then run
with the machine's font directories replaced by an empty `tmpfs` in a mount namespace. **Twelve
tests in eight binaries** fail there and nowhere else, beside the two commit 02e4be0e had already
taught to skip. Every one of them is a measurement of the instrument: what they report is which
faces the machine has. Two more, in `pdf-vfs` and `viewer-confined`, are *about* the machine's
face — a broker handing one to a confined worker — and are left alone here; `pdf-vfs`' own
assertion message already names the condition it fails on.

The reproduction is worth writing down: `unshare -rm --propagation private`, a `tmpfs` over
`/usr/share/fonts`, and an empty `HOME` — because `substitute::font_directories` walks
`$HOME/.fonts` too.

## 2. Why a composite font is the one that depends on a machine at all

§9.6.2.2's fourteen faces are compiled in (ADR 0133), so a *simple* substituted font needs no
machine. §9.7.4.2 leaves a composite font whose program the document did not embed reachable only
by character — "CIDs shall not participate in glyph selection" — so its stand-in has to answer a
`cmap` lookup, which an `sfnt` does and the compiled-in name-keyed CFF faces cannot. A machine
with no `sfnt` face therefore draws no such font, and that is a fact about the machine.

So the rule is the one `doc/habits/tests-gates-and-reports.md` now carries: a test whose fixture
is a composite font with no `/FontFile` asks whether this machine can stand in, and skips with a
printed sentence where it cannot. Eleven tests in five files do (`hostile_budgets`,
`composite_fonts`, `text_state`, `variable_text`, `vertical_forms`). **It asks `pdf-font`, not the shape of a refusal.** A predicate
matching a message's words would swallow a refusal the code under test had started giving for
another reason, and `LoadedFont::machine_offers_a_substitute` runs `composite_substitute` — the
same search the load runs — so there is no second matcher to drift from the first.

## 3. The file is asked before the machine is

`load_composite` searched the machine for a face and *then* asked whether §9.10.2 gave the codes
any character to reach one by. Both refusals are `FontError::NoSubstitute`, and only the second is
about the document: on `issue6127.pdf`, whose `/C2_7` and `/C2_14` break §9.7.5.2's "[t]he
Identity-H and Identity-V CMaps shall not be used with a non-embedded font", a reader with fonts
was told the file's fault and a reader without was told about its own machine. The order is now
the other way round, which costs a catalogue walk that could never have helped, changes nothing
about which fonts load, and makes the sentence a producer's fault earns independent of who reads
it. `silent_fonts.rs` passes with no fonts installed as a consequence rather than by skipping.

## 4. Two defects the fontless run exposed, neither of them a skip

**The bare-CFF corpus population was a function of the machine.** `corpus_bare_cff_fonts` kept
every first-page font whose program was `Program::BareCff`, and the compiled-in faces are bare
CFF — so a substituted font arrived looking like an embedded one. Nine corpus fonts did on a
machine with no fonts, and `ICC-1_1998-09.pdf` `/F19` then failed the widths test with a Foxit
face's advances against a producer's array. Five join it on a machine *with* fonts and passed by
luck. All three tests over that population reason about the program the producer shipped —
`metrics::substitute_stretch` says so from the other side — so the population is now the embedded
ones.

**`substituted_shapes` asserted more than §9.2.4 gives.** `bounds.width() <= stated`, per letter,
is not a property of any font: a width is a *displacement*, so ink overhanging it is a negative
side bearing and ordinary typography. Which letters do it is a fact about the face — none of this
machine's Arial-metric ones, the compiled-in fallback's `y`, `DejaVuSans`' `f` and `t` — so the
per-letter form was an assertion about what is installed, which is what the file's own opening
paragraph says it is not making. A margin wide enough for `f` (19 units of the 1000-per-em grid)
would not have separated it from the defect either: unscaled, `f` is 110 units over.

So the claim is a **share** — at most a quarter of the line, where 2 of 19 is the worst of four
faces measured — and its control is arithmetic rather than a second measurement: the ink without
the scale is the ink with it divided by `stretch`, and the test asserts that *most* of the line
would be over that way. It is 14 or 15 of 19 on every face tried. Planting the defect back by
using the unscaled figure in the first assertion fails it at 14 of 19, which is trap 13's
calibration done at the test's own measuring point.

## 5. And one fixture stopped depending on the machine altogether

`composite_font_stating_ranges` drew `<0001>`, which its `CMap` sends to CID 2 and
`Adobe-Japan1-UCS2` makes `!`. The face that substitutes for it is chosen by whether it covers あ,
and `DroidSansFallback` covers あ and has no Latin at all — so on a machine carrying that face the
font loaded, the skip did not fire, and the control drew nothing: which is how the *second* CI run
of these tests failed, on a runner whose `ghostscript` install had brought that font in. It draws
`<034a>` now — CID 843, the first character of `Adobe-Japan1-UCS2`'s own hiragana range
`<034a> <039c> <3041>` — so the code shows the character the face was **chosen** for, and "drew
nothing" can only mean the font failed to load.

The general rule, for the next fixture of this shape: a substituted composite font's face is
chosen against `script_sample`'s characters and nothing else, so a fixture may only show a code
that reaches one of them. Where the collection implies no script the sample is empty and the face
is matched by family instead, which is `substitute::PREFERENCES` — Latin text families throughout,
so a Latin code is safe there for the same kind of reason, one step weaker.

## 6. What CI installs, and why two packages

`fonts-dejavu-core` and `fonts-droid-fallback`, and they answer different halves of the search.
`installed` matches by *family* against `substitute::PREFERENCES`, so a font whose collection
implies no script is answered only by a name on one of those lists — `DejaVuSans` and
`DejaVuSerif` are. `installed_covering` matches by *coverage* and walks the whole catalogue, so
anything carrying U+3042 answers an `Adobe-Japan1` font, and `DroidSansFallback` does.

**Both are already on the runner, and the step is named anyway.** `ghostscript` pulls in
`libgs10-common`, which *recommends* `fonts-droid-fallback`; `fontconfig-config` depends on an
alternatives list beginning `fonts-dejavu-core`. A recommend and an alternative are both things
apt can stop doing without this repository changing, and what would then happen is seven tests
quietly skipping. A test's requirement belongs to the workflow. Verified against the noble package
indexes, and by running these tests in namespaces holding one candidate package's files and
nothing else: `fonts-misaki` (306 kB) and `fonts-sawarabi-mincho` (841 kB) are the smallest that
carry hiragana in an `sfnt` if the droid package ever goes, against 60 MB for `fonts-noto-cjk`.

The point of naming them is that the eleven tests that would skip **run** there — measured: with
these two faces and nothing else, none of the eight binaries prints a skip. A skip is what an
honest test does on a machine it cannot measure on; it is not what a CI runner should be doing
every push.
