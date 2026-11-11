"""Audits Rust source-file structure, visibility gates, module registry purity, and production completeness."""

from __future__ import annotations

import argparse
import json
import re
from dataclasses import dataclass
from typing import List, Optional

from .common import Finding, ROOT, SRC
from .import_parser import is_test_path
from .rust_source import clean_rust_code
from .structure_checks import structural_findings

# Soft ceiling for AGENTS.md Section 4 file sizing (target: 200 to 500 lines; math/arch/ exempt).
COHESION_LINE_LIMIT = 500
ARCHITECTURE_ROOT = "src/int/logic/unsigned/math/arch/"
FORBIDDEN_VISIBILITY_RE = re.compile(r"\bpub\s*\(\s*(?:super\s*\)|in\b[^)]*\))")
PLACEHOLDER_RE = re.compile(r"\b(?:todo|unimplemented)\s*!\s*\(")
PRIVATE_LIB_USE_RE = re.compile(r"^\s*use\s+")


@dataclass(frozen=True)
class SizeReview:
    path: str
    lines: int


def line_number(text: str, offset: int) -> int:
    return text.count("\n", 0, offset) + 1


def run_structure_audit(argv: Optional[List[str]] = None) -> int:
    parser = argparse.ArgumentParser(description="Audit Rust source-file structure, visibility constraints, module registry purity, and production completeness.")
    parser.add_argument("--json", action="store_true", help="Print findings as JSON.")
    parser.add_argument("--deny-oversized", action="store_true", help="Treat the 500-line cohesion target as an error (architecture backends exempt).")
    args = parser.parse_args(argv)

    findings: List[Finding] = []
    oversized_files: List[SizeReview] = []

    roots = (SRC, ROOT / "benches", ROOT / "fuzz" / "src", ROOT / "fuzz" / "fuzz_targets", ROOT / "tools" / "tune", ROOT / "build_support")
    paths = {path for root in roots for path in root.rglob("*.rs")}
    for path in sorted(paths):
        is_benchmark = "benches" in path.relative_to(ROOT).parts
        if is_test_path(path) and not is_benchmark:
            continue
        rel = str(path.relative_to(ROOT)).replace("\\", "/")
        text = path.read_text(encoding="utf-8")
        cleaned = clean_rust_code(text, scrub_attributes=False)

        for match in FORBIDDEN_VISIBILITY_RE.finditer(cleaned):
            findings.append(Finding("forbidden_visibility", rel, line_number(cleaned, match.start()), match.group(0)))

        for match in PLACEHOLDER_RE.finditer(cleaned):
            findings.append(Finding("placeholder_in_production", rel, line_number(cleaned, match.start()), match.group(0)))

        findings.extend(structural_findings(text, rel, registry=path.name in {"mod.rs", "lib.rs"}, test_file=is_benchmark))
        if path.name == "lib.rs":
            for number, line in enumerate(cleaned.splitlines(), 1):
                if PRIVATE_LIB_USE_RE.match(line):
                    findings.append(Finding("private_import_in_library_facade", rel, number, line.strip()))

        line_count = len(text.splitlines())
        if line_count > COHESION_LINE_LIMIT and not rel.startswith(ARCHITECTURE_ROOT):
            oversized_files.append(SizeReview(path=rel, lines=line_count))
            if args.deny_oversized:
                findings.append(Finding("oversized_source_file", rel, 1, f"{line_count} lines exceeds the 500-line cohesion target"))

    if args.json:
        payload = {
            "findings": [f.__dict__ for f in findings],
            "oversized_files": [o.__dict__ for o in oversized_files],
        }
        print(json.dumps(payload, indent=2))
        return 0 if not findings else 1

    print(f"Structure findings: {len(findings)}")
    print(f"Files > 500 lines (architecture backends exempt): {len(oversized_files)}")

    if findings:
        print("\nFindings:")
        for f in findings:
            print(f"  {f.path}:{f.line} [{f.kind}] {f.detail}")
        return 1

    return 0
