#!/usr/bin/env python3
"""Reconcile ledger-symbol-precheck's file-only references with the current ledger."""

from __future__ import annotations

import json
import tomllib
from pathlib import Path

ROOT = next(p for p in Path(__file__).resolve().parents if (p / "Cargo.toml").is_file())
PRECHECK = ROOT / "doc/reviews/2038-specification-audit/test-reference-precheck.json"
LEDGER = ROOT / "doc/conformance/ledger.toml"


def main() -> None:
    precheck = json.loads(PRECHECK.read_text())
    ledger = tomllib.loads(LEDGER.read_text())
    output = []
    for ref in precheck["exceptions"]:
        if ref.get("check") != "file-only":
            continue
        candidates = [
            row
            for row in ledger["clause"]
            if row.get("clause") == ref["clause"]
            and ref["reference"] in row.get("test", [])
        ]
        named = sorted(
            {
                test
                for row in candidates
                for test in row.get("test", [])
                if "::" in test
            }
        )
        output.append(
            {
                "clause": ref["clause"],
                "status_from_inventory": ref["status"],
                "reference": ref["reference"],
                "path_exists": (ROOT / ref["reference"]).is_file(),
                "matching_ledger_rows": len(candidates),
                "same_row_named_tests": named,
                "semantics": "inventory_metadata_only",
            }
        )
    print(
        json.dumps(
            {
                "root": str(ROOT),
                "precheck_counts": precheck["counts"],
                "file_only_reference_count": len(output),
                "ledger_matched_reference_count": sum(
                    1 for row in output if row["matching_ledger_rows"]
                ),
                "all_references_have_named_sibling_tests": all(
                    row["same_row_named_tests"] for row in output
                ),
                "references": output,
            },
            indent=2,
        )
    )


if __name__ == "__main__":
    main()
