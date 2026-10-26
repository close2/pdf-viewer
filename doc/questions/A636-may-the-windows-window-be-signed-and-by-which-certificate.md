Status: complete
Given: 2026-10-26, in conversation — transcribed by the round
Owes: none until the gate — a person has run the WinUI window on a Windows desktop and the
`window` job's start on a document has passed; then the project acquires a code-signing identity
(a publicly trusted certificate held as a repository secret, or a cloud signing service CI
calls — either paid for and held by the owner, which no round can invent), CI signs the zip's
executables first, and the MSIX is built only if a user asks for installation (acting round,
on the owner's word that the cost is carried)

> Otherwise I agree with all recommendations.  (only departures: XFA and Q632, which I haven't
> fully understood yet)

Reading: one message answered six questions together and named its two departures; this one
adopts Q636's recommendation as presented, quoted and attributed: *"Not yet. Wait until a
person has run the window on a Windows desktop and the `window` job's start on a document has
passed. Then sign the zip's executables first, and the MSIX only if a user asks for
installation."*

What the answer settles:

- **Nothing is bought before the window is proven on a real desktop**: the unsigned
  self-contained `quorra-winui-x64-<commit>.zip` ships as is, and its README says how to get
  past SmartScreen's "unknown publisher" prompt.
- **The order when it comes is the cheap half first**: sign the zip's executables — SmartScreen's
  prompt goes, or fades as the signature builds reputation — and the MSIX, with its Start menu
  entry, clean uninstall and file associations, only on a user's ask, because it is a second
  package to build and keep working beside the zip.
- **The cost stays the owner's**: an annual certificate fee and a secret CI must guard — a
  signing step holds a credential that can sign anything — which is why the question could not
  be answered by a round and why the answer defers until the purchase buys something proven.