# 3843 — A group of pictures too long to hold is let out in parts, and the ceiling stays

Session 2285. Status: **accepted**. Code: `pdf-media-worker`'s `video.rs` (`Group::keeping`,
`Group::wants_more`), `wire.rs` (`VideoRun::Backward`'s `from` and `to`), `worker.rs` (`backward`,
`hold_group`) and `client/video.rs` (`parts`, `MAX_GROUP_PARTS`, `backward_part`,
`MediaWorker::with_group_bytes`). Clauses: ISO 32000-2 §13.4, Table 307. Price: ADR 3837.

## Context

ADR 3837 bounds a backward group at `MAX_GROUP_BYTES`, 96 MiB of decoded planes — thirty-two
1080-line pictures — and refused a longer group: a two-second group at 30 pictures a second is
sixty. It priced the alternative as raising `Profile::Media`'s 256 MiB address-space ceiling to
about 512 MiB. The brief preferred keeping the ceiling.

## Decision

1. **A group is let out in parts, the last first.** The host splits a group into as few equal
   ranges of output places as keep each range's planes within the bound, and sends the group once
   for each, from its sync sample; the worker decodes every picture from there — a later picture
   may refer to any of them — holds only those in the range, and stops decoding once the range's
   last place is out. Each part is let out last picture first, so the pass is exactly an unsplit
   one: the same pictures at the same times.
2. **At most `MAX_GROUP_PARTS`, four.** `k` parts decode about `(k + 1) ÷ 2` times the group, so
   four admit a 1080-line group of 128 pictures — over four seconds at 30 a second — at two and a
   half times its forward decode. A longer group is refused by name before any of it is decoded,
   as before.
3. **The ceiling is not raised.** `pdf-sandbox`'s media profile is untouched; the worker's system
   calls, ring and descriptor are unchanged; the wire's backward run carries its range as eight
   bytes after its flag.
4. **A test may lower the bound, never raise it** (`MediaWorker::with_group_bytes`), so the eight
   32 by 32 pictures of the fixture split exactly as a 1080-line group does.

## Measured

`cargo nextest run -p pdf-media-worker --test video --run-ignored only --no-capture address_space`
(debug worker, intra pictures and skipped copies, three runs each, this VM):

| group, 1920 by 1088 | parts | first backward picture | last picture | worker `VmPeak` |
|---|---|---|---|---|
| 32 pictures | 1 | 244–482 ms | 319–598 ms | 178 424 kB |
| 60 pictures | 2 | 229–378 ms | 505–934 ms | 218 708 kB |
| 128 pictures | 4 | 338–353 ms | 1.45–1.55 s | 224 320 kB |

- **Every part stays under the 256 MiB ceiling** (262 144 kB): the sixty-picture group peaks 42
  MiB short of it and the 128-picture group 37 MiB short. The peak rises past one part's 174 MiB
  because the allocator keeps the first part's freed planes in its heap for the next; it does not
  grow with the number of parts.
- **The first backward picture is no later** than an unsplit group's: it waits for one decode of
  the group, the last part's.
- **The decode keeps up here and may not elsewhere**: 128 pictures, 4.3 seconds at 30 a second,
  are all out in about 1.5 seconds for these cheap pictures; a real stream decodes each slower, and
  a backward pass that falls behind drops pictures late and says so, as a forward one does.

## Rejected

Raising the media worker's ceiling to 512 MiB: it costs every media worker, forward or backward,
twice the address space it may take, for a case the parts serve within the present one.
