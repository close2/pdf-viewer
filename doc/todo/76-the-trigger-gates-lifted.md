# 76 — The trigger gates, audited and lifted

Status: **open** — the audit is done and its first effort-only gate is built (ADR 2799); the rest of
section 2 is the order. ADR 2800 is the decision this file carries out.
Priority: the owner's, 2026-11-06, `doc/questions/A800` item 2: "build now and revisit all 'wait for
a trigger' gates".
How the population was found: `grep -rniE 'untriggered|trigger|wait(s|ing)? for a|until a |deferred
until|revisit(ed)? when|reopen'` over `doc/questions/A*.md`, `doc/adr/`, `doc/todo/`,
`doc/conformance/ledger.toml`, `doc/scope-ratifications.md`, `doc/stack.md`, `CLAUDE.md` and
`doc/rfc/`, each hit read in context. §12.6.3's trigger *events*, a gate or report that "fires", and
a test's timing are not work held back and are not here.

## 1. The kinds, and what A800 does to each

- **Effort only** — the gate waited for a document, a host or a request that no longer has to come.
  A800 lifts it: the work is owed now, in the order of section 2.
- **Another lane's** — effort only, and owned by the signatures lane or the PDF/A lane.
- **Owner-blocked** — the gate is a purchase, a person's act, or a decision the owner reserved, and
  "build now" does not supply it. Each stays, named, and `doc/questions/Q828` asks about them.
- **Outside release** — the gate is a crate that does not exist yet; nothing in this tree can build it.
- **Measured and declined** — principle 2: "an optimization must be justified by a benchmark". The
  benchmark was taken and did not justify the build; building it now would be the speculative
  optimisation `CLAUDE.md` forbids, so these are not triggers in A800's sense and stand.
- **Scope exclusion** — one of `CLAUDE.md`'s closed list, ratified by the owner with its reopening
  trigger in `doc/scope-ratifications.md`; "revisit an exclusion by argument, never by attrition",
  so these go to the owner (Q828) rather than being built.
- **Already built** — the gate's text outlived the work.

## 2. The gates

