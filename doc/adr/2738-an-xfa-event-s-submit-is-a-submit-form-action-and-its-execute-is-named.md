# 2738 — An XFA event's `submit` is a submit-form action, and its `execute` is named

Session 1961, a ledger round (K.2). Status: implemented. Extends ADR 2702 (the pre- and post-submit
events) and ADR 1062 (a submission is composed in the core and sent by a host); reuses
`pdf_model::submission::compose` and `doc/questions/A98`'s submit level unchanged.

## Context

XFA 3.3 page 703 gives an `event` one of five properties: `script`, `submit`, `execute`,
`signData`, `encryptData`. Every property but `script` was reported where its event fired as "not
performed (page 703)". In a conforming hybrid file Annex K keeps `A` and `AA` off the widget of a
field the XFA resource gives actions, so a template's `<event activity="click"><submit …/></event>`
is the only statement of a submit button the file makes, and this reader dropped it.

Page 858 makes `submit` an HTTP POST of the form's content to its `target`; page 859 lists six
packagings in `format`: `xdp` (underlined, so the default by page 597's convention), `formdata`,
`pdf`, `urlencoded`, `xfd`, `xml`. Page 453 orders a client's submit: page 446's pre-submit steps
(the `preSubmit` events and the validations), packaging, signing by any `signData` children,
encryption by the `encrypt` child, the POST, and page 447's post-submit steps. Pages 722–723 make
`execute` a call of a WSDL web service named by a connection of the `connectionSet` packet, which
a client ignores where `runAt` is `server`; an `execute` without a `connection` is an error.

## Decision

1. **Three packagings are §12.7.6.2's.** `pdf` is Table 240 bit 9's whole document, `xfd` bit 6's
   XFDF, `urlencoded` bit 3's HTML Form format. `xfa::submit::Submit::action` states the `submit`
   as the `SubmitForm` whose flags ask for that body: `/F` its `target`, `/Fields` empty, bit 4
   clear (page 858's POST) and bit 2 set, since page 445 sends the form's variable data, an empty
   field's included. A `textEncoding` other than UTF-8 is named on what the submit owes, as Table
   239's `/CharSet` is, and the body is written in UTF-8.
2. **What cannot be composed is said and nothing is sent**: `xdp` and `xml` (chapter 23's packaging
   and chapter 15's unloading, not written), `formdata` (page 859 describes it only as emulating
   unnamed legacy software — a silence, recorded as one), a format page 858 does not list, a
   `submit` with no `target` (page 860 leaves the URI to the product), and one holding `signData`,
   `encrypt` or `encryptData`, because sending content the form asked to have signed or encrypted
   unsigned or in the clear sends what the form did not ask to send.
3. **The activation holds; the window carries out.** `Session::perform` holds a composable submit
   as an `xfa::FormSubmit` on `Activation::submits` (at most `MAX_SUBMITS`, four; the rest said),
   with the field a click was within, which the `preSubmit` events read as their primary event
   (page 398). The view state holds them until `ViewState::take_form_submits`. `viewer-core`'s
   `Viewer::handle` takes them after every command (`send_form_submits`) and runs each through the
   path a submit-form action takes (`interact::carry_out`): `preSubmit`, `compose`, `Event::Submit`
   to the host's submit policy, and `postSubmit` where nothing was sent. An activation ended on a
   budget keeps no submit, as it keeps no value.
4. **No submit inside a submit.** A `submit` performed by a `preSubmit` or `postSubmit` event is
   said and not held: page 446 pairs pre- and post-events around one transaction and says nothing
   of one begun inside another, and a `postSubmit` that submits would start a new submit at every
   answer a server sends. A choice where the pages are silent.
5. **`execute`**: `runAt="server"` runs nothing and says nothing, which is all page 722 asks of a
   client; otherwise the call is named ("calls the web service of the connection …, which this
   reader does not call"), and a missing connection is named as page 722's error.

## Consequences

- A submit is performed where an activation runs: in a window whose reader lets scripts run or
  has been asked. At `off` no runner exists and nothing of the form is activated, so a `submit` is
  not performed there; the confined window is pinned `off` (ADR 1616). A submit is no script, and
  the path that performs it without a runner is left on K.2's list.
- `signData` and `encryptData` as an event's own property stay "not performed (page 703)".
- Owed on K.2: the `xdp` and `xml` packagings, an `execute`'s WSDL call, XML signatures and
  encryption, and a submit at `off`.
- Tests: `crates/pdf-formcalc/tests/cascade.rs` (the activation: formats, refusals, the encoding,
  `execute`, the pre/post-submit guard, the bound) and `crates/viewer-core/tests/script_form.rs`
  (a click hands the host a PDF submission after `preSubmit` ran, and nothing at `off`).
