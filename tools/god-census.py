#!/usr/bin/env python3
"""The god-class census: where one file, one type or one function carries too much.

Four rankings over every `.rs` file under `crates/` and `raster/crates/`, each a different
measure of the same smell, because one measure alone is gamed by the habit it counts (a file
split into one huge `impl` is short by file and long by block):

  files        lines in the file, tests included, with the share that is `#[cfg(test)]`
  impl blocks  lines from `impl` to its closing brace, with the methods it holds
  types        methods summed over every `impl` of one type name (`impl T`, `impl Tr for T`)
  functions    lines from `fn` to its closing brace, nested functions counted in their parent

A type or a block is ranked by what it is compared on, not by a single score: the ordered list in
`doc/todo/74-the-god-classes.md` is a reading of these four tables, not a formula. A bodiless
declaration (a trait method, an `extern` item) is not a function here.

    python3 -I -B tools/god-census.py              # the default top 10 of each table
    python3 -I -B tools/god-census.py --top 25     # a longer list
    python3 -I -B tools/god-census.py --count      # the file and function totals alone

It reads and prints; it rewrites nothing. The scanner understands comments, strings, raw strings
and character literals well enough to count braces; it is not a parser, and a macro that builds
an `impl` out of tokens is not seen.
"""

import os
import re
import sys

ROOT = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))
TREES = ("crates", "raster/crates")

IMPL = re.compile(r"\bimpl\b")
FN = re.compile(r"\bfn\s+([A-Za-z_][A-Za-z0-9_]*)")


def masked(text):
    """The text with comments, strings and character literals blanked, newlines kept."""
    out = []
    i, n = 0, len(text)
    while i < n:
        c = text[i]
        two = text[i : i + 2]
        if two == "//":
            j = text.find("\n", i)
            j = n if j < 0 else j
            out.append(" " * (j - i))
            i = j
        elif two == "/*":
            depth, j = 1, i + 2
            while j < n and depth:
                if text.startswith("/*", j):
                    depth += 1
                    j += 2
                elif text.startswith("*/", j):
                    depth -= 1
                    j += 2
                else:
                    j += 1
            out.append("".join(ch if ch == "\n" else " " for ch in text[i:j]))
            i = j
        elif c == "r" and re.match(r'r#*"', text[i : i + 20]) and not (i and (text[i - 1].isalnum() or text[i - 1] == "_")):
            m = re.match(r'r(#*)"', text[i:])
            end = '"' + m.group(1)
            j = text.find(end, i + m.end())
            j = n if j < 0 else j + len(end)
            out.append("".join(ch if ch == "\n" else " " for ch in text[i:j]))
            i = j
        elif c == '"':
            j = i + 1
            while j < n and text[j] != '"':
                j += 2 if text[j] == "\\" else 1
            j = min(j + 1, n)
            out.append("".join(ch if ch == "\n" else " " for ch in text[i:j]))
            i = j
        elif c == "'":
            m = re.match(r"'(\\.[^']*|[^\\'])'", text[i:])
            if m:
                out.append(" " * m.end())
                i += m.end()
            else:
                out.append(c)
                i += 1
        else:
            out.append(c)
            i += 1
    return "".join(out)


def block_end(code, start):
    """Index of the brace closing the first `{` at or after `start`; -1 if a `;` comes first."""
    depth = 0
    for j in range(start, len(code)):
        ch = code[j]
        if depth == 0 and ch == ";":
            return -1
        if ch == "{":
            depth += 1
        elif ch == "}":
            depth -= 1
            if depth == 0:
                return j
    return -1


def type_name(header):
    """The self type of an `impl` header: the name after `for`, else the first path's last segment."""
    header = re.sub(r"^impl\s*(<[^{]*?>)?\s*", "", header, count=1) if not header.startswith("impl<") else header
    # strip the generics list of `impl<...>` by bracket matching
    h = header.strip()
    if h.startswith("impl"):
        h = h[4:].strip()
    if h.startswith("<"):
        depth = 0
        for k, ch in enumerate(h):
            depth += ch == "<"
            depth -= ch == ">"
            if depth == 0:
                h = h[k + 1 :].strip()
                break
    h = re.split(r"\bwhere\b", h)[0]
    if re.search(r"\bfor\b", h):
        h = re.split(r"\bfor\b", h, maxsplit=1)[1]
    h = h.strip().lstrip("&").strip()
    m = re.match(r"(?:dyn\s+)?([A-Za-z_][A-Za-z0-9_:]*)", h)
    if not m:
        return "?"
    return m.group(1).split("::")[-1]


