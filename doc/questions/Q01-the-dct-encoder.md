# Q01 — A DCT encoder dependency: which crate, or none?

Source: RFC 0002 §13 question 2.
Status: **open** — answered when `A01-the-dct-encoder.md` exists beside this file.

## Why it needs the owner

`optimize --images` and JPEG output from `render` both need to *write* a JPEG, and nothing in the tree encodes one. `doc/stack.md`'s rules decide which crate is admissible; the choice is the owner's because it is a dependency in a shipped path, not a test one.

## What the tree does meanwhile

Both features are refused by name. `optimize` states no `--images` flag at all rather than one that silently does nothing: without an encoder, recompressing as DCT cannot be done, and downsampling to a lossless encoding makes photographs larger, so the verb's own keep-what-fails-to-shrink rule would leave every image untouched. Session 900 wrote that argument down.

## Recommendation

Take a reviewed pure-Rust encoder if `doc/stack.md` admits one, on the precedent where the owner chose reviewed crypto dependencies over in-tree arithmetic. One answer unblocks two features.
