# 3807 — A `DeviceGray` image's samples take the gray function alone on the simulated press

Session 2276. Status: accepted. Context: ISO 32000-2 §10.5, §10.4.2.3, §10.8.3, §8.9.5, §8.9.6.2;
ADRs 3655, 3770.

§10.5's special case is "colours specified in the DeviceGray colour space": "a PDF processor shall
use only the gray transfer function. The normal conversion from DeviceGray to DeviceCMYK produces
0.0 for the cyan, magenta, and yellow components. These components shall not be passed through
their respective transfer functions but are rendered directly". An image's samples are colours
specified in its `/ColorSpace` (§8.9.5), and on the press this tree carries a `DeviceGray` sample
as §10.4.2.3's black ink alone, with cyan, magenta and yellow painted at 0 on the chromatic plane.
ADR 3770 gave a fill or a stroke `Transfer::grey_alone` and left images with the full transfer, so
the chromatic plane's 0 went through the first three functions — under `/TR [zero zero zero
identity]` a white grey image printed full cyan, magenta and yellow.

`Interpreter::image_transfer` gives an image `grey_alone` where its own space, read under the
interpreter's `reading()` as the decode reads it, is the black-ink space, and a stencil mask where
the current non-stroking space is (§8.9.6.2: its colourants are the current colour's). A
`/DefaultGray` outranks the reading by §8.6.5.6 as it does for a fill. The entry is parsed only
where a function is in force and `DeviceGray` can be the black ink at all. An `Indexed` space over
`DeviceGray` stays an `Indexed` colour, as it does for a fill. Tested on every lane by
`press_transfers`' grey image.