def scan(path, rel):
    with open(path, encoding="utf-8", errors="replace") as f:
        text = f.read()
    code = masked(text)
    starts = [0]
    for m in re.finditer("\n", code):
        starts.append(m.end())

    def line_of(index):
        lo, hi = 0, len(starts) - 1
        while lo < hi:
            mid = (lo + hi + 1) // 2
            if starts[mid] <= index:
                lo = mid
            else:
                hi = mid - 1
        return lo + 1

    total = text.count("\n") + (0 if text.endswith("\n") or not text else 1)
    test_lines = 0
    for m in re.finditer(r"#\[cfg\(test\)\]\s*(?:pub\s+)?mod\s+\w+\s*\{", code):
        e = block_end(code, m.end() - 1)
        if e > 0:
            test_lines += line_of(e) - line_of(m.start()) + 1
    impls, fns = [], []
    for m in IMPL.finditer(code):
        before = code[max(0, m.start() - 1) : m.start()]
        if before.isalnum() or before == "_":
            continue
        brace = code.find("{", m.end())
        semi = code.find(";", m.end())
        if brace < 0 or (0 <= semi < brace):
            continue
        header = code[m.start() : brace]
        # `impl Trait` in argument or return position has no body before the next `{` of a fn
        if re.search(r"[(,:>]\s*$", code[max(0, m.start() - 4) : m.start()]) and "fn" in code[max(0, m.start() - 80) : m.start()]:
            continue
        end = block_end(code, brace)
        if end < 0:
            continue
        body = code[brace:end]
        methods = len(FN.findall(body))
        impls.append((type_name(header), line_of(m.start()), line_of(end) - line_of(m.start()) + 1, methods))
    for m in FN.finditer(code):
        brace_or_semi = re.compile(r"[{;]").search(code, m.end())
        if not brace_or_semi or brace_or_semi.group() == ";":
            continue
        # a `where` clause or return type may hold `{`-free text only; the first `{` is the body
        end = block_end(code, brace_or_semi.start())
        if end < 0:
            continue
        fns.append((m.group(1), line_of(m.start()), line_of(end) - line_of(m.start()) + 1))
    return {"rel": rel, "lines": total, "tests": test_lines, "impls": impls, "fns": fns}


def files():
    for tree in TREES:
        base = os.path.join(ROOT, tree)
        for dirpath, dirnames, names in os.walk(base):
            dirnames[:] = sorted(d for d in dirnames if d not in ("target", "fuzz"))
            for name in sorted(names):
                if name.endswith(".rs"):
                    path = os.path.join(dirpath, name)
                    yield path, os.path.relpath(path, ROOT)


def table(title, header, rows, top):
    print(f"\n{title}")
    print("  " + header)
    for row in rows[:top]:
        print("  " + row)


def main(argv):
    top = 10
    count_only = False
    args = list(argv)
    while args:
        a = args.pop(0)
        if a == "--top":
            top = int(args.pop(0))
        elif a == "--count":
            count_only = True
        else:
            print(__doc__)
            return 2
    scans = [scan(p, r) for p, r in files()]
    n_fns = sum(len(s["fns"]) for s in scans)
    print(f"{len(scans)} files, {sum(s['lines'] for s in scans)} lines, {n_fns} functions")
    if count_only:
        return 0
    by_file = sorted(scans, key=lambda s: -s["lines"])
    table("files by lines", "lines  of-which-tests  file",
          [f"{s['lines']:>5}  {s['tests']:>14}  {s['rel']}" for s in by_file], top)
    blocks = sorted(((i[2], i[3], i[0], s["rel"], i[1]) for s in scans for i in s["impls"]), reverse=True)
    table("impl blocks by lines", "lines  methods  type  file:line",
          [f"{b[0]:>5}  {b[1]:>7}  {b[2]}  {b[3]}:{b[4]}" for b in blocks], top)
    types = {}
    for s in scans:
        for name, _line, lines, methods in s["impls"]:
            t = types.setdefault((name, s["rel"].split("/src/")[0]), [0, 0, set()])
            t[0] += methods
            t[1] += lines
            t[2].add(s["rel"])
    ranked = sorted(types.items(), key=lambda kv: (-kv[1][0], kv[0]))
    table("types by methods (all impls of one name within a crate)", "methods  impl-lines  files  type  crate",
          [f"{v[0]:>7}  {v[1]:>10}  {len(v[2]):>5}  {k[0]}  {k[1]}" for k, v in ranked], top)
    longest = sorted(((f[2], f[0], s["rel"], f[1]) for s in scans for f in s["fns"]), reverse=True)
    table("functions by lines", "lines  name  file:line",
          [f"{f[0]:>5}  {f[1]}  {f[2]}:{f[3]}" for f in longest], top)
    return 0


if __name__ == "__main__":
    sys.exit(main(sys.argv[1:]))
