#!/usr/bin/env python3
"""Read-only comparison of local Annex D markdown tables to compiled-in font maps.

Prints only populations and mismatch code indices; never prints or writes a table.
Run from the repository root (no Cargo or repository mutation):
  python3 scratchpad/r2038/spec-audit/annex_d_encoding/drafts/compare_local_annex_d.py

Provenance: primary ISO_32000-2_sponsored_EC3.md (Annex D Tables D.2, D.4–D.6),
crates/pdf-font/src/encoding.rs, and the exact read-fonts version selected by
crates/pdf-font/Cargo.toml/Cargo.lock (0.44.0 here), ps/encoding.rs + ps/string.rs.
"""
from __future__ import annotations

import re
import subprocess
import sys
import tomllib
from pathlib import Path

ROOT = Path.cwd()
MD = ROOT / "doc/md/ISO_32000-2_sponsored_EC3.md"
CODE = ROOT / "crates/pdf-font/src/encoding.rs"
FAILURES: list[str] = []


def table_lines(start: str, end: str) -> list[list[str]]:
    text = MD.read_text(encoding="utf-8")
    a = text.index(start)
    b = text.index(end, a)
    rows = []
    for line in text[a:b].splitlines():
        if line.lstrip().startswith("|"):
            rows.append([cell.strip() for cell in line.strip().strip("|").split("|")])
    return rows


def rust_body(text: str, name: str) -> str:
    match = re.search(rf"\b(?:static|const)\s+{re.escape(name)}\b[^=]*=\s*&?\s*\[(.*?)\];", text, re.S)
    if not match:
        raise RuntimeError(f"cannot locate Rust array {name}")
    return re.sub(r"//.*", "", match.group(1))


def rust_names(name: str) -> dict[int, str]:
    body = rust_body(CODE.read_text(encoding="utf-8"), name)
    vals = re.findall(r'"([^"\\]*(?:\\.[^"\\]*)*)"', body)
    if len(vals) != 256:
        raise RuntimeError(f"{name}: expected 256 names, found {len(vals)}")
    return {i: val for i, val in enumerate(vals) if val}


def octal(value: str) -> int | None:
    value = value.strip()
    return int(value, 8) if re.fullmatch(r"[0-7]{3}", value) else None

def clean_name(value: str) -> str:
    # Markdown conversion keeps footnote markers in cells (e.g. "Euro 1").
    return re.sub(r"\s+[1-9]$", "", value.strip())


def d2_expected() -> dict[str, dict[int, str]]:
    result = {key: {} for key in ("STD", "MAC", "WIN", "PDF")}
    for row in table_lines("## D.2 Latin", "## D.3"):
        if len(row) != 12 or row[0] in ("CHAR", "--------"):
            continue
        for base in (0, 6):
            if not row[base + 1] or base + 1 >= len(row):
                continue
            name = clean_name(row[base + 1])
            for key, col in (("STD", 2), ("MAC", 3), ("WIN", 4), ("PDF", 5)):
                code = octal(row[base + col])
                if code is not None:
                    old = result[key].get(code)
                    if old is not None and old != name:
                        raise RuntimeError(f"D.2 duplicate {key} code {code:o}: {old}/{name}")
                    result[key][code] = name
    return result


def compare(label: str, expected: dict[int, str], actual: dict[int, str]) -> tuple[int, int, list[int]]:
    keys = set(expected) | set(actual)
    mismatches = sorted(code for code in keys if expected.get(code) != actual.get(code))
    print(f"{label}: source={len(expected)} implementation={len(actual)} mismatches={len(mismatches)}")
    if mismatches:
        print(f"  mismatch code indices (octal): {' '.join(f'{c:03o}' for c in mismatches)}")
        FAILURES.append(label)
    return len(expected), len(actual), mismatches


def d_table(start: str, end: str, groups: tuple[tuple[int, int, int], ...]) -> tuple[dict[int, str], dict[int, str]]:
    names: dict[int, str] = {}
    chars: dict[int, str] = {}
    for row in table_lines(start, end):
        for char_col, name_col, code_col in groups:
            if max(char_col, name_col, code_col) >= len(row):
                continue
            name = clean_name(row[name_col])
            char_cell = row[char_col].strip()
            if not name and re.search(r"\s+a[0-9]+$", char_cell):
                name = char_cell.rsplit(None, 1)[1]
            code = octal(row[code_col])
            if not name or code is None:
                continue
            if code in names and names[code] != name:
                raise RuntimeError(f"duplicate table code {code:o}: {names[code]}/{name}")
            names[code] = name
            char = char_cell
            char = re.sub(r"\s+a[0-9]+$", "", char)
            if char:
                chars[code] = char
    return names, chars


def d6_from_primary_pdf() -> tuple[dict[int, str], dict[int, str]]:
    """Extract Table D.6 body rows from primary PDF pages 890–891, not converted markdown."""
    pdf = ROOT / "doc/ISO_32000-2_sponsored_EC3.pdf"
    text = subprocess.check_output(
        ["pdftotext", "-f", "890", "-l", "891", "-layout", str(pdf), "-"],
        text=True,
    )
    started = False
    names: dict[int, str] = {}
    chars: dict[int, str] = {}
    for line in text.splitlines():
        if "Table D.6" in line:
            started = True
            continue
        if not started:
            continue
        cells = re.split(r" {2,}", line.rstrip())
        if len(cells) != 12:
            continue
        for char_col, name_col, code_col in ((0, 1, 2), (3, 4, 5), (6, 7, 8), (9, 10, 11)):
            name = clean_name(cells[name_col])
            code = octal(cells[code_col])
            if not name or code is None:
                continue
            if code in names and names[code] != name:
                raise RuntimeError(f"duplicate primary PDF D.6 code {code:o}")
            names[code] = name
            char = cells[char_col].strip()
            if char:
                chars[code] = char
    return names, chars


