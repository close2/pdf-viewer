# 80 — Rich media's `Sound` and `Video` configurations, and a movie played backward

Status: section 1's steps 1 to 3 **built** (ADRs 3854, 3855, 3865 to 3867), steps 4 and 5 planned (ADR 3820; review
2278 D03); section 2 **built** (ADRs 3818, 3837, 3841 to 3843, 3853; review 2278 D01), and the 13.4
row `implemented`. The rich media rows stay `out-of-scope` until section 1's last step is built.
Priority: the clause 13 lane, after its open fix rounds; questions never block it.
Inputs: RFC 0015 sections 5, 6 and 10.1; A946 and A1006; `doc/scope-ratifications.md`'s clause 13
entry. The `3D` configuration, `/Scripts` and ISO 21757-1's 3D API stay excluded and are refused by
name wherever a configuration or an instance names them.

## 1. Rich media, in RFC 0015 section 10.1's order (estimated five rounds)

1. **Settings and activation** — **built** (ADRs 3854, 3855). Tables 334 to 336: `/Activation` and `/Deactivation`'s
   `/Condition` (`XA`, `PO`, `PV`; `XD`, `PC`, `PI`), the configuration `/Configuration` names or
   the first, and the press path of ADR 3819 replaced, for a configuration the players take, by a
   play. Activated by a page open or visibility as the screen annotation's `/PO` and `/PV` are.
2. **Presentation** — **built** (ADRs 3865, 3866). Table 338: `Embedded` ("A interactive PDF processor shall support the state
   Embedded") in the annotation's rectangle as the video player already draws, `Windowed` in a
   floating window held "within the presentation area" (Table 339), `Transparent`,
   `PassContextClick`.
3. **Configurations and instances** — **built** (ADR 3867). Table 342's `Sound` and `Video`, Table 343's instance
   `/Subtype` matched to its asset: the asset types Parts 1 and 2 decode (WAVE, MP4 with H.264 and
   AAC) played; any other handed off by name. This match is a documented choice: §13.7 names no
   asset format for either (RFC 0015 section 10.1).
4. **Views and snapshots** — Table 344's `Snapshot`, and §13.7.2.3.6's state data read past by name
   (its protocol is the run time's, which the script exclusion holds).
5. **Animation** — Table 337's `PlayCount` and `Speed` on the players' clock.

## 2. A movie played backward, and a synchronous movie (built; ADRs 3818, 3837, 3841 to 3843, 3853)

1. **The worker** — **built** (ADR 3837): it decodes a group of pictures from its sync sample
   and writes it to the ring last picture first, and the clock runs a pass down. A group past 96
   MiB of decoded planes is let out in up to four parts, each decoded from the sync sample again,
   and the 256 MiB ceiling stands (ADR 3843).
2. **The sound and the modes** — **built**: the section's sound held whole and handed to the
   device turned, on the pictures' clock, bounded and refused by name (ADR 3841); `Palindrome`
   alternating pictures and sound pass by pass from one decode (ADR 3842). Table 212's sounds are
   not affected.
3. **Table 307's `/Synchronous`** — **built** (ADR 3853): "the movie player shall retain control
   until the movie is completed or dismissed by the user". The picture player holds the reader
   while a synchronous movie's clock runs, as the sound player holds it for Table 212's
   synchronous sound, and every window refuses every key and click but Escape, which dismisses it.

Each round leaves its clause's row re-read, and moves the row's status only once its last
`shall` is built.
