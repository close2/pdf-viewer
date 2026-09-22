# 1183 — A print has a quality, and a page has a place on the sheet

The HOST-UI round of batch twenty-eight: RFC 0004's second pass, on `doc/todo/65`'s two expired
premises — ADR 0803 section 1 and ADR 1057 section 4.

## What was found

**Table 22's bit 12 states two consequences, and this tree read one as a silence.** Bit 3 clear
withholds printing; bit 12 clear *limits a print that goes ahead* — the cell's own parenthesis,
"(and bit 3 is set)". `Bit::PrintFaithfully` said the implementation-dependent algorithm was one
"this tree has not chosen" — a claim that decays: what leaves this program is a raster, so the
algorithm is a resolution and a destination, both already in `viewer_host::printing`.

**Session 1171 recorded that no bit-3 fixture exists and a forty-minute walk was killed looking.**
`pdf-model/examples/encryption_census`, given both printing positions to report, walks `doc/pdf.js`
in about a second and finds `secHandler.pdf` (`/V 5 /R 6 /P −3136`), which clears bit 3 *and* bit
12; the four levels over bit 12 are tested against it. **No document there permits printing and
withholds only its fidelity**, so the pair the cell's second sentence is written for is untested
against a real file, and the wider submodules are not checked out here.

**No window assembles, and bit 11 is gated anyway** — `pdf-vfs`'s `InsertPages` and `DeletePage`
ask `Operation::Assemble` — so the sentence holding two rows at `partial`, "operations this program
does not have", was false for bit 11 too. The whole §7.6.4 family is `implemented` as a result.

## What was built

`Operation::PrintFaithfully` through the policy, the wire, the C ABI and the windows' menu;
`Fidelity` on the grant, with the floor and the refused file destination carrying it out;
`Held::PrintFaithfully`, the one held question whose `no` starts a job. `view::TargetMedia` — a
media rectangle and what B scales the page by — with `fixed_print` dividing by it, and
`printing::placed`/`cell` composing fit-to-page and n-up.
**`quorra-qt` prints**: a hand-written `QPrintDialog`/`QPrinter` path with the scale mode and the
pages per sheet on a tab of its own, behind a `QtPrintSupport` the build script and the C++ each
ask for separately — and no new `unsafe` token.

## What is left owed

Page *tiling* (the model expresses it, no host composes one); the winit panel and IPP; the spool
container; §12.2's `/PrintArea` and `/PrintClip`, now work rather than a capability nothing uses.
**`QPrinter` was compiled, linked and not driven**: a job is somebody else's paper here.
