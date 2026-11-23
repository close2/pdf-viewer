# 3501 — A stream is one request answered by items the host grants, stops and budgets

Session 2200. Status: accepted. Context: RFC 0015 sections 4.3 and 6 (the streaming arm, video's
first round); RFC 0012 section 5.1 (the wire one reply per request); `doc/questions/A1006`; ADRs
0846, 3297, 3381, 3457.

## Decision

1. **`confined_transport::stream` is the streaming arm**, and `Host::stream` opens one. Five frame
   kinds, the transport's own and past both protocols' numbering beside `RESOURCE_REQUEST`'s:
   `OPEN` (the asking protocol's kind, a first grant, its request), `CREDIT` (a grant above zero),
   `STOP`, `ITEM` (a sequence number, a carriage byte, the body) and `END` (finished or stopped,
   and a closing of at most 4 096 bytes). Every frame is framed and length-checked as a reply is.
2. **Flow control is credit granted only when the last grant is spent.** The worker sends no item
   it holds no credit for, and with none it reads the host's next frame before writing anything,
   its end included; the host grants again only once every granted item has arrived. So one control
   frame at most is in flight, every one is read, and a worker with blocking reads and one thread
   leaves nothing stale in its input — the next request on the wire reads clean.
3. **A stop is cooperative first and a kill second.** `Stream::stop` reads what was granted, writes
   `STOP` where the grant would have gone, and reads the worker's end; the worker is kept. Any
   broken contract — an item past the credit, out of sequence, of a carriage this build does not
   read, a frame of another kind, an end giving a stop nobody sent — ends the worker through its
   `Canceller`, as does a stream dropped before its end.
4. **Three budgets per stream, each the host's** (`stream::Budget`): frames bound every grant;
   bytes are checked against an item's header before its payload is allocated, and an item past
   them is read past and the stream stopped; wall clock is held by a watch thread that ends the
   worker when it passes, because the read it would interrupt is the one a stalled worker never
   answers. Each is said by name (`StreamError::Spent`); frames and bytes keep the worker.
5. **The carriage byte is the seam round 2 uses.** Today one carriage, `Inline`; the ring's items
   will name a slot, and its grant will be the slots the host has freed. The control frames and
   their rules stay as they are.

## Consequences

- The host cannot tell, item by item, a worker that ignores its credit from one that read its grant
  promptly; it sees an over-send where it granted nothing (after a stop, past the frames budget). The
  pipe's buffer and the frames budget bound what such a worker costs, so nothing more is asked.
- A watch thread a stream: tens of microseconds against a stream's seconds, and the price of a
  wall-clock bound a blocked read cannot otherwise have.
- Pacing at a grant's boundary waits one round trip; a host that wants the worker never idle reads
  items into its own queue and grants as it frees room, which is the ring's shape.
