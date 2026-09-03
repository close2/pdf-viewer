# ADR 0850 — §11.5.3's own sentence decides half of the route into grey, and three of the reports left are the file's fault rather than a debt

Status: accepted. Session 904.
Clauses: ISO 32000-2 §11.5.3, §11.3.4, §11.3.5.3, §11.6.6, §11.6.5.1, §10.3.1, §10.3.2,
§10.4.2.1, §10.4.2.2, §10.4.2.3.
Code: none — this decision reads clauses and moves no pixel. What it changes is
`doc/todo/23-transparency-departures.md` and §11.3.4's ledger row, which stated the question
wrongly.
Continues ADRs 0790, 0792, 0797, 0796.

## The question

`doc/todo/23` calls the route into a one-component space "§11.3.4's last debt" and states it as
one decision over two places:

> what decides it is that a mask and a blending space are one sentence of §11.6.6 and may not
> take two conversions. Moving both to §10.3's route is one decision, priced against the mask
> population the oracle already judges, and it has not been taken.

Round 904 was asked to take it. Reading the clauses for it found that the premise is wrong: there
are two sentences, not one, and the standard has already answered one of the two questions.

## The reading

**A `/Luminosity` mask group in a device space is settled by §11.5.3, against two references.**
The clause's device branch is one sentence and its last clause is the operative one:

> For device colour spaces, convert the colour to DeviceGray by implementation-defined means and
> use the resulting gray value as the luminosity, with no compensation for gamma or other colour
> calibration.

"Implementation-defined means" is what lets a processor pick a conversion; "with no compensation
for gamma or other colour calibration" is what bounds the pick. §10.3's route — the colour's sRGB
taken to linear light and its `Y` read — *is* a gamma compensation and a colour calibration; it is
the whole of what distinguishes it from §10.4.2.2's weights. So the clause states the classic
route for this branch, and EXAMPLE 2 then prints it.

That matters for this tree's construction rather than only for its arithmetic. A `DeviceGray` or
`DeviceCMYK` mask group is painted in the quantity §10.4.2.3 weighs (`Compositing::Luminosity`,
ADR 0220) precisely because that weighting is linear and therefore commutes with the compositing —
so the conversion *in* and §11.5.3's derivation are one function here, and a compensation applied
on the way in is a compensation in the result the clause forbids. `mupdf` and `ghostscript` put it
exactly there. **Nine renderers agreeing is evidence about a reading and never a target
(`CLAUDE.md` principle 5); two renderers disagreeing with a sentence that names their difference
is a clause read.**

**§11.6.6's conversion into a grey blending space is not settled, and the ranking that would
settle it points less firmly than it looks.** For a page or an isolated group whose `/CS` is
`DeviceGray` or `CalGray`, §11.5.3 does not apply at all: the group is painted, not masked.
§10.4.2.1 is the clause that ranks the two routes —

> Although ICC enabled PDF processors should always follow the provisions and recommendations
> provided in 10.3, "CIE-Based colour to device colour", a less-capable PDF processor may choose
> to use the algorithms specified in the following subclauses 10.4.2.2 through 10.4.2.5.

— and this tree is an ICC-enabled processor, so the *should* is addressed to it. Three things
weigh against reading it as deciding this conversion, and none of them is decisive either:

- §10.3's subject, in its own title, is "CIE-Based colour to device colour". The conversion at
  issue has a **device** source: a `DeviceRGB` mark painted into a `DeviceGray` group.
- §10.3.2 is what would make that source CIE-based, and it conditions itself: a processor should
  establish CIE specifications for device spaces "when those device colour spaces do not match
  that of the raster output device". This processor's `DeviceRGB` *is* the raster device's sRGB
  (§10.3.2, ADR 0009), so on the source side there is nothing to remap.
- The one grey round trip the standard writes out is §11.3.5.3's, for the non-separable blend
  modes: "Blending in gray colour spaces ( DeviceGray , CalGray and ICCBased gray) shall be done
  by conversion to RGB, blending in RGB, and then converting back to gray." It names no
  conversion, and the only pair the standard defines for grey and RGB is §10.4.2.2's — under
  which the round trip that sentence requires is the identity on a grey. That is evidence rather
  than a rule, because the sentence is scoped to those four modes.

So the question is genuinely open on this half, it is one half rather than two, and this decision
does not move it: moving it changes the picture on every `DeviceGray` blending space with a
chromatic mark, which is a measurement against the oracle rather than a reading.

**And the two halves may be two functions.** `doc/todo/23` treated "they may not take two
conversions" as a constraint; §11.5.3 is what makes two the correct answer if the other half ever
moves. `InkScale::grey_of` is one function serving both today, and splitting it would be following
the clauses rather than drifting from them.

## The other half of the question: which reports left are debts

The same round was asked to say, for each condition `doc/todo/23`'s two rows still fire on,
whether it should stay reported or become drawn, **derived from the clause**. Three of them are
not debts at all, and the rows had not distinguished that:

- **`Lab` as a blending or mask group space.** §11.3.4: it "shall not be used as blending colour
  spaces because the compositing computations in such spaces do not give meaningful results when
  applied separately to each component". A file stating one is outside what the clause admits.
- **A one-component space §11.3.4 does not list** — a `Separation` or an `Indexed` — which §11.6.6
  excludes outright by name.
- **A profile with no way in**: §11.3.4 requires "the ICC profile shall be capable of both device
  to PCS and PCS to device transformations", and §11.6.5.1 makes the `/CS` "the colour space in
  which the compositing computation is to be performed", so without the PCS-to-device half there
  is nothing to convert the group's marks into.

For each of those three the *document* is what departs, and the report is this reader saying so.
They stay, permanently, and the rows now say which of them is a statement about a file and which
is a construction owed. What is genuinely owed is the four-component mask (ADR 0851) and the
group-scoped conversion between two spaces at a `Do`.

## Consequences

- §11.3.4's ledger row records the split and stays `partial` on the open half.
- `doc/todo/23`'s route-into-grey bullet is corrected; its new section *What the two condition
  rows still fire on* carries the six conditions with the clause for each.
- Nothing is drawn differently by this decision, deliberately. It removes a claim, halves a
  question, and takes three rows off the list of things anybody has to build.
