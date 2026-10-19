# 1975 — A script the view state does not run is said once, and the refused properties are properties

Session 1569, a script round of batch ninety, `doc/todo/70` items 1 and 5. Status: **accepted,
built** (section 1); section 2 is a reading that moved nothing. Context: ADR 1752 (a host drops the
action path's refusal of a chain whose scripts the runner was handed), ADR 1950 section 3 (a script
past `MAX_SCRIPT_BYTES`, or with an unreadable `/JS`, is said), ADR 1750 (an event before the open
sequence), ADR 1957 section 2 (`Member`). Code: `crates/pdf-model/src/view/script_sites.rs`
(`ScriptsRan::said`, `ScriptsRan::answered`), the three host sites in
`crates/viewer-core/src/{viewer,interact}.rs`. Clauses: §12.6.3 (Table 197), §12.3.3, §12.6.4.17.

## 1. What the view state said is carried to the host

**What was wrong.** At a widget's pointer and focus events, a link's click and an outline item's
activation, a host decided whether to say Tier 0's refusal of the chain's ECMAScript actions from
`ScriptsRan::handed > 0`. Where the view state handed nothing over because it had said why — a text
of more than 1 048 576 bytes, a `/JS` that is neither of Table 221's forms, or an event raised before
the open sequence — the host said Tier 0's sentence as well, "a script this tier does not run — Tier
0 runs a field's one-call AF library scripts and no ECMAScript", which is not why the script did not
run at a level that runs scripts: one script, two sentences, one of them wrong (trap 5).

**The rule.** `ScriptsRan::said` counts the chain's ECMAScript actions the view state did not hand
over and said, with their reason; `ScriptsRan::answered` is `handed > 0 || said > 0`, and a host drops
the action path's refusal on it. Table 198's page events need nothing: their host already drops the
refusal wherever a runner exists. With no runner the view state says nothing at these sites, `said`
is 0, and the action path's refusal is the one sentence, as it was. `tests/script_said_once.rs`
(viewer-core) drives all three sites; its three tests fail with `handed > 0` put back, and its
control passes either way.

## 2. ADR 1957 section 2's eight property rows, against Adobe's reference

The brief's premise was that a row marked `Member::Property` might be a method, whose `typeof` would
then throw where the reference has a function. Read against the *JavaScript for Acrobat API
Reference* (RFC 0008's pinned commit, `JS_API_AcroJS/`), none is:

| row | the reference |
|---|---|
| `Net`, `SOAP` | objects: "The Net object …", "The SOAP object …" |
| `app.fs` | `app` properties: "Type Object Access R" |
| `Collab`, `security`, `SecurityHandler`, `identity` | "This static object …", "a static JavaScript object", objects "obtained using the security.getHandler method", "a static object" |
| `this.media`, `app.media` | `Doc` and `app` properties: "Type Object Access R" |
| `XFA`, `xfa` | "The XFA object provides access to the XFA appModel container" |
| `this.dynamicXFAForm` | `Doc` property: "Type Boolean Access R" |
| `this.documentFileName` | `Doc` property: "Type String Access R" |

So nothing moves, and a read of each still throws its row's `NotAllowedError`. What the reading
found instead is left in `doc/todo/70` item 5: the XFA page's own example tells an Acrobat form from
an XML one by `typeof xfa == "object"`, with the comment "The xfa object is undefined in an Acrobat
form", and here that read throws. Whether a processor that declines Annex K should answer it as an
Acrobat form's is a change to ADR 1957 section 2's rule for one row, and is not taken here.
