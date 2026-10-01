# 1315 — A right-to-left word is found as it is typed

The HOST-UI round of batch forty-nine. ADRs 1465 and 1466. No ledger row moved; no question.

**Found (ADR 1465).** `ArabicCIDTrueType.pdf` reads back presentation forms in display order
(§9.10.2 hands its `/ToUnicode` through; §14.8.2.5.3 NOTE 1's "reverse order" show strings). The
search folds a form to its `UnicodeData.txt` decomposition on both sides (`pdf_font::shaping::fold`),
spells the needle as UAX #9 displays it under both paragraph directions, and admits each spelling
only over the right-to-left runs whose glyph positions say the page stored that order
(`select::Order`, cached beside the readback). Tests: `select.rs` six, `shaping::fold` two,
`headless.rs` two (the fixture, highlight at each line's left end; a Type 3 page storing "عرب"
three ways); with the order planted out both headless tests fail. The first table was
`[(char, &str)]`, nine hundred relocations before `main`: `launch_path` read
`xfa_filled_imm1344e.pdf`'s open at 1826.25 k instructions (ceiling 1820); as a letter pool
indexed by `(u16, u8)` it is 1818.73.

**Wheel and popup (ADR 1466).** Control and the wheel zoom about the pointer in all three windows
through one `viewer_host::wheel::ZoomWheel`. §12.5.6.14: "It shall have no appearance stream or
associated actions of its own"; Qt drew no edge, GTK no fill, and both now draw `quorra`'s paper
and edge (`viewer_host::popup`). Qt's form controls carry their §14.9.3 names.

**A refused frame, out loud (ADR 1466 section 5).** `quorra` and `quorra-confined` said a
device-refused frame on the terminal only; the sentence is now in the title too, written when it
changes. `ContentStreamCycleType3insideType3.pdf` under lavapipe: 377 221 248 scene bytes against
268 435 456, the page drawn, the title saying so. GTK and Qt draw no page on a device.

**Coordinates.** `tools/drive-windows.sh` runs one private AT-SPI bus and asks `GetExtents` for
the outline rows, the pages tab and row, the check box, the choice and the find bar: 16 asked, 4
measured (`quorra`'s drawn panel rows and tab; accesskit bounds only document nodes).

**Driven.** Release, `Xvfb`: 70 works, 0 wrong, 10 looked at; `25-find-arabic` and
`27-processor-fallback` work. An earlier run read `quorra-qt`'s `01-center-window` at 0,0 once.

**Gates.** fmt; clippy `--no-deps` (6 crates); nextest 918; `conformance`; `launch_path`; both
censuses: exit 0. With deps, clippy stops in a sibling's `pdf-model/src/image.rs`.

**Left.** Diacritic folding; a mirrored glyph votes the wrong way; `quorra` takes two zoom steps
per `xdotool` notch under Xvfb (so did the old binary); `quorra-confined`'s title is undriven:
on that page its worker sent pixels, so its device had nothing to refuse.
