# 1980 — An XFA `submit`'s `xdp` and `xml` packagings are written from the saved form

Session 1980, a single ledger round (K.2), the ledger lane. ADR 2771; 2772 not used; no question.
**The slice, and why.** The `xdp` and `xml` packagings: one construction (XML of the form's own,
from the saved file) that fits a round; the other events are many small slices of their own.

**The premise held.** `xfa::submit::Submit::action` refused both by name ("which this reader does
not write"); page 859 defines each body, chapters 23 and 4 the packaging, and the host's submit
level (`doc/questions/A98`) already decides what leaves the machine. Effort only, no owner's word.

**Built.** `submission::compose_packaged` saves (`ViewState::save`, where §K.2's consistency is
kept), opens what it wrote, and has `xfa::Packaging::body` write: for `xdp`, the resource read whole
(`xfa::resource_text`, §K.2's array concatenated, a rootless one inside page 1041's root), the
producer's root start tag, the packets page 861's `xdpContent` selects, page 1039's `pdf` (base64,
RFC 2045, where `embedPDF="1"`; empty at the underlined `0`) and page 1042's `xfdf` (empty with no
markup annotation, else left out and said); for `xml`, page 149's data document, the one element of
`xfa:data`, with the namespaces in scope (`xml::Tree::extracted`, from new element spans and
declarations). `FormSubmit::body` is a `FormBody`; `viewer-core`'s `carry_out` composes either;
`submission::Format` gains `Xdp`, `Xml` (confined frame 4, 5). No host changed: each sends
`media_type` and `body`. Choices, said in ADR 2771: omitted `xdpContent` = `datasets pdf xfdf`,
`*` = every packet, `text/xml` for `xml`. `signData`/`encrypt`/`encryptData` and an `execute`'s
WSDL call are now refused by name under the JavaScript exclusion, not owed.

**Tests.** `xml.rs`: namespaces kept on an extracted element. `package.rs`: RFC 4648 vectors in
76-character lines; `xdpContent` read. `cascade.rs`: each packaging held as itself; `formdata` and
an unlisted format still said. `script_form.rs` at `off`: the `xdp` body with the entry `5` in
`datasets`, no template, empty `pdf` and `xfdf`; `embedPDF="1"` with `template pdf config` (chunk
starts `JVBE`, the missing `config` said); the `xml` body, exact. The confined frame's round trip.
**Plants**, applied together and reversed: the body written from the unsaved document, the
namespace injection dropped, `embedPDF` ignored — four tests fail; reversed, all pass.

**Gates.** fmt `--check` and clippy `-D warnings --all-targets` of `pdf-model`, `pdf-formcalc`,
`viewer-core`, `viewer-confined`: exit 0. nextest of the four: 3088 of 3088, 25 skipped.
`cargo test -p conformance` after `git add -N`: exit 0. `state.sh --round 1980 docs`: exit 0.
Tier 2 under `bounded.sh --lock`: `pdf-model script_corpus` 0 (67 445 PDFs walked, safedocs'
65 968 among them; tika-issue-tracker and openpreserve absent; 5 929 s), `selection_census` 0;
`viewer-confined awkward_classes` queued behind the other lane's docs at the report. No host changed: each sends the submission's
`media_type` and `body` as they come, and the frame is `viewer-confined`'s.

**Left of the row**: the events other than the sixteen carried out, and RFC 0011 step 5's.
