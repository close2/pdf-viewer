# zune-jpeg: the DC dequantisation overflows where the DC prediction beside it wraps

For the owner to file at <https://github.com/etemesi254/zune-image/issues>, under `zune-jpeg`.
The fix is [`zune-jpeg-dc-prediction-overflow.patch`](zune-jpeg-dc-prediction-overflow.patch),
which `close2/zune-image` carries until a release does (`doc/questions/A227`, ADR 1589).

## The issue

**Title:** zune-jpeg 0.5.15: "attempt to multiply with overflow" in `bitstream.rs` on a hostile DC
prediction

**The sentence:** `decode_dc` updates the DC prediction with `wrapping_add`
(`src/bitstream.rs:332`), but the dequantising multiply after its call
(`block[0] = *dc_prediction * qt_table[0]`, `src/bitstream.rs:400`) does not wrap, so a frame whose DC differences accumulate a prediction past `i32` once multiplied
by the quantiser's DC entry panics wherever overflow checks are on (debug, test and fuzz builds)
while a release build wraps; making the multiply `wrapping_mul` gives every profile the release
build's behaviour and changes no sample any release build produces.

Found by fuzzing (a `cargo fuzz` target decoding untrusted codestreams to RGBA). Panic:

```text
thread 'main' panicked at zune-jpeg-0.5.15/src/bitstream.rs:400:20:
attempt to multiply with overflow
```

## Reproducer

An 846-byte baseline codestream, one component, 2122 x 509 (hexadecimal; `xxd -r -p` makes the
file). With the patch the decode returns `Ok` with 2122 x 509 x 4 bytes of RGBA.

```text
ffd8ffe000104a46494600010100000100010000ffdb004300100b0c0e0c0a100e0d0e1211101318281a181616183123
251d283a333d3c3933383740485c4e404457453738506d51575f626768673e4d71797064785c656763ffc0000b0801fd
084a01011100ffc4001a00010003010101000000000000000000000f00000000010405030207ffc4001f100100020202
0301010000000000000000001361111403120102044131ffda0008010100003f00f9f800000000000000000000000000
000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000
000000000000024c183060c183060c183060c183060c183060c183060c183060c183060c183060c183060c183060c183
060c183060c183060c183060c183060c183060c183060c183060c183060c183060c19d4ea753a9d4ea753a9d4ea753a9
d4ea753a9d4ea753a9d4ea3a759d4ea753a9d4ea753a9d4ea753a9d4ea753a9d4ea7533a9d4ea753a9d4ea753a9d4ea7
53a9d4ea753a9d4ea753a9d4ea753a9d4ea753a9d4ea753a9d4ea753a9d4ea753a9d4e7553d4a9a7ea3a9d4ea753a9d4
ea753a9d4ea753a9d4ea753a9d4ea753a9d4ea753a9d4ea753a9d4ea753a9d4ea753a9d4ea753a1ad4866b51ad46b51a
d46b51ad46b51ad46b51ad46b51ad46b51ad46b51ad46b51ad46b51affffffffffffffffffffffffffffffffffffffff
ffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffd46b51ad46b5
1ad46b51ad46b51ad46b51ad46b51ad46b51ad46b51ad46b51ad46b51ad46b51ad46b51ad46b51adffb51a2b6b51ad46
b51ad46b510a0a0a0ac18308f3e1cfdbc38727850e7f0cbfa7c7f593f4f8feb27e8f1fd67731a7bb9797904f875f4581
818181818181818181818181818181818181818181818181818181818181818181818181818181818181818181818181
818181818181818181818181818181818181818181818181818181818181818181818181818181818181818181818181
81998181818181818181819e2687cff8d5f97f1aff002fe35be75fe25af474f090000000000000000000000000000000
000000000000000000000a00000000000000000000000a0a0a0a0a0a0a00
```

Decoded with, in a debug build (`cargo run`):

```rust
use zune_jpeg::zune_core::{bytestream::ZCursor, colorspace::ColorSpace, options::DecoderOptions};

fn main() {
    let data = std::fs::read(std::env::args().nth(1).expect("a codestream")).expect("readable");
    let options = DecoderOptions::default().jpeg_set_out_colorspace(ColorSpace::RGBA);
    let mut decoder = zune_jpeg::JpegDecoder::new_with_options(ZCursor::new(&data[..]), options);
    println!("{:?}", decoder.decode().map(|pixels| pixels.len()));
}
```

## The patch

Against `31d81fed7551c8ccea456d9d8e2b1fd8bebb6995` (0.5.15 as published), in `crates/zune-jpeg`:

```diff
-        block[0] = *dc_prediction * qt_table[0];
+        block[0] = dc_prediction.wrapping_mul(qt_table[0]);
```
