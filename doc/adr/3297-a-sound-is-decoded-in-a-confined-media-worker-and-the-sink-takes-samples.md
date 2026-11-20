# 3297 — A sound is decoded in a confined media worker, and the window's sink is handed samples

Session 2149. Status: accepted. Context: `doc/questions/A946` (RFC 0015 section 12, all six
recommendations adopted), RFC 0015 sections 4 and 5, RFC 0012 section 5.1, RFC 0008 section 6.2,
ADRs 0014, 1608, 1609, 1945, 2382, 2428, 3133, 3201, `doc/scope-ratifications.md`.

## Decision

The owner opened **audio alone** (A946, item 2): decoding and presenting a sound is in scope, video,
U3D and PRC stay excluded behind their triggers. RFC 0015 section 5's first step is built:

1. **`pdf-media-worker`**, a crate and a program of its own, on `confined-transport`'s wire. Its host
   half, `MediaWorker`, starts nothing when it is made; the worker is started at the first sound
   decoded and replaced after a death at the next (RFC 0015 section 4.6: nothing on the launch path).
   Its worker half, `serve`, confines its process before it reads a byte and answers requests.
2. **`pdf_sandbox::lockdown::Profile::Media`**: `Profile::Script`'s system-call list unchanged — one
   thread, no descriptor received or made (`RLIMIT_NOFILE` zero), `mmap` only without `PROT_EXEC`,
   no `mprotect` — with an address-space ceiling of 64 MiB. A sample-format decoder reads a frame,
   allocates on one thread and writes a frame, which is the list's whole shape, so nothing was added;
   the end-to-end tests run every encoding through the confined program and none is killed. Probes in
   `crates/pdf-sandbox/tests/confinement.rs` show the file, the socket, `fcntl`, a thread and
   executable memory refused under it.
3. **The decoder** (`pdf_media_worker::sound`) reads §13.3's own formats — Table 305's `Raw` and
   `Signed` at every width from 1 to 32 bits, packed most significant bits first, and µ-law and
   A-law codes by ITU-T G.711 (1988) Tables 1 and 2 — into **one** format, an `i32` with the sample in
   its most significant bits. That is §13.3's "Sound players shall convert between formats"; the
   clause's other two verbs, downsampling and combining channels, belong to the sink beside the
   device they are for, so rate and channels cross as the object states them.
4. **The wire** is RFC 0015 section 4.3's shape for audio: one request per run of whole frames, at
   most 256 KiB, cut on `lcm(frame bits, 8)` so no sample is split; one reply of samples. The host
   reads a reply as a compromised worker's: the sample count a request holds is the host's own
   arithmetic and any other count is refused.
5. **The budgets**: per request, one second of the worker's processor time read from
   `/proc/<pid>/stat` by the host, and ten times that of wall clock as the machine's ceiling (ADR
   3133's arrangement; `pdf_sandbox::Allowance` and `confined_transport::Canceller::process_id` were
   made public for it). A caller's bound on the samples a sound decodes to is checked before the
   worker is started.
6. **A fuzz target from the first parser commit** (principle 3): `fuzz/fuzz_targets/media_sound.rs`
   over the worker's entry — the request decoded and `sound::decode` run on it — and over the reply
   as the host reads it, seeded by `fuzz/seed_media_sound.py`.

## The measurements

| what | measured (debug build) | bound set |
|---|---|---|
| worker address space at its greeting | 4.1 MiB | — |
| peak over the worst request (256 KiB of one-bit samples, 8 MiB out) | 20.3 MiB | 64 MiB ceiling |
| processor time of that request | 30–40 ms | 1 s budget, 10 s wall ceiling |

Read with `/proc/<pid>/status` and `/proc/<pid>/stat` from a parent driving the worker over a pipe,
three requests in a row.

## The C on the output side, for step 2

Principle 3: "Any C dependency … must be isolated in the sandboxed process and justified in
writing." The window's sink — `cpal` for `quorra` and GTK, which links `libasound` through
`alsa-sys`; Qt's `QAudioSink`; WinUI's audio graph — **is C in the host, on the output side**: it
receives `i32` samples this program decoded inside `Profile::Media`, and parses nothing the
document wrote. That is the standing the toolkits' own C already has, and it is the justification
written here so that step 2 does not have to argue it again. What the justification does not cover,
and stays refused: GTK's `MediaFile`, `GStreamer`, or any system framework handed the document's bytes
(RFC 0009 section 3.6), and AAC, which comes through `rusty_aac` in this worker (ADR 2428, RFC 0015
section 5 step 9), never through a platform decoder.

## Why not a feature-gated decoder

`pdf-script-worker` builds its engine only under a feature, so that the host links none of it. The
media decoder is a hundred lines of safe arithmetic with no dependency, and the host half never
calls it; what principle 3 asks is that the document's bytes reach it only in the confined process,
which the wire guarantees. A feature would make `cargo nextest -p pdf-media-worker` build no worker,
so the gate would run no end-to-end test — a cost with no safety bought.

## Cost

A crate of about 1,350 lines with its tests, a profile that adds a ceiling and no system call, two
public items in shared crates, and no change to the launch path, the host's binaries or any window:
no window plays yet. Step 2 is the sink in `quorra` behind ADR 3201's `Media` level's *play here*
answer.
