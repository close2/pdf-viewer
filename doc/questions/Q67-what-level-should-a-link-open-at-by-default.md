# Q67 — What level should §12.6.4.8's link open at by default?

Source: round 1159, building the host surface §12.6.4.8's row owed (ADR 1155).

## What was built, and the one value that is a policy rather than a reading

A URI action now reaches a person. `viewer_host::Links` is `CLAUDE.md`'s four levels over the one
act the clause describes — "[a] URI action causes a URI to be resolved" — set by `--links=` in all
three windows: `refuse` hands it to nothing, `ask` puts the URI to the person and hands it over on
a `yes`, `warn` opens it and says so, `open` opens it silently. Two things are refused before the
level is consulted and neither is a thing `open` turns on: a reference nothing could resolve, and
any scheme outside `http`, `https`, `mailto` (`file` deliberately among them, on §12.7.6.4's
neighbouring hazard). A window with no dialogue answers `ask` by handing nothing over.

Everything above is derived. **The default is not**, and it is the one value a round should not
settle alone.

## The two candidates

- **`ask` (what this round shipped).** Nothing reaches another program without a person pressing a
  key, and nobody has to configure the viewer before a link works. Its cost: a document's
  `/OpenAction` may be a URI action, so a file can put one modal in front of a reader who has done
  nothing but open it. Nothing is opened by that modal — but the file chose to raise it.
- **`refuse`.** Consistent with every other host policy in `viewer_host::policy` — `may_submit`
  refuses, `trust_anchors` answers *nobody*, `may_write_extracted` declines — each on the rule that
  a decision about *this machine* takes the narrowest answer until a person says otherwise. Its
  cost: a reader who clicks a link gets a sentence instead of a browser until they find the word,
  and the level that makes the clause's own act reachable is one nobody meets.

## Recommendation

**Keep `ask`.** The narrowest-answer rule was written for policies a document cannot trigger and a
person cannot see — a network request, a certificate store, a file written to disk. This one is a
person clicking a link they are looking at, and the prompt shows the resolved URI whole before
anything is started. `refuse` would be this program imposing on the reader the thing `CLAUDE.md`
forbids a document to impose, one direction over.

Either answer is one constant: `impl Default for viewer_host::Links`, with the argument in ADR 1155
section 2 and one line in `doc/todo/38`.
