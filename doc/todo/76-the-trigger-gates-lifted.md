# 76 — The trigger gates, audited and lifted

Status: **open** — the audit is done; rows 1, 4, 5, 6, 7, 8 and 21 are built (ADRs 2799, 2805, 2836, 2919, 2185,
2884, 2356 and 2404), row 10's measurement is taken and its build scheduled (ADRs 2837 and 2876, `doc/todo/78`),
and the effort-only rows left outside the other lanes are row 6's and row 7's WinUI print dialogue (`doc/todo/30`'s
N4) and row 9, whose spool path is unbuilt (ADR 2930 is the sweep that found each). ADR 2800 is the decision this
file carries out.
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
  "build now" does not supply it. Each stays, named: `doc/questions/A828` (2026-11-11) bought no
  standard and kept the launcher and signing gates, so rows 11 to 15 stand.
- **Outside release** — the gate is a crate that does not exist yet; nothing in this tree can build it.
- **Measured and declined** — principle 2: "an optimization must be justified by a benchmark". The
  benchmark was taken and did not justify the build; building it now would be the speculative
  optimisation `CLAUDE.md` forbids, so these are not triggers in A800's sense and stand.
- **Scope exclusion** — one of `CLAUDE.md`'s closed list, ratified by the owner with its reopening
  trigger in `doc/scope-ratifications.md`; "revisit an exclusion by argument, never by attrition",
  so these went to the owner, and A828 kept rows 22 and 23 as written.
- **Already built** — the gate's text outlived the work.

## 2. The gates

