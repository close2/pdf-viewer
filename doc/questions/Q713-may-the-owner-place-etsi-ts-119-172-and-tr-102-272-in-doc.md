# Q713 — May the owner place ETSI TS 119 172-1 to -4 and TR 102 272 in `doc/`?

Source: round 1847, reading §12.8.3.4.4's last `partial` sentence ("a validator shall enforce
signature policy constraints") for whether its block is in reach (ADRs 1219, 1709, 1728, 2529).

## Where the clause stands

The row is `partial` on one requirement: enforcing the constraints of a signature policy, which
need a machine-readable policy document in the syntax the signature names (EN 319 122-1 clause
5.2.9.2's specification qualifier). Everything else is built — the profile, Table 1's
cardinalities, the policy's identifier, digest, binding and fetch. No signature on this disk names
a syntax (ADR 1709); the one policy most of them sign under is a human-readable PDF.

## What was tried

- RFC 3125 (the ASN.1 syntax, category Experimental) is free and was fetched without trouble.
- ETSI TR 102 272, ETSI TS 119 172-1 (the document on signature policies that EN 319 122-1 and
  EN 319 122-2 point to) and EN 319 122-2 itself, which ADR 1085 fetched in a minute, all answer
  this machine's client with HTTP 403 today. Presenting the client as a browser would be
  circumventing a block, so it was not tried.

## Options

1. **You place the PDFs in `doc/`** (they are untracked and ignored, ETSI's notice permits no
   reproduction), and a later round builds the enforcement over them with synthetic fixtures.
   Recommended, if you want the row `implemented`: the enforcement is a real build (a policy parser,
   the signing period, mandated attributes, algorithm constraints, commitment rules), and the
   document on signature policies that EN 319 122-1 and EN 319 122-2 point to is TS 119 172-1, not
   RFC 3125.
2. **Nothing is placed, and the row stays `partial`** with the block named as outside the tree.
   This is what the tree does meanwhile.
3. **RFC 3125 alone is built.** Not recommended: no signature in reach states a policy in it, and the
   row would stay `partial` because the document the profile's own references point to for
   policies is TS 119 172-1, which would still not be read.
