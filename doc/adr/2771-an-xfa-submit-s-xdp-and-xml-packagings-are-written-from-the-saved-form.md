# 2771 — An XFA `submit`'s `xdp` and `xml` packagings are written from the saved form

Session 1980, a ledger round (K.2). Status: implemented. Extends ADR 2738 (an event's `submit` is a
submit-form action) and ADR 2767 (a `submit` is carried out at every level); keeps the host's submit
level (`doc/questions/A98`) as what decides whether anything leaves the machine.

## Context

XFA 3.3 page 859 lists six values for a `submit`'s `format`. ADR 2738 carried out the three that
are bodies a submit-form action already composes — `pdf`, `xfd`, `urlencoded` as Table 240 bits 9,
6 and 3 — and said the rest without sending. Two of the rest are XML of the form's own: `xdp`, "the
data is packaged in XDP format, as described in XDP Specification", which page 858 underlines and so
is what a `submit` stating no `format` asks for (page 597), and `xml`, the data "packaged in XML
format … The schema is determined according to the same rules used for data unloading". Neither has
a bit in Table 240, so neither can be stated as a `SubmitForm`. Effort was the only obstacle: the
host's submit level already decides what leaves the machine for every other packaging.

## Decision

1. **The data is the saved form's.** `submission::compose_packaged` calls `ViewState::save`, opens
   the bytes it wrote, and writes the body from that document. §K.2's consistency of the `datasets`
   packet with the `/V` entries is kept in one place, the save, and a second account of the data
   could send values the saved file does not hold. A password field the save keeps out and a field
   it could not write into the packet are named, as for a whole-document submission; page 447's note
   still runs the validations over the whole form.
2. **`xdp` is the resource itself, filtered.** §K.2 says the resource's format "is described in the
   XML Data Package (XDP) Specification", and the array form's first and last packets are the
   `xdp:xdp` element's tags, so the resource read whole (`xfa::resource_text`) is an XDP document.
   The body is its root's start tag as the producer wrote it (the `uuid` and `timeStamp` page 1042
   has every later serialiser keep), the root's packets `xdpContent` selects (page 861), and two
   packets this reader writes from the file it submits: `pdf` (page 1039), base64 per RFC 2045
   where `embedPDF` is `1`, empty where it is `0` because page 858's URI comes from a path this
   process is never given; and `xfdf` (page 1042), empty where the document holds no markup
   annotation (the XFDF specification's content model makes every child of `xfdf` optional), left
   out and said where it holds some, since this program writes no XFDF annotations. A tag naming a
   packet the resource does not hold is said. An array lacking §K.2's begin and end packets is read
   inside page 1041's root, declared as page 1035 asks.
3. **`xml` is the data document.** Page 149's unloader writes "an XML data document which reflects
   the contents of the XML Data DOM"; in a `datasets` packet that is the one element `xfa:data`
   holds, written with the namespace declarations in scope around it (`xml::Tree::extracted`), so a
   `xfa:dataNode` attribute keeps its prefix. A `data` holding no element or several is no XML
   document, and is refused by name.
4. **Through the same path.** `xfa::FormSubmit::body` is `FormBody::Action` or
   `FormBody::Packaged`; `viewer-core`'s `carry_out` composes either between the form's `preSubmit`
   and `postSubmit` events, and the result is the `Event::Submit` every host already sends under its
   submit level. `submission::Format` gains `Xdp` and `Xml`, the confined protocol's frame encodes
   them as 4 and 5, and no host changes: each sends `media_type` and `body` as they come.

## Choices where the pages are silent

- `xdpContent` is cdata, so no underline names its default; an omitted one is read as page 861's
  first value, `datasets pdf xfdf`. `*`, "\[a]ll data elements", is read as every packet.
- The `xml` body's media type is `text/xml`, the type page 1038 gives the `datasets` packet; the
  `xdp` body's is page 1041's registered `application/vnd.adobe.xdp+xml`.
- Both are written in UTF-8; another `textEncoding` (page 860) is said, as for `urlencoded`.

## Refused, by name

`signData`, `encrypt` and `encryptData` (page 453's steps 3 and 4) need signing certificates and
XML encryption that reach outside the document, and an `execute`'s WSDL call (page 722) reaches a
server: `CLAUDE.md`'s JavaScript exclusion keeps every effect that leaves the document and this
viewer outside, so each stays refused and said, not owed. `formdata` stays said: page 859 names
neither the legacy software it emulates nor its encoding.

## Consequences

- A hybrid form's submit button whose `submit` states no `format` now sends, at every level.
- Tests: `crates/pdf-model/src/xfa/xml.rs` (namespaces kept), `crates/pdf-model/src/xfa/package.rs`
  (base64, `xdpContent`), `crates/pdf-formcalc/tests/cascade.rs` (each packaging held as itself),
  `crates/viewer-core/tests/script_form.rs` (the `xdp` body with an entry, the embedded document and
  a missing packet said, the `xml` body), and the confined frame's round trip.
