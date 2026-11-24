#!/usr/bin/env bash
# Holds every errata source this project reads, in doc/errata/, and says when one has moved (ADR 3583).
#
#   tools/fetch-errata.sh          fetch, write doc/errata/ and its MANIFEST.tsv
#   tools/fetch-errata.sh --check  fetch into a temporary directory and compare with MANIFEST.tsv's
#                                  hashes; exit 1, naming each file, when any differs
#
# The sources are two, both the PDF Association's and both CC-BY-4.0 (doc/errata/README.md):
#   - the pages https://pdf-issues.pdfa.org/ is built from: the Jekyll source under `docs/` of
#     github.com/pdf-association/pdf-issues, at the commit `main` names when the script runs. The
#     source rather than the built HTML, because it is what the site is made from and it diffs;
#     every Markdown page, the site's data file and the small grammar files, not the figures.
#   - that repository's issues, every state, as JSON Lines: one issue a line with its labels, and
#     one comment a line, which is where a resolution is recorded.
# A comment on any issue changes the second file's hash, which is the point: `--check` failing is
# the prompt to read what moved, never something to quieten.
set -euo pipefail
here=$(cd "$(dirname "$0")/.." && pwd)
out_dir="$here/doc/errata"
repository=pdf-association/pdf-issues
check=0
[ "${1:-}" = "--check" ] && check=1

work=$(mktemp -d)
trap 'rm -rf "$work"' EXIT

commit=$(gh api "repos/$repository/commits/main" --jq .sha)
gh api "repos/$repository/git/trees/$commit?recursive=1" \
    --jq '.tree[] | select(.type == "blob") | .path' > "$work/paths"
grep -E '^(README\.md|LICENSE|PUBLICATION-PROCESS\.md|ERRATA-AS-XFDF\.md|docs/.*\.(md|yml|ebnf))$' \
    "$work/paths" | sort > "$work/wanted"
mkdir -p "$work/site"
while read -r path; do
    mkdir -p "$work/site/$(dirname "$path")"
    curl -fsS --retry 3 -o "$work/site/$path" \
        "https://raw.githubusercontent.com/$repository/$commit/$path"
done < "$work/wanted"

# Only the fields a reading needs, sorted keys, one record a line, ordered by number and id, so
# that the same tracker always writes the same bytes.
gh api --paginate "repos/$repository/issues?state=all&per_page=100&direction=asc" \
    --jq '.[] | {number, title, state, state_reason, labels: [.labels[].name], user: .user.login,
                 created_at, updated_at, closed_at, url: .html_url,
                 pull_request: (.pull_request != null), body}' \
    | jq -cS . | jq -cs 'sort_by(.number) | .[]' > "$work/issues.jsonl"
gh api --paginate "repos/$repository/issues/comments?per_page=100&sort=created&direction=asc" \
    --jq '.[] | {id, issue: (.issue_url | split("/") | last | tonumber), user: .user.login,
                 created_at, updated_at, url: .html_url, body}' \
    | jq -cS . | jq -cs 'sort_by(.issue, .id) | .[]' > "$work/comments.jsonl"

today=$(date -u +%Y-%m-%d)
{
    while read -r path; do
        printf '%s\t%s\thttps://github.com/%s/blob/%s/%s\t%s\n' "site/$path" \
            "$(sha256sum "$work/site/$path" | cut -d' ' -f1)" "$repository" "$commit" "$path" "$today"
    done < "$work/wanted"
    for name in issues comments; do
        printf '%s\t%s\thttps://api.github.com/repos/%s/%s\t%s\n' "$name.jsonl" \
            "$(sha256sum "$work/$name.jsonl" | cut -d' ' -f1)" "$repository" \
            "$([ "$name" = issues ] && echo 'issues?state=all' || echo 'issues/comments')" "$today"
    done
} > "$work/MANIFEST.tsv"

if [ "$check" = 1 ]; then
    [ -f "$out_dir/MANIFEST.tsv" ] || { echo "fetch-errata: no $out_dir/MANIFEST.tsv to check against"; exit 1; }
    moved=$(join -t $'\t' -a1 -a2 -e missing -o 0,1.2,2.2 \
        <(cut -f1,2 "$out_dir/MANIFEST.tsv" | sort) <(cut -f1,2 "$work/MANIFEST.tsv" | sort) \
        | awk -F'\t' '$2 != $3 { print "  " $1 ": held " $2 ", now " $3 }')
    if [ -n "$moved" ]; then
        echo "fetch-errata: the errata sources moved since doc/errata/ was written; read them, then re-run without --check"
        echo "$moved"
        exit 1
    fi
    echo "fetch-errata: every held errata source is unchanged ($(wc -l < "$work/MANIFEST.tsv") files, $repository at $commit)"
    exit 0
fi

rm -rf "$out_dir/site" "$out_dir/issues.jsonl" "$out_dir/comments.jsonl"
mkdir -p "$out_dir"
cp -r "$work/site" "$work/issues.jsonl" "$work/comments.jsonl" "$work/MANIFEST.tsv" "$out_dir/"
echo "fetch-errata: wrote $(wc -l < "$work/MANIFEST.tsv") files from $repository at $commit into doc/errata/"
