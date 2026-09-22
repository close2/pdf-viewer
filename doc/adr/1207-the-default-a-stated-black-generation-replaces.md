# 1207 — The default a stated black generation replaces

Status: accepted. Session 1185.
Supersedes: ADR 1069, on a premise about this tree's own code that ADR 1069 stated and did not
check.
Context: ISO 32000-2 §8.4.5 (Table 57), §10.3.1, §10.3.2, §10.4.2.1, §10.4.2.4, §11.6.7, §11.7.2,
§11.7.5.3; ADRs 0009, 0042, 0262, 0263, 0796, 1054, 1131, 1157, 1194.
Code: `crates/pdf-colour/src/black_generation.rs` (new), `crates/pdf-colour/src/colour.rs`,
`crates/pdf-model/src/content.rs`, `crates/pdf-model/src/content/{colour,ext_gstate,pattern,
run,transparency}.rs`, `crates/pdf-model/src/soft_mask.rs`.
Tests: `black_generation.rs::the_stated_functions_are_the_ones_the_clause_calls`,
`black_generation.rs::table_57_decides_which_of_each_pair_is_in_force`,
`transparency_groups.rs::a_device_rgb_colour_in_a_cmyk_group_is_separated_by_the_stated_functions`,
`transparency_groups.rs::a_stated_black_generation_is_reported_and_the_page_keeps_its_space`.

`§N` is ISO 32000-2 and nothing else.

## 1. The premise that expired, and it was about this tree rather than about the standard

ADR 1069 decided that §11.7.5.3's black-generation bullets "select among functions a conversion
uses; they do not require a processor to use one", on this ground:

> This tree's conversion of a colour into a four-component group is §10.3's — the profile's own
> `B2A` table where the press has one, the ink cube's inverse where it does not (ADRs 0009, 0042,
> 0263, 0796) — and neither has a black-generation step for a stated function to replace.

The first half of that sentence is true and is still the design. **The second half is false, and
the code that falsifies it is forty lines from the function the ADR named.** `colour::INK_LADDER`'s
own doc comment:

> How many black generations [`rgb_to_ink`] tries after §10.4.2.4's nominal one.
>
> Twelve, from all the black there is down to none.

and, on why the ladder reaches above the nominal black as well as below it, that a
black-generation function is free to "return a larger value for extra black" — this clause's own
permission, cited in this tree's own comment, to justify the range of the search. A search that
tries twelve black amounts and picks one **is** a black generation; the undercolour removal is in
it too, because having fixed the black the search solves the other three for the colour, which is
what §10.4.2.4 calls compensating "for the amount of black added by black generation".

This is trap 40's shape exactly: the capability a refusing function says does not exist was
forty lines above it.

## 2. The clause, read again from the step rather than from the branch

Three sentences decide it and they are in three clauses.

**§10.4.2.4 requires a device to have the pair.** Its last paragraph, after the formula: "The
correct choice of black-generation and undercolour-removal functions depends on the
characteristics of the output device. Each device shall be configured with default values that are
appropriate for that device." So a conversion from `DeviceRGB` to `DeviceCMYK` *always* uses a
black generation and an undercolour removal. The only question a file can settle is whose.

**§11.7.5.3's first bullet settles it, for the colours it names**, and the clause's lead sentence
restricts where the bullets bite — the functions "shall be applied only during conversion from
DeviceRGB to DeviceCMYK colour spaces":

> When painting an elementary object with a DeviceRGB colour directly into a transparency group
> whose colour space is DeviceCMYK , the functions used shall be the current black-generation and
> undercolour-removal functions in effect in the graphics state at the time of the painting
> operation.

Read against §10.4.2.4's last paragraph this is not a demand to change algorithms: it says that
where the file states a pair, the file's pair is the one used instead of the device's default. The
device's default here is ADR 0263's search — which §10.4.2.4's row has recorded as this device's
default since session 426, in those words, while §11.7.5.3's row went on saying there was no step.

**§10.4.2.1's ranking is untouched by that**, and this is the half ADR 1069 was right to insist
on. It states a `should` about which *route* a processor takes — "ICC enabled PDF processors
should always follow the provisions and recommendations provided in 10.3" — and this tree follows
it. What a stated pair changes is one parameter of one conversion, over the colours §11.7.5.3's
own sentence names, on a page that asked for it in writing. Nothing else on the page moves branch,
and a page stating no function converts exactly as it did.

