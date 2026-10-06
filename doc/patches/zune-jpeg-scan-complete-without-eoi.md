# zune-jpeg: a baseline scan that holds every MCU but no EOI loses its last MCU row to grey

For the owner to file at <https://github.com/etemesi254/zune-image/issues>, under `zune-jpeg`.
The fix is [`zune-jpeg-scan-complete-without-eoi.patch`](zune-jpeg-scan-complete-without-eoi.patch),
which `close2/zune-image` carries until a release does (`doc/questions/A227`, ADRs 1520 and 1589).

## The issue

**Title:** zune-jpeg 0.5.15: the last MCU row of a complete scan with no EOI is filled with 128

**The sentence:** when a baseline scan's entropy-coded data holds every MCU the frame header
counts but the stream ends with no `EOI` after it, the decoder fills the last MCU row with 128,
because the bit reader's lookahead reaches the end of the data while that row's bits are still
buffered, `overread_by` turns positive on that read, and the MCU loop in `src/mcu.rs` stops at the
next row on `overread_by > 0` alone; ITU-T T.81 section E.2.3 ends a scan on its MCU count, so the
row is in the data, and the patch stops a row only once the zeros appended past the end, or more
bits than the buffer held, have actually been consumed.

Found by fuzzing (a target comparing whole and banded decodes). Evidence for the fix: a frame with
an `EOI` decodes to the same bytes as before (27 frames, lenient and strict); over every truncation
of fifteen `cjpeg` fixtures the patched decoder never yields fewer rows equal to the whole frame's;
a whole decode's instruction count does not rise (callgrind, 510.2 M before and 509.0 M after, on
a 4 MB photograph).

## Reproducer

A 348-byte codestream, one component, 100 x 107 (hexadecimal; `xxd -r -p` makes the file). Without
the patch lines 104 to 106 are 128; with it they equal the decode of the same bytes with `FF D9`
appended.

```text
ffd8ffe000104a46494600010100000100010000ffdb004300100b0c0e0c0a100e0d0e1211101318281a181616183123
251d283a333d3c3933383740485c4e404457453738506d51575f626768673e4d71797064785c656763ffc0000b08006b
006401011100ffc400190001000301010000000000000000000000000102030407ffc400191001010101010100000000
000000000000000102111203ffda0008010100003f00f3f0000000048701000253c4f0e1c388e23480129916917994f9
4f856e55b956c56a0131791a672d6656984f856e19eb2cf519d54168d331b6236ce5acc26e14d658ef2c3719695a8168
d72dfe6e8c46d989b19ea30dc73ed8e94a8168d32e8f9d7462b6cd5ad67bae7fa5736eb2d29502634cd6d8adf1a6b9da
6ed4d6986f4c3759d56a04c5a5699d34ceda4da7dabadb2d699eaa955013169569a5a693ed5ba56e95b508004a7a74e9
d47440000000000000037fd9
```

Decoded with the program in
[`zune-jpeg-dc-prediction-overflow.md`](zune-jpeg-dc-prediction-overflow.md), once as it is and
once with `FF D9` appended, comparing the last 3 x 100 x 4 bytes.

## The patch

Against `31d81fed7551c8ccea456d9d8e2b1fd8bebb6995` (0.5.15 as published), in `crates/zune-jpeg`:
`src/bitstream.rs` counts the zeros appended once the data has ended and records an exhausted
buffer, and `src/mcu.rs` stops on `stream.consumed_past_end()` instead of `overread_by > 0`. The
whole diff is the `.patch` file beside this one.
