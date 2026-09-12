# ADR 1001 — A source of meaning read at one call site is a route the other call sites do not take

Status: accepted, 2026-09-11. Session 980.

## Context

The round's question was the one `CLAUDE.md` principle 5 asks of colour: for every colour that
reaches a pixel, which of ISO 32000-2's two routes does it take — §10.3's ICC conversion, which
§10.4.2.1 says an ICC-enabled processor "should always follow", or §10.4.2's "crude
approximations" — and does the code say why. The tree's answer, argued in ADRs 0009 and 0042, is
that a `DeviceCMYK` colour is remapped under §10.3.2's licence by whichever of three sources the
document supplies first — `/DefaultCMYK` (§8.6.5.6), an output intent's `/DestOutputProfile`
(§14.11.5), an `ICCBased` profile — and by an assumed press, `CMYK_CORNERS`, where it supplies
none.

That is a statement about a *space*. Reading the code for the route found it was true of an
*operator*.

## What was found

`Interpreter::device_space` ranked the three sources and was called from exactly three places:
the `g`, `rg` and `k` operators. Everything else that selects a device colour space — `cs`
naming `/DeviceCMYK`, a resource name resolving to the family, a `Separation` or `DeviceN`
reverting to it, an `Indexed` base, a `Pattern`'s underlying space, and every image's, shading's,
inline image's and soft mask's own `/ColorSpace` — reached `ColourSpace::parse`, which asked
§8.6.5.6's default and stopped. The output intent was never in its signature.

So on a page carrying a readable `/DestOutputProfile`, `1 0 0 0 k` was the intent's cyan and
`/DeviceCMYK cs 1 0 0 0 scn` was the assumed press's: (1, 255, 0) against (0, 173, 239) on the
fixture, 173 levels apart in green, for one set of four numbers on one page. Trap 6 — *one
conversion, reached by every route* — with an operator on one side of it.

Three things let it survive since the output intent landed in ADR 0009:

- **The one-conversion test states no intent.** `colour_paths.rs` drives one CMYK value through
  `k`, `scn` and an image and demands they agree, and they did, because with no intent all three
  reach `CMYK_CORNERS`. A test of *one conversion* tests it under the parameters its fixture
  states; a fourth ranked source of meaning needs the same fixture with that source stated.
- **The corpus cannot see it.** `examples/raster_digest` over the 974 corpus first pages is
  byte-identical before and after the fix. The 17 documents stating a catalog intent either
  select no device space by name on page one or carry a profile this tree does not read.
- **The ledger row was about the entry, not the route.** §14.11.5's row said the intent "becomes
  what the document's device colours mean" — true of the reader, and nobody asked who called it.
  That is `doc/habits/the-ledger-and-claims-about-this-tree.md`'s "the model implements this —
  who calls it?" one level down: not a capability that never reached the program, but a source
  that reached one of the program's routes.

## Decision

- **The ranking lives in one function**, `ColourSpace::device_family`: §8.6.5.6's default, then
  the intent with the same number of components, then the device space. It is reached from the
  family name, from the array form, and from every special space's reference to the family,
  because a meaning is a property of the space and not of the operator that named it.
- **`ColourSpace::parse_with_output_intent` is the entry point that carries the intent**, and
  `parse` is it with `None`. `Interpreter::set_colour_space` and `Interpreter::device_space` call
  the first; the operand-count recovery in `set_colour` takes `device_space` too, so a malformed
  `sc` lands on the colour the matching operator would have given.
- **The array form takes the name's route.** §8.6.3 makes the array the general form — "[a]
  colour space shall be defined by an array object whose first element is a name object
  identifying the colour space family" — and the bare name its shorthand, and §8.6.5.6 remaps a
  device space "[r]egardless of how the colour space is specified". `[/DeviceRGB]` honoured no
  `/DefaultRGB` until this session, and `[/CalCMYK]` — which §8.6.5.1 says to render "as if"
  `DeviceCMYK` — now means what `DeviceCMYK` means on the page.
- **§14.11.5's row goes `partial`**, naming the routes that still parse with no intent: three
  sites in `image.rs`, two in `shading.rs`, one each in `inline_image.rs` and `soft_mask.rs`.
  None of those files was this round's to edit. Each needs the interpreter's `output_intent`
  handed to `parse_with_output_intent` where the resources are already handed; until then an
  image or shading in `DeviceCMYK` on such a page draws through the assumed press beside a fill
  drawn through the intent.

## What else the round read, and kept

- **§8.6.5.7's `should` has a price.** `data/icc/sRGB2014.icc`, read as an `ICCBased` source and
  taken through the conversion the clause says to avoid, lands within 0.88 of 255 of the
  pass-through at its worst grid point (pure cyan). `icc.rs::the_shipped_srgb_profile_is_the_identity_through_the_profile_route`
  holds it to one level. The destination half of the ICC route — `xyz_d50_to_srgb`'s folded
  matrix — and the ICC's own description of the same device agree.
- **§8.6.5.5's `/N` is a writer's `shall`** with no reader's consequence: the profile's header
  decides the component count and `/N` is read only for the fallback. The profile header's
  rendering-intent field is read nowhere, which is the clause's "a PDF reader shall ignore this
  information" by construction. **One profile class Table 67 admits is not read**: a `'Lab '`
  data colour space. `Profile::parse` answers `None` for it and the space falls to its
  `/Alternate`. That row goes `partial` for it.
- **§8.6.6.5 requires the tint transform of a display**, in its own words — "such as a display.
  To accomplish this, the colour space definition provides a tint transformation function that
  shall be used to convert all the components to an alternate colour space" — so `/Process`,
  `/Colorants` and `/MixingHints` are a `may` here. What stays owed is the `NChannel` sentence,
  which on a display asks for a per-component blending the standard does not state.
- **§8.6.4.4, §10.4.2.5, §10.4, §10.4.2**: the choice recorded there is unchanged and is now made
  in one place.

## Cost, stated

`device_family` returns an owned `ColourSpace`, so where the intent applies it clones the
intent's profile — `ColourSpace::Icc { profile: Box<Profile> }`, a deep copy of the lookup table —
once per `cs` naming a device family, as `device_space` already did once per `k`. Making the
variant an `Arc` would make both a refcount bump; `image.rs:2115` constructs the variant too, so
that change was outside this round's files and is reported rather than made.

## The habit

**When a new ranked source of meaning is added, re-run the one-conversion test with that source
stated.** `colour_paths.rs`'s first test guards "one conversion, every route" and it guards it
under the parameters its fixture names; a source added later — a default space, an output intent,
a blending space — is a parameter the fixture does not name, and every route agrees under its
absence exactly as they did before it existed. The instrument to ask is not "does the test still
pass" but "which call sites read the new source", which is one `grep` for the field's name.
