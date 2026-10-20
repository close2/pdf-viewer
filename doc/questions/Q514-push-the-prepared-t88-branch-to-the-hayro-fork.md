# Q514 — Push the prepared T.88 branch to `close2/hayro`?

Source: session 1619 (batch ninety-seven, slot 5), ADR 2074; the patches are ADRs 1447, 1459 and
1485.
Status: **open** — answered when `A514-…` exists beside this file.

## Why it needs the owner

§7.4.7 is `partial` for one reason. The pinned `hayro-jbig2` reads T.88 7.4.6.2's EXTTEMPLATE bit
and ignores it, so `pdf_sandbox::decode` refuses such a region by segment (ADR 1459). The fix is
`doc/patches/hayro-jbig2-extended-template.patch`, and it lands in your fork, which a round can
neither push to nor try to. None of the fork's four branches carries any of the three patches.
They are now on a branch of their own, ready to push.

## What is prepared

`/home/AI/hayro-fork/` is a clone of `https://github.com/close2/hayro` with no working tree. Branch
`quorra/t88-patches` is three commits on `ea9c81dc70cde40c64205986fdcfcd481fe934d8`, the commit the
manifest pins today (upstream's `hayro-jbig2-v0.3.1`). Its tip is
`7e39af411b807cc267fb7a72545083f10ca2077f`. The base is the pin and not your `main`, because `main`
is 31 upstream commits older and would take `hayro-jpeg2000` back past 0.4.1 (ADR 2074 section 2).
`/home/AI/hayro-fork/quorra-t88-patches.bundle` holds the same three commits, 13 791 bytes; it needs
`ea9c81dc`.

## The push

`/home/AI` is readable only by the agent's user. From your own clone of the fork:

```
git fetch https://github.com/LaurenzV/hayro refs/tags/hayro-jbig2-v0.3.1:refs/tags/hayro-jbig2-v0.3.1
sudo -u AI cat /home/AI/hayro-fork/quorra-t88-patches.bundle > /tmp/quorra-t88-patches.bundle
git fetch /tmp/quorra-t88-patches.bundle quorra/t88-patches:quorra/t88-patches
git rev-parse quorra/t88-patches   # 7e39af411b807cc267fb7a72545083f10ca2077f
git push origin quorra/t88-patches
```

Fetching the bundle keeps the commit hashes, so the rev the manifest will name is the one above.

## What the tree does meanwhile, and after

Meanwhile the row stays `partial` and the refusal stays, out loud. Once the branch is on GitHub,
the next round can do the following without you (ADR 2074 section 4):

1. Point the three stanzas at `close2/hayro` at `7e39af41`, and add the fork to `deny.toml`'s `allow-git`.
2. Run `cargo update -p hayro-jbig2 -p hayro-jpeg2000 -p hayro-ccitt`.
3. Delete the three patch files.
4. Delete `decode::extended_template_region`'s refusal, and turn its test into a decode.
5. Move §7.4.7 and its headings to `implemented`.
6. Compare the six arms by digest.

## Recommendation

**Push it.** Upstream has implemented the same clause on its own, in #1485 (`e7c49ef5`,
2026-10-13). That commit is unreleased and comes with 118 other commits. Waiting for a release
carrying it is the alternative, and it would leave the refusal standing until then. If you would
rather wait, say so in A514. The prepared branch then stays where it is, and the row's note says
which release it waits for.