def expected_standard_from_dependency() -> dict[int, str]:
    lock = tomllib.loads((ROOT / "Cargo.lock").read_text(encoding="utf-8"))
    workspace = tomllib.loads((ROOT / "Cargo.toml").read_text(encoding="utf-8"))
    dependency = workspace["workspace"]["dependencies"]["read-fonts"]
    version = dependency["version"]
    if version != "0.44.0" or not any(p["name"] == "read-fonts" and p["version"] == version for p in lock["package"]):
        raise RuntimeError("comparison needs review after read-fonts dependency change")
    # Select the read-fonts version used by pdf-font's skrifa dependency.
    # Resolve the `read-fonts` package reached by the workspace `skrifa` 0.47.0 stanza.
    read_fonts_dir = Path.home() / ".cargo/registry/src"
    candidates = list(read_fonts_dir.glob("*/read-fonts-0.44.0/src/ps"))
    if not candidates:
        raise RuntimeError("read-fonts 0.44.0 source unavailable in local Cargo registry")
    ps = candidates[0]
    enc_text = (ps / "encoding.rs").read_text(encoding="utf-8")
    strings_text = (ps / "string.rs").read_text(encoding="utf-8")
    enc_body = rust_body(enc_text, "STANDARD_ENCODING")
    enc_body = re.sub(r"//.*", "", enc_body)
    sids = [int(x) for x in re.findall(r"\b\d+\b", enc_body)]
    string_body = rust_body(strings_text, "STANDARD_STRINGS")
    strings = re.findall(r'"([^"\\]*(?:\\.[^"\\]*)*)"', string_body)
    if len(sids) != 256 or not strings:
        raise RuntimeError(f"bad dependency parse: {len(sids)} codes, {len(strings)} strings")
    return {code: strings[sid] for code, sid in enumerate(sids) if sid and sid < len(strings)}


def dingbat_chars_from_code() -> dict[int, str]:
    body = rust_body(CODE.read_text(encoding="utf-8"), "ZAPF_DINGBATS_CHARACTERS")
    values = []
    # Regex alternation returns empty capture for None and glyph for Some.
    for match in re.finditer(r"\b(None|Some\('([^']*)'\))", body):
        values.append(None if match.group(1) == "None" else match.group(2))
    if len(values) != 256:
        raise RuntimeError(f"D.6 implementation char array has {len(values)} entries")
    return {i: value for i, value in enumerate(values) if value}


def main() -> int:
    d2 = d2_expected()
    impl_mac = rust_names("MAC_ROMAN")
    impl_win = rust_names("WIN_ANSI")
    dep_standard = expected_standard_from_dependency()
    compare("D.2 StandardEncoding vs read-fonts StandardEncoding", d2["STD"], dep_standard)
    note_mac = {0o312: "space"}
    note_win = {code: name for code, name in {
        0o177: "bullet", 0o201: "bullet", 0o215: "bullet", 0o217: "bullet",
        0o220: "bullet", 0o235: "bullet", 0o240: "space", 0o255: "hyphen",
    }.items()}
    compare("D.2 MacRomanEncoding incl. note 6", d2["MAC"] | note_mac, impl_mac)
    compare("D.2 WinAnsiEncoding incl. notes 3 and 4", d2["WIN"] | note_win, impl_win)
    print(f"D.2 body rows: MacRoman={len(d2['MAC'])} WinAnsi={len(d2['WIN'])}; note additions: MacRoman={len(note_mac)} WinAnsi={len(note_win)}")

    d4, _ = d_table("## D.4 Expert", "## D.5 Symbol", ((0, 1, 2), (3, 4, 5)))
    d5, _ = d_table("## D.5 Symbol", "## D.6 Zapf", ((0, 1, 2), (3, 4, 5)))
    d6, chars6 = d6_from_primary_pdf()
    compare("D.4 MacExpertEncoding", d4, rust_names("MAC_EXPERT"))
    compare("D.5 Symbol", d5, rust_names("SYMBOL"))
    compare("D.6 ZapfDingbats names (primary PDF)", d6, rust_names("ZAPF_DINGBATS"))
    impl_chars = dingbat_chars_from_code()
    # D.6's space row intentionally has a blank CHAR cell; compare nonblank source rows.
    compare("D.6 nonblank CHAR entries (primary PDF)", chars6, {i: ch for i, ch in impl_chars.items() if ch != ' '})
    print(f"D.6 blank CHAR rows={len(d6) - len(chars6)}; implementation explicit space entries={sum(1 for v in impl_chars.values() if v == ' ')}")
    if FAILURES:
        print(f"FAILED comparisons: {len(FAILURES)}")
        return 1
    print("all comparisons matched")
    return 0


if __name__ == "__main__":
    try:
        sys.exit(main())
    except Exception as exc:
        print(f"comparison could not complete: {exc}", file=sys.stderr)
        sys.exit(2)
