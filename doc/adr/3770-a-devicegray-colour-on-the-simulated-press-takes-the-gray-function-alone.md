# 3770 — A `DeviceGray` colour on the simulated press takes the gray function alone

Session 2267. Status: accepted. Context: ISO 32000-2 §10.5, §10.4.2.3, §10.8.3, §8.6.5.6; review
2248's B04; ADRs 2758, 2765, 3655 (how `DeviceGray` reaches a press), 3769.

## The finding

§10.5: "When the current colour space is DeviceGray and the output device's native colour space is
DeviceCMYK , a PDF processor shall use only the gray transfer function. The normal conversion from
DeviceGray to DeviceCMYK produces 0.0 for the cyan, magenta, and yellow components. These components
shall not be passed through their respective transfer functions but are rendered directly". On the
simulated press `/TR [zero zero zero identity]` over `1 g` drew black, through the screen's three.

## The decision

`DeviceGray` reaches a CMYK device in this tree as one space: §10.4.2.3's black ink alone, which
`output_intent::grey_reading` chooses (`GreyReading::IntentBlack` under a CMYK intent,
`AssumedInksBlack` on the simulated press of the assumed inks) and `pdf_colour`'s `intent_grey`
builds (`ColourSpace::is_grey_as_intent_black`). Neither function changes: they already are the
place the special case's precondition is decided.

`Interpreter::mark_transfer` gives a fill or a stroke painted in that space `Transfer::grey_alone`:
the press's black function, and no function on cyan, magenta and yellow, which are 0 and rendered
directly. `TransferState` derives it beside the composed transfer, so every grey mark under one
graphics state shares one `Arc` and one sampled map. The screen's three are kept in it, because the
special case is a CMYK device's: on the screen, and on a CMYK intent's preview, which is the
screen, nothing changes.

A `/DefaultGray` outranks the reading by §8.6.5.6 and is then a CIE-based colour, not a `DeviceGray`
one; a shading and an image carry their own spaces and keep the full transfer. An image whose own
space is `DeviceGray` is the clause's "colours specified in the DeviceGray colour space" as much as
a fill is, and is left (the report of round 2267).