**And §10.3 does not claim this conversion for itself.** §10.3.2 tells a processor to remap a
device space into a CIE-based one "when those device colour spaces do not match that of the raster
output device", and in the same breath hands one device-to-device conversion back to §10.4.2:
"If the native device colour space is CMYK, then converting colours in the DeviceGray colour space
to that CMYK should follow the method described in 10.4.2.3". So §10.3's own text expects §10.4.2's
algorithms to be reachable from inside the recommended route. (That sentence bears on §10.4.2.3's
departure, ADR 1194, and this ADR does not take it: a `DeviceGray` colour is not the "DeviceRGB
colour" §11.7.5.3's bullet conditions on, and the grey case has no file-stated parameter.)

## 3. What is built

**`pdf_colour::black_generation`**, a module beside `transfer` and on the same boundary (ADR 1131):
`pdf_model::content` reads Table 57 and this holds the pair and the arithmetic, so a shading's ramp
and a mesh's vertices can be separated without the colour crate depending on the interpreter.

- `BlackGeneration` is two `Option<Arc<Function>>`. A half that is `None` is §10.4.2.4's nominal
  function, `f(k) = k`, which the clause describes for each of the two in those words and which is
  the pair that makes §10.4.2.5 an exact inverse — already recorded in `colour::rgb_to_cmyk` as
  this device's defaults *for this formula*. A state with neither half stated holds no pair at all
  and converts as before.
- `BlackGeneration::read` applies one `/ExtGState` to the pair in force. Table 57's precedence is
  read per pair ("[i]f both BG and BG2 are present in the same graphics state parameter
  dictionary, BG2 shall take precedence", and the same of `/UCR2`), and the `2` variants' name
  `Default` — "the black-generation function that was in effect at the start of the page" — clears
  that half back to the device's, which is a third answer distinct from saying nothing. §10.4.2.4
  makes a stated function "defined as PDF function dictionaries (see 7.10, "Functions")", so a
  name on the plain entry is not one.
- `BlackGeneration::separate` is the clause's formula with the outer clamps on the *results* — not
  on what a function returned, because Table 57 gives `/UCR` the range `[-1.0 1.0]` precisely so
  that it may add colourant rather than remove it.

**The pair is a graphics-state parameter** — `GraphicsState::black_generation`, saved and restored
by `q`/`Q` — and it reaches a conversion the way §8.6.5.9's black point already does, through
`colour::Conversion`, which §11.7.5.3's own first paragraph lists the four parameters together in.
`ColourSpace::to_cmyk_under` has the one arm that uses it, on the leaf rather than the operand: an
`Indexed` entry or a `Separation`'s alternate arrives there as the `DeviceRGB` colour the bullet
names, and a `CalRGB`, `Lab` or `ICCBased` colour never does.

**Table 57 has two routes and both are read.** `content/ext_gstate.rs` for `gs`, and
`PatternInitial::augmented` for a shading pattern's own dictionary — §11.6.7's third bullet admits
"those parameters that affect the sh operator, such as the current transformation matrix, black
point compensation and rendering intent", and the list is "such as" rather than closed: this pair
decides what the ramp's colours become in a `DeviceCMYK` group, so it affects `sh` on the bullet's
own test. That cost `PatternInitial` its `Copy`.

## 4. What is *not* built, and the report is now about exactly that

`Unsupported::BlackGeneration` used to fire wherever a function was stated and four components were
converted into. It now fires on the two moments that remain, and its condition names each:

- **§11.7.5.3's second bullet** — a group this tree composites on the device's three components
  painted into a four-component parent. That conversion is a cube resolved per pixel in a backend,
  over a composited raster, where no colour space and no graphics state exist; the functions in
  effect at the `Do` reach nothing. `Interpreter::into_parent`'s keys are the test, and it
  over-approximates by exactly as much as `Compositing::Device` is wider than `DeviceRGB`.
- **A press sampled from the document's own bi-directional profile**, whose `B2A` is the
  conversion in (§8.6.5.5, ADR 0796). That is the document's measured transform rather than a
  device default a stated pair stands in for, and substituting §10.4.2.4's arithmetic there would
  convert into a profile's space by a formula that never reads the profile.

**The third bullet is inapplicable and this ADR does not move it.** It conditions on "the native
colour space of the output device" being `DeviceCMYK`; ADR 1157 makes a *group* the device a rule
about the group's colourants addresses, and this bullet is about the page group's conversion onto
the device, whose native space here is sRGB (ADR 0009).

**Trap 5's distinction is the one to hold on to**: this population stops being reported because the
clause is carried out, not because a condition was tuned until it stopped matching. The test keeps
a case where the report still fires, so the narrowing cannot be mistaken for the other thing.

## 5. What it costs, measured

`a_device_rgb_colour_in_a_cmyk_group_is_separated_by_the_stated_functions` draws §10.4.2.4's own
EXAMPLE colour, `0.2 0.7 0.4 rg`, into a `DeviceCMYK` page group under a state whose `/BG` is
`1 − x` and whose `/UCR` is `x ÷ 2`. `k = min(0.8, 0.3, 0.6) = 0.3`, so the clause gives
`(0.65, 0.15, 0.45, 0.7)`; the control, the same page with no `gs`, is the press's search, which
reproduces the stated colour. The two differ, and the test requires them to — a stated pair that
changed nothing would witness nothing.

**A page that states no function is unchanged, bit for bit**, which is the whole population of the
corpus but one document: `black_generation_census` counts 1 of `doc/pdf.js`'s 963 and 189 of the
39 127 web documents stating a *function* at all, and a page moves only where such a statement
meets a `DeviceCMYK` group.
