#!/usr/bin/env python3
"""Seeds `confined_stream`: runs of stream frames in the target's own layout (ADRs 3501, 3513).

The layout is the target's: a window byte (its top bit a ring of 64-byte slots, as many as its low
nibble and one), a two-byte frames budget, a four-byte bytes budget,
then frames of a kind byte, a two-byte length and the payload. Each seed is a run that keeps the
stream contract, so that the mutator starts from the paths a real worker takes — items under a
grant, an end after the last, a stop answered — rather than from a first frame that is refused.
"""

import struct
import sys
from pathlib import Path

ITEM, END = 0xF6, 0xF7


def frame(kind: int, payload: bytes) -> bytes:
    return struct.pack(">BH", kind, len(payload)) + payload


def item(sequence: int, body: bytes) -> bytes:
    return frame(ITEM, struct.pack(">IB", sequence, 0) + body)


def slot_item(sequence: int, slot: int, length: int) -> bytes:
    return frame(ITEM, struct.pack(">IBII", sequence, 1, slot, length))


def end(reason: int, closing: bytes = b"") -> bytes:
    return frame(END, bytes([reason]) + closing)


def head(window: int, frames: int, nbytes: int) -> bytes:
    return struct.pack(">BHI", window, frames, nbytes)


def main() -> None:
    out = Path(sys.argv[1])
    out.mkdir(parents=True, exist_ok=True)
    seeds = {
        "finished": head(2, 10, 1000)
        + b"".join(item(n, b"frame") for n in range(5))
        + end(0, b"done"),
        "frames-spent-then-stopped": head(3, 6, 1000)
        + b"".join(item(n, b"x") for n in range(6))
        + end(1),
        "bytes-spent": head(4, 10, 8) + item(0, b"abcdef") + item(1, b"ghijkl") + end(1),
        "an-item-read-past": head(2, 10, 4) + struct.pack(">BH", ITEM, 0xFFFF),
        "empty-items": head(1, 3, 0) + b"".join(item(n, b"") for n in range(3)) + end(0),
        "through-a-ring": head(0x82, 10, 1000)
        + b"".join(slot_item(n, n % 3, 64) for n in range(5))
        + item(5, b"small")
        + end(0, b"done"),
    }
    for name, data in seeds.items():
        (out / name).write_bytes(data)


if __name__ == "__main__":
    main()
