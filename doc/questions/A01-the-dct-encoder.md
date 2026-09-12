Status: complete
Given: 2026-09-06, by the owner's hand
Owes: the `mozjpeg-rs` DCT encoder in `doc/stack.md`, which gates lossy image optimisation — absent from the tree at retrofit

Use mozjpeg-rs 

However be very careful to only reencode when explicitly requested.
Unless a specific format is requested, we should always extract the binary data into the corresponding image format (*without reencoding*)

Use any pure image encoding libraries for other formats (if the license allows it).

