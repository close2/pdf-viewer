# 80 — Rich media's `Sound` and `Video` configurations, and a movie played backward

Status: **planned**, not built (ADRs 3818, 3820; review 2278 D01 and D03). The rich media rows stay
`out-of-scope` and the 13.4 row `partial` until the rounds below are built.
Priority: the clause 13 lane, after its open fix rounds; questions never block it.
Inputs: RFC 0015 sections 5, 6 and 10.1; A946 and A1006; `doc/scope-ratifications.md`'s clause 13
entry. The `3D` configuration, `/Scripts` and ISO 21757-1's 3D API stay excluded and are refused by
name wherever a configuration or an instance names them.

## 1. Rich media, in RFC 0015 section 10.1's order (estimated five rounds)

1. **Settings and activation** — Tables 334 to 336: `/Activation` and `/Deactivation`'s
   `/Condition` (`XA`, `PO`, `PV`; `XD`, `PC`, `PI`), the configuration `/Configuration` names or
   the first, and the press path of ADR 3819 replaced, for a configuration the players take, by a
   play. Activated by a page open or visibility as the screen annotation's `/PO` and `/PV` are.
2. **Presentation** — Table 338: `Embedded` ("A interactive PDF processor shall support the state
   Embedded") in the annotation's rectangle as the video player already draws, `Windowed` in a
   floating window held "within the presentation area" (Table 339), `Transparent`,
   `PassContextClick`.
3. **Configurations and instances** — Table 342's `Sound` and `Video`, Table 343's instance
   `/Subtype` matched to its asset: the asset types Parts 1 and 2 decode (WAVE, MP4 with H.264 and
   AAC) played; any other handed off by name. This match is a documented choice: §13.7 names no
   asset format for either (RFC 0015 section 10.1).
4. **Views and snapshots** — Table 344's `Snapshot`, and §13.7.2.3.6's state data read past by name
   (its protocol is the run time's, which the script exclusion holds).
5. **Animation** — Table 337's `PlayCount` and `Speed` on the players' clock.

## 2. A movie played backward (estimated two rounds, ADR 3818)

1. **The worker** decodes a group of pictures from its sync sample and writes it to the ring last
   picture first; the clock runs a pass down. Bound: one group of decoded planes, refused by name
   past it.
2. **The sound and the modes** — the sound's blocks reversed in the same pass; Table 307's negative
   `/Rate` and `Palindrome`'s alternation; Table 212's sounds are not affected.

Each round leaves its clause's row re-read, and moves the row's status only once its last
`shall` is built.
