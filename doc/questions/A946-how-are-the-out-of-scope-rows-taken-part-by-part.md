Status: complete
Given: 2026-11-20, in conversation — transcribed by the round
Owes: (1) build the owed Media level (A254) and starting the host's own external player first (2 rounds); (2) open Tier C for audio decoding and presentation alone in RFC 0015 section 5's order (confined media worker, raw sound samples and scalar rusty_aac, window audio sinks, 9 rounds), leaving video closed until audio ships and its clock and sinks are measured in use; (3) restate U3D's trigger in CLAUDE.md and doc/scope-ratifications.md to ECMA-363's third edition and keep it closed; (4) add PRC's trigger requiring ISO 14739-1 to be held plus a fuzzed Rust PRC reader or the owner's decision to write one in-tree; (5) let rich media and Go-To-3D-View follow the parts they depend on, leave Rich-Media-Execute and §13.7 scripting with the ISO 21757-1 script exclusion, and record §13.5 as planned by no part (media / scope rounds)

> Q946: agreed

Reading: adopts all six recommendations of RFC 0015 / Q946:
1. Build the owed `Media` permission level and launching the machine's own external player first (2 rounds).
2. Lift Tier C for **audio** alone in RFC 0015 §5's order (confined media worker with raw sound samples and scalar `rusty_aac`, plus window audio sinks, 9 rounds).
3. Keep **video** closed until audio has shipped and its clock and sinks are measured in use, then ask before opening §6's 14 rounds.
4. Restate **U3D**'s trigger to ECMA-363's third edition (matching ISO 32000-2) and keep it closed.
5. Add **PRC**'s trigger requiring `ISO 14739-1` to be held plus a fuzzed Rust PRC reader or an owner decision to write one in-tree.
6. Let **rich media** and **Go-To-3D-View** follow their underlying parts with no trigger of their own, keep **Rich-Media-Execute** and §13.7 scripting under the ISO 21757-1 script exclusion, and record **§13.5** (alternate presentations) as planned by no part.