| # | where | holds back | row or feature | kind |
|---|---|---|---|---|
| 1 | `doc/questions/A45`; `viewer-core/src/notes.rs` | the attachments panel's row for a file outside the document | §14.13.2, §14.13.4 | **built** (ADR 2799) |
| 2 | `doc/questions/A66`, `A168`; ADRs 1485, 1202; `doc/todo/01`; `doc/todo/65` section 5 | §7.6.5's public-key handler | §7.6.5, §7.6.5.1–3, §7.6.6 | another lane's — A800 item 1 lifted it by name |
| 3 | ADR 2529 (status deferred to Q713) | §12.8.3.4.4's policy enforcement | §12.8.3.4.4 | another lane's — `doc/questions/A713` placed the documents |
| 4 | ADR 1736 section 1 | `app.goBack`/`goForward`, held in `surface::REFUSED` until a window keeps a history of views | JavaScript `app` members | effort only |
| 5 | `doc/todo/63` "what is left" | one `Query` for a page's text on the confined pipe | `viewer-core` vocabulary | effort only |
| 6 | `doc/todo/65` section 1's host rows | per-host reads of print and collection entries | rows needing a host | effort only |
| 7 | ADR 2137 | the records the WinUI window declines in the library's terms | WinUI host | effort only |
| 8 | ADR 0705 | the dot-pattern splits | §10.7.4 | effort only |
| 9 | RFC 0009 (CMYK spool through BToA) | a print spool that converts through the intent's `BToA` | §14.11.5 on paper | effort only, after the spool path it names |
| 10 | `CLAUDE.md`; `doc/scope-ratifications.md`; A639, A633 | XFA's dynamic half: the measurement round that prices page one of the `/NeedsRendering` documents | Annex K | effort only for the measurement; the build stays a scope exclusion |
| 11 | `doc/questions/A270`, `A595`; ADRs 1585, 2299 | the launcher's two Vulkan variables | GPU selection | owner-blocked: "until a launcher exists" is the owner's packaging decision |
| 12 | `doc/questions/A636`; ADR 2374 | signing the Windows zip's executables | WinUI release | owner-blocked: a certificate, and a person running the window on Windows |
| 13 | ADR 2489 | MPEG-1 and MP3 clip timing | clause 13 media | owner-blocked: ISO/IEC 11172 and 13818-3 are not held |
| 14 | ledger §7.4.9; ADRs 2793, 2292; A348, A169 | `CIEJab` beyond its fallback; T.801's 2004 edition | §7.4.9 | owner-blocked: CIE 131 not bought; the edition's purchase |
| 15 | `doc/questions/A97` | ISO 19444-1 | §12.7.8.3.4 | owner-blocked, and conditional: only if the edition difference bites |
| 16 | `CLAUDE.md` principle 3; `doc/stack.md`; A130; ADR 1327 | `ring` replaced by a pure-Rust provider | TLS | outside release |
| 17 | `doc/todo/65` section 3; `doc/stack.md`; A170, A192 | the curves swapped for a reviewed crate | ISO/TS 32002 | outside release (the curves are built) |
| 18 | ADR 2038 | three ignored advisories | `accesskit_unix` | outside release |
| 19 | `doc/todo/49` (`--cache-text`) | a readback budget flag | the CLI | measured and declined (ADR 0256) |
| 20 | ADRs 2283, 2349, 2351, 2366, 2297, 2365, 0767, 2189, 2049; `doc/todo/47`, `75`, `30` | bring-up, plane, half-float, shading, knockout, decoder, encode, replay and read-back levers | performance | measured and declined |
| 21 | ADR 2341 | a ramp's two pages, until the oracle draws its own ramp | the oracle's shadings | effort only |
| 22 | `CLAUDE.md`; `doc/scope-ratifications.md`; A632, A649; ADRs 2428, 2400 | Tier C, decoding audio and video; Tier D, U3D and PRC | clause 13 | scope exclusion |
| 23 | A254 question 7; RFC 0009 | overlay and underlay of a producer's page | authoring | scope exclusion |
| 24 | ADR 1672 section 7 | a projected `/DCS` written "until a host writes eastings" | §12.10.2 | already built: `viewer-host/src/measuring.rs` writes the grid |
| 25 | ADRs 1608, 1614 | the script worker, "until a host supplies a level" | JavaScript | already built (ADRs 1616, 1625) |
| 26 | ADR 2784 | a drop-down's `preOpen` and `postOpen` | JavaScript events | already built (ADR 2791) |
| 27 | ADR 0207 | fragment identifiers "for a host with a URI" | Annex O | already built |
| 28 | ADRs 2378, 2445, 2519, 2630, 2677, 2682; `doc/todo/72` | the archive conversion's font and press gates | `archive` | the PDF/A lane's |

## 3. The order

Row 1 first, because it is the one whose shape the owner had already given (A45's second paragraph)
and whose only gate was "when the attachments panel is designed" — and the panel is
`Query::Attachments` in four windows. Then rows 4 and 5, each one `Query` or one host history and
a test; row 21's ramp in the oracle; row 6's host reads by `doc/todo/30`–`38`; row 8 by `doc/todo/_scan-conversion.md`; row 10's
measurement as its own measure round, whose result goes to the owner with the near-side
construction RFC 0011 section 7 asks for. Rows 2, 3 and 28 are the other lanes' and are listed so
that this file is the whole population.

## 4. What the lift changed in the text

A record keeps its words (ADR 1023); a living document says what is. `doc/todo/01`'s §7.6.5 bullet,
`doc/todo/63`'s "take it when", `doc/questions/A45`'s `Owes:` and the Readings of `A66` and `A168`
now say what A800 did; ADRs 1485, 1672, 1736, 2137 and 2529 are records and are cited from here and
from ADR 2800 instead.