| # | where | holds back | row or feature | kind |
|---|---|---|---|---|
| 1 | `doc/questions/A45`; `viewer-core/src/notes.rs` | the attachments panel's row for a file outside the document | §14.13.2, §14.13.4 | **built** (ADR 2799) |
| 2 | `doc/questions/A66`, `A168`; ADRs 1485, 1202; `doc/todo/01`; `doc/todo/65` section 5 | §7.6.5's public-key handler | §7.6.5, §7.6.5.1–3, §7.6.6 | **built in part** (ADRs 2801, 2802, 2809, 2810, 2821, 2822, 2826, 2827): the handler, all seven envelope ciphers, Table 27's per-stream list, RSAES-PKCS1-v1_5 and RSAES-OAEP key transport, RFC 5652 key agreement for an elliptic-curve certificate under all four key wraps, the host input (ADR 1134) and RFC 5753 section 3.2's 1-Pass ECMQV (ADR 2904) run; brainpoolP512r1 alone is left, refused by its curve's name because no reviewed package does Diffie–Hellman or MQV on it (ADRs 2827, 2904) |
| 3 | ADR 2529 (status deferred to Q713) | §12.8.3.4.4's policy enforcement | §12.8.3.4.4 | the signatures lane's, **built in part**: `doc/questions/A713` placed the documents, and TS 119 172-3's ASN.1 and TS 119 172-2's XML policies are enforced (ADRs 2560, 2604), with the mandated path (ADR 2781), the trusted-list anchors (ADR 2874) and a signer's SAML assertion (ADRs 2922, 2923); the row's note names what holds it `partial` (ADRs 2725, 2782, 2895) |
| 4 | ADR 1736 section 1 | `app.goBack`/`goForward`, held in `surface::REFUSED` until a window keeps a history of views | JavaScript `app` members | **built** (ADR 2805) |
| 5 | `doc/todo/63` "what is left" | one `Query` for a page's text on the confined pipe | `viewer-core` vocabulary | **built** (ADRs 2836, 2844): `Query::Text`, the pages on the screen, a reading at every restriction level, and the C ABI's `quorra_page_text` |
| 6 | `doc/todo/65` section 1's host rows | per-host reads of print and collection entries | rows needing a host | **built** (ADR 2919): section 1 names no row of its own and §12.3.5's collection is presented in all four windows; the one print entry no dialogue presented, Table 147's `/PickTrayByPDFSize`, is a box on `quorra-gtk`'s and `quorra-qt`'s dialogues that, checked, puts each page on paper of its own size (`Query::PrintSize`, `quorra_print_size`). The WinUI window's half waits for its print dialogue, `doc/todo/30`'s N4, which ADR 2919 says what it owes |
| 7 | ADR 2137, superseded by ADR 2185 | the records the WinUI window declines in the library's terms | WinUI host | **built** (ADR 2185): all five cards are answered on a `ContentDialog`; the print card's printer selection and spooling are `doc/todo/30`'s N4, row 6's WinUI half |
| 8 | ADR 0705 | the dot-pattern splits | §10.7.4 | **built** (ADR 2884): a scene keeps the marks §8.5.3.2's splits stated and another view asks their two answers again; the shader mirror of `point_mark` is declined there, because the oracle's answer for a dot inside a path depends on its neighbours |
| 9 | RFC 0009 (CMYK spool through BToA) | a print spool that converts through the intent's `BToA` | §14.11.5 on paper | effort only, and its spool path is not built: `quorra-gtk` and `quorra-qt` print through their toolkits' RGB contexts, and RFC 0009 sections 2.7 and 2.8's `ipp` submission and spool container, owed by `doc/questions/A254` item 2 and placed in `doc/todo/65` section 1's "RFC 0004's print path" (ADR 2564), do not exist (no `ipp` in `Cargo.lock`); A254 adopted section 11's answer 9, "no, until a spool path exists", and A800 lifted only its document half. `pdf-colour` reads a `lutBToAType` tag and writes a `B2A` table (ADR 2700) but evaluates no `BToA` |
| 10 | `CLAUDE.md`; `doc/scope-ratifications.md`; A639, A633 | XFA's dynamic half: the measurement round that prices page one of the `/NeedsRendering` documents | Annex K | measured (ADR 2837, `doc/questions/Q840`), and the exclusion lifted by the owner's answer A840 as `CLAUDE.md`'s fourth amendment (ADR 2876); the build is scheduled after the ISO 32000-2 ledger rows reach their final states, `doc/todo/78`'s order |
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
| 21 | ADR 2341 | a ramp's two pages, until the oracle draws its own ramp | the oracle's shadings | **built** (ADRs 2356, 2404): the oracle draws a fill's and a stroke's ramp from its own cells at each pixel's `t`, and ADR 2341's two pages fell; `alphatrans.pdf`'s tie is a stored alpha's, not the ramp's (ADR 2367) |
| 22 | `CLAUDE.md`; `doc/scope-ratifications.md`; A632, A649; ADRs 2428, 2400 | Tier C, decoding audio and video; Tier D, U3D and PRC | clause 13 | scope exclusion |
| 23 | A254 question 7; RFC 0009 | overlay and underlay of a producer's page | authoring | scope exclusion |
| 24 | ADR 1672 section 7 | a projected `/DCS` written "until a host writes eastings" | §12.10.2 | already built: `viewer-host/src/measuring.rs` writes the grid |
| 25 | ADRs 1608, 1614 | the script worker, "until a host supplies a level" | JavaScript | already built (ADRs 1616, 1625) |
| 26 | ADR 2784 | a drop-down's `preOpen` and `postOpen` | JavaScript events | already built (ADR 2791) |
| 27 | ADR 0207 | fragment identifiers "for a host with a URI" | Annex O | already built |
| 28 | ADRs 2378, 2445, 2519, 2630, 2677, 2682; `doc/todo/72` | the archive conversion's font and press gates | `archive` | the PDF/A lane's, **built in part**: ADRs 2378, 2677 and 2682 are built, ADR 2445's deferral by ADR 2454 and ADR 2519's by ADR 2700's assumed-inks profile, and ADR 2642 is the restatement ADR 2630 put first; ADR 2630's Noto `latin-greek-cyrillic` pack is not built |
| 29 | `doc/questions/A800` item 3; ADRs 2725, 2782 | European signatures' eIDAS levels against EU trusted lists, and a *locally valid* standing for a reader's own anchors | §12.8.1, §12.8.3.4.4, §12.8.3.4.5 | the signatures lane's, **built in part** (ADRs 2834, 2835, 2842, 2843, 2852, 2853, 2864, 2865, 2866, 2872, 2874, 2878, 2879, 2882, 2883, 2886, 2896, 2897): locally valid, a TS 119 612 reader, TS 119 615 clauses 4.1 and 4.2 authenticating the lists (RSASSA-PSS signer keys included) and clause 4.3 run, EN 319 412-5's QC statements read, clauses 4.4 and 4.5's qualified-certificate and QSCD determinations run over the services' `Qualifications` extensions, and TS 119 172-4 clause 4.4.2's level (QES, qualified seal, AdES/QC, AdES, indeterminate) determined and reported with REQ-4.5-01's scope text as printed, signer, pseudonym, times, attributes, and suite with its potential security issues against TS 119 312 beside it, a signature policy's `TAsInTrustedList` anchors determined from the host's lists, clauses 4.6 and 4.7 run so that a token's authority is anchored by a listed time-stamping service and the report says whether a token is an EU qualified time stamp, a C host's lists taken through `quorra_trusted_lists`, and the lists fetched by a host at the reader's network level and kept only where clauses 4.1 and 4.2 authenticate them, with PRO-4.1.4-16 and -17's configuration persisted, asked for by every window on Shift + T and handed to the core again as `Command::Trust`; the MGF1 hash read (ADR 2905), and TS 119 312 clause 9.3's lifetimes asked of a signer's key (ADR 2927); h)'s remainder, clause 9.5, is `doc/todo/77`'s order |

## 3. The order

Row 1 first, because it is the one whose shape the owner had already given (A45's second paragraph)
and whose only gate was "when the attachments panel is designed" — and the panel is
`Query::Attachments` in four windows. Then rows 4 and 5, each one `Query` or one host history and
a test; row 21's ramp in the oracle (built); row 6's host reads by `doc/todo/30`–`38`; row 10's
measurement as its own measure round, whose result went to the owner with the near-side
construction RFC 0011 section 7 asks for (A840). What is left of the order is row 6's and row 7's
WinUI print dialogue, which only CI compiles and nothing here runs, and row 9 after A254 item 2's
spool path. Rows 2, 3, 28 and 29 are the other lanes' and are listed so
that this file is the whole population.

## 4. What the lift changed in the text

A record keeps its words (ADR 1023); a living document says what is. `doc/todo/01`'s §7.6.5 bullet,
`doc/todo/63`'s "take it when", `doc/questions/A45`'s `Owes:` and the Readings of `A66` and `A168`
now say what A800 did; ADRs 1485, 1672, 1736, 2137 and 2529 are records and are cited from here and
from ADR 2800 instead.
