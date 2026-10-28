# 2549 — A stated black generation replaces the profile's table, and the table is the default

Status: implemented. Session 1857. Q723 is not asked.
Context: ISO 32000-2 §11.7.5.3, §10.4.2.4, Table 52, Table 57, §8.6.5.5, §14.11.5; ADRs 0796, 1207, 1242,
1622, 2170, 2187. Code: `crates/pdf-colour/src/colour/to_cmyk.rs` (`to_cmyk_at`, `to_press`),
`crates/pdf-colour/src/colour/conversion.rs` (`Conversion::black_generation_in_force`),
`crates/pdf-model/src/content/colour.rs` (`Interpreter::colour`),
`crates/pdf-model/src/content/transparency/group_runs.rs` (`group_commands`).

## 1. Decided

**A `DeviceRGB` colour painted into a `DeviceCMYK` group under a pair the graphics state states is
separated by §10.4.2.4's arithmetic with that pair, whichever press the group is composited in — the
one sampled from the output intent's bi-directional profile included — and where the state states
none the conversion is the press's own: the profile's `B2A` table for that press.** The departure
ADR 1242 section 2 left, and `Unsupported::BlackGeneration` with it, are gone; so is the tracking
that fed the report (`black_generation_painted`, `black_generation_bypassed` and
`bypasses_black_generation` through the image, shading and pattern caches, ADRs 2187, 2258).

## 2. Why the table is the default and the pair is not refused

§10.4.2.4 closes: "The correct choice of black-generation and undercolour-removal functions depends
on the characteristics of the output device. Each device shall be configured with default values
that are appropriate for that device." Table 52 gives both parameters the same initial value: "a PDF
reader shall initialise this to a suitable device dependent value". The press sampled from the
output intent's profile is the device the group composites on, and its `B2A` table is how this
reader converts into it when the file says nothing (§8.6.5.5's "from CIE" information, ADR 0796).
So the table *is* that device's default, in the only sense §10.4.2.4 gives the word, and a stated
pair is what §11.7.5.3's first bullet puts in its place: "the functions used shall be the current
black-generation and undercolour-removal functions in effect in the graphics state at the time of
the painting operation". ADR 1242 read the table as "the document's measured transform rather than
the device default a stated pair substitutes for"; the text has no third category between a device
default and a stated function, and a `shall` naming the stated one leaves nothing for the table to
win on. The arithmetic's result is the group's own four components, so no table follows it.

The second bullet changes with it. `conversion_into_parent` already took the pair at the `Do`
(ADR 1242), and `to_cmyk_at` sent it past a profile's press without a word — a departure no report
named. The same arm now carries both bullets.

## 3. The target is the caller's, and an `ICCBased` group is not one

§11.7.5.3: the functions "shall be applied only during conversion from DeviceRGB to DeviceCMYK
colour spaces". A group whose blending space is `ICCBased` CMYK, stated or by `/DefaultCMYK`, is
what `Interpreter::cmyk_group` names, and ADR 2170 decided its `B2A` route *is* the clause. Only
the caller knows the group's space, so the pair is withheld there: `Conversion::black_generation_in_force`
for every mark the caches convert, `Interpreter::colour` for the rest, the parent's scope at the
`Do`. **One moved case follows, and it is the clause's**: an isolated `ICCBased` group whose profile
has no `B2A` used to take a stated pair, because `to_cmyk_at` asked only whether the press converted
in by its profile; it now keeps the right inverse of its grid.

## 4. Consequences

- §11.7.5.3 moves `departed` → `implemented`; `doc/todo/65`'s paragraph goes.
- Tests: `a_device_rgb_colour_into_the_output_intents_press_is_separated_by_the_stated_functions`
  (both branches and the `ICCBased` control) and
  `a_device_rgb_group_into_the_output_intents_press_takes_the_pair_at_the_do`; the three tests that
  pinned the report are rewritten as pins of the conversion;
  `a_device_rgb_group_into_an_icc_based_parent_takes_no_pair_at_the_do` holds the parent's scope.
- An entry that is not a function — a dictionary §7.10 cannot build, a name Table 57 does not give
  the entry, any other object — sets nothing, as before, and is now said:
  `Unsupported::UnreadableBlackGeneration`, the file's (principle 1). Removing the departure would
  otherwise have left the reader no sign that a page's `DeviceRGB` paints into a `DeviceCMYK` group
  were separated with functions the file did not state there.
- No loop, recursion or allocation is new: the arithmetic replaces a table lookup per colour, and the parent
  cube per distinct pair at a `Do` is ADR 1242's memo, keyed as before. A page of 2 000 isolated `DeviceRGB`
  groups, each after its own `/BG`, over a CMYK output intent, interpreted by the debug
  `callgrind_interpret`: 707 MiB peak resident and 7.3 s against HEAD's 707 MiB and 57.3 s (load 47–50),
  HEAD reporting its one departure and this tree none. The 707 MiB is the memo's, at HEAD as here.
