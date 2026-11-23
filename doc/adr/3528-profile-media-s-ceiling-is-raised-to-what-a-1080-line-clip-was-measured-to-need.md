# 3528 — `Profile::Media`'s ceiling is raised to what a 1080-line clip was measured to need

Session 2206. Status: accepted. Context: RFC 0015 section 4.1 (video: "1 GiB"), ADRs 3297, 3381,
3514, 3526, 3527; `pdf_sandbox::lockdown`'s `MEDIA_ADDRESS_SPACE_LIMIT`.

## Context

At 64 MiB the worker died decoding a 1080-line clip that holds pictures for order: "memory
allocation of 522240 bytes failed". The figure is measured in the worker, read by the host from
`/proc/<pid>/status` (`MediaWorker::process_id`), with the ceiling lifted to a gibibyte for the
reading only: a 1920 × 1088 IDR and sixty all-skipped P pictures, the sequence keeping sixteen
reference frames, sixteen more held for output order, the ring of four 1080-line slots mapped.

| clip | `VmPeak` | `VmHWM` |
|---|---|---|
| one reference frame | 84 572 kB (82.6 MiB) | 81 804 kB |
| sixteen reference frames | 132 704 kB (129.6 MiB) | 129 860 kB |

## Decision

`MEDIA_ADDRESS_SPACE_LIMIT` is 256 MiB: twice the measured peak, for the allocator's slack, what
the clip does not exercise (B pictures, many slices) and a grown stack. RFC 0015's gibibyte is not
taken. `MAX_PICTURE_PIXELS` (1920 × 1088) is the picture the figure holds; a larger declared track
is refused by name, and a stream larger than its track declares meets the ceiling and dies, said.

## Consequences

The sound worker shares the profile and so the ceiling: its measured 36 MiB worst holding is
unchanged, and its ceiling is no longer three times its need. The ring may be half the ceiling, 128 MiB,
and is 12.5 MB. `tests/video.rs`'s ignored measurement re-reads the figure.
