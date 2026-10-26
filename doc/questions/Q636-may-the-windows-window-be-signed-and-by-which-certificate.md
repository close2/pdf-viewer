# Q636 — May the Windows window be signed, and by which certificate?

Source: session 1770, the owner's request of 2026-10-26 to "build and release the new windows UI".
ADR 2374 section 2 is the decision this leaves open.

## The question

The snapshot now carries the WinUI window as an unsigned, self-contained zip,
`quorra-winui-x64-<commit>.zip`. Should the project acquire a code-signing identity so that
`Quorra.App.exe` (and the workers beside it) are signed in CI, and so that an MSIX can be offered
beside the zip? The candidates are a publicly trusted code-signing certificate held as a repository
secret, or a cloud signing service that CI calls. Either is paid for and held by the owner, and
neither can be invented by a round.

## What signing buys and costs

- **For**: SmartScreen's "unknown publisher" prompt goes, or fades as the signature builds
  reputation. An MSIX becomes installable, and with it a Start menu entry, a clean uninstall and file
  associations from a manifest.
- **Against**: an annual cost, a secret CI must guard (a signing step holds a credential that can
  sign anything), and an MSIX is a second package to build and keep working beside the zip.

## Recommendation

Not yet. Wait until a person has run the window on a Windows desktop and the `window` job's start on
a document has passed. Then sign the zip's executables first, and the MSIX only if a user asks for
installation.

## What the tree does meanwhile

It ships the unsigned zip. Its README says how to get past SmartScreen's prompt, and no MSIX is
built.
