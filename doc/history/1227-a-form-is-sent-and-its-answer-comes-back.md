# 1227 — A form is sent, and its answer comes back

Date: 2026-09-23. Branch `batch-1227-1232`, shared worktree. ADRs 1291, 1292. Question Q130.

## What the owner answered

`A98`: the client built, `ureq` in `viewer-host`, four levels read where `may_submit` is asked,
`ask` by default, `http`/`https` only and checked first, global, on the restriction menu.

## Built (ADR 1291)

- `viewer_host::submit`: `transmit` sends the composed request unchanged; `Submitter` runs each on
  a `submit-form` thread and the window collects the answer on its own timer.
- `may_submit(submission, Submissions)`: scheme first, then `refuse|ask|warn|send`. The words
  are ADR 1155's direction, not the `off`/`on` Q98 wrote.
- A third menu group in all three windows, *What a document may ask this machine to do*.
- `viewer_core::Command::Respond`: an FDF or XFDF answer is imported into the document that sent
  it, `/Status` shown. A PDF answer opens beside. Anything else is said with its media type.
- `openssl`, `native-tls` and `webpki-roots` are banned. `quorra --release` grew by 1 856 104
  bytes. `ring` is C in the host, and Q130 asks about it.

## Found (ADR 1292)

FDF was sent as `application/fdf` on a comment saying the standard named no type. §12.7.8.1 says
`application/vnd.fdf`, and that is what is sent now.

## Rows

§12.7.6.2 `partial` → `implemented`, and §12.7.6 followed (ADR 1261).

## Driven

Under Xvfb against a loopback server: `quorra` (ask → FDF imported; menu → `send` → PDF opened as a
second tab) and `quorra-gtk` (ask dialogue → FDF imported; menu → `refuse` → declined). Qt was not
driven.
