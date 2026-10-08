# 1764 — A luminosity group's default backdrop is the initial colour of the family its `/CS` states

Session 1465. Status: **accepted** and **built**. Context: ISO 32000-2 §11.6.5.1 Table 142 (`/BC`),
§8.6.8 (the initial colours), §14.11.5 (output intents); ADR 1755, whose §5 left this site open.
Code: `backdrop_values` and `backdrop` in `crates/pdf-model/src/soft_mask.rs`;
`ColourSpace::initial_colour_of` in `crates/pdf-colour/src/colour.rs`.

## 1. The defect

Table 142 gives `/BC` the default "the colour space's initial value, representing black". `luminosity`
parses the group's `/CS` through `ColourSpace::parse_with_output_intent`, so a `/DeviceCMYK` group on a
page whose output intent is a four-component profile is the profile, and `backdrop_values` took
`space.initial_colour()` of *that*: `[0 0 0 0]`, no ink, the paper. §8.6.8 starts `DeviceCMYK` at
`[0.0 0.0 0.0 1.0]`, which is the black the table names. A PDF/X page that masks through an empty
`/DeviceCMYK` luminosity group showed what the mask covered at full strength where the table makes it
almost transparent.

## 2. The reading

ADR 1755 §2 is the argument and nothing here adds to it: §14.11.5 describes the device a document's
colours were prepared for and replaces no colour space, so the group's space is still `DeviceCMYK`, and
so is the initial colour Table 142 asks for. A group attributes dictionary is not drawn from a
resource dictionary, so no `/DefaultCMYK` can stand in here; the output intent is the only route.

## 3. What was built

`backdrop_values` is handed the `/CS` entry as the file states it beside the parsed space, and the
default is `space.initial_colour_of(document, stated, &Dictionary::new())` — the one function `cs`
already asks, so the two readings of "initial colour" cannot drift apart. A stated `/BC` of the right
length is read as before.

## 4. Fixture and population

`soft_mask::tests::a_cmyk_mask_groups_default_backdrop_is_the_familys_black_under_an_intent` paints
black through an empty `/DeviceCMYK` mask group under an intent whose press darkens by `0.9 k`: the
default now draws 230, the level `/BC [0 0 0 1]` draws, and the old line planted back draws 0 —
`/BC [0 0 0 0]`'s level, 230 levels away. HEAD's six corpus arms and this change's agree digest for
digest, none of 5 795 pages moved: no corpus page masks through a defaulted `/BC` under a CMYK intent,
so the population is the fixture (trap 8), which fails with the old line planted.
