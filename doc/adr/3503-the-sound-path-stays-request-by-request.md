# 3503 — The sound path stays request by request

Session 2200. Status: accepted. Context: ADRs 3381, 3457, 3501; the round's brief (move ADR 3457's
`FRAME_UNITS` loop onto the stream only if that is simpler and measured no worse).

## Decision

`MediaWorker::decode_track` keeps one `FRAME_UNITS` request per run. Not moved, and therefore not
measured with `tools/audio-in-use.sh`: the condition was "simpler", and it is not.

## Why

A stream is one request answered by many replies. The sound path is many requests: the worker
never holds the file — the host does, and each run's access units cross *in* the request (ADR
3381's runs, bounded by `MAX_REQUEST_BYTES`). On the stream, either the whole track's units cross
in the open request — a clip's whole media data in one message, where ADR 3381 bounds a request
by `MAX_REQUEST_BYTES` — or the
worker asks for units through the resource port inside the stream, which is two protocols where one
serves. The run-by-run loop already paces the worker by the host's demand, which is all the arm's
flow control would add. Video differs because its decoder will need the stream to run ahead of the
host by a grant, and its frames are thirty a second.

## Consequences

The arm's first user is video's decoder (RFC 0015 section 6, round 3); until then
`tools/stream-witness` is its only caller, and the arm is held by its tests and fuzz target.
