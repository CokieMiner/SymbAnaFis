"""Inventories Rust lint allowance and expectation attributes across source trees.

Audits compliance with AGENTS.md Section 3 (Lint Allowances), which requires
every lint allowance to use `#[expect(...)]` or `#![expect(...)]` with an explicit,
descriptive `reason = "..."`. Aggregates entries by lint identifier and source path.
"""

from __future__ import annotations

import argparse
import json
import re
from collections import Counter, defaultdict
from dataclasses import dataclass
from pathlib import Path
from typing import Dict, List, Optional, Set, Tuple

from .rust_source import clean_rust_code, matching_delimiter, split_top_level

# ---------------------------------------------------------------------------
# Constants
# ---------------------------------------------------------------------------

ROOT = Path(__file__).resolve().parents[2]

SOURCE_DIRECTORIES = (
    "src",
    "benches",
    "examples",
    "tests",
    "fuzz/fuzz_targets",
    "tools/tune",
    "build_support",
)

TEST_PATH_PARTS = {"benches", "tests", "fuzz", "fuzz_targets"}

CFG_TEST_RE = re.compile(r"#\s*\[\s*cfg\s*\(\s*test\s*\)\s*\]", re.MULTILINE)
TEST_MODULE_RE = re.compile(r"\bmod\s+(?:tests|[A-Za-z_][A-Za-z0-9_]*_tests)\s*\{", re.MULTILINE)

# Matches inner and outer #[allow(...)] and #[expect(...)] attribute declarations.
ALLOW_EXPECT_RE = re.compile(r"(?P<prefix>#!|#)\s*\[\s*(?P<kind>allow|expect)\s*\(")


# ---------------------------------------------------------------------------
# Data
# ---------------------------------------------------------------------------


@dataclass(frozen=True)
class AllowEntry:
    kind: str  # Attribute descriptor format: "outer #[allow]", "outer #[expect]", or "inner #![allow]".
    path: str
    line: int
    lints: Tuple[str, ...]
    in_test: bool


# ---------------------------------------------------------------------------
# Discovery
# ---------------------------------------------------------------------------


def rust_source_paths(root: Path = ROOT, *, production_only: bool = False) -> List[Path]:
    paths: Set[Path] = set()
    directories = ("src",) if production_only else SOURCE_DIRECTORIES
    for relative in directories:
        directory = root / relative
        if directory.is_dir():
            paths.update(directory.rglob("*.rs"))
    build_script = root / "build.rs"
    if not production_only and build_script.is_file():
        paths.add(build_script)
    return sorted(paths)


def is_whole_file_test(rel_path: str) -> bool:
    parts = rel_path.replace("\\", "/").split("/")
    stem = Path(parts[-1]).stem
    return (
        any(part in TEST_PATH_PARTS for part in parts)
        or stem == "tests"
        or stem.endswith("_tests")
    )


# ---------------------------------------------------------------------------
# Test-range helpers
# ---------------------------------------------------------------------------


def skip_attributes_and_whitespace(text: str, offset: int) -> int:
    while offset < len(text):
        if text[offset].isspace():
            offset += 1
            continue
        if text.startswith("#![", offset):
            opening = offset + 2
        elif text.startswith("#[", offset):
            opening = offset + 1
        else:
            break
        closing = matching_delimiter(text, opening, "[", "]")
        if closing is None:
            break
        offset = closing + 1
    return offset


def find_item_end(text: str, offset: int) -> Optional[int]:
    depths = {"(": 0, "[": 0, "<": 0}
    closing = {")": "(", "]": "[", ">": "<"}
    in_initializer = False
    while offset < len(text):
        char = text[offset]
        if char in {"(", "["}:
            depths[char] += 1
        elif char == "<" and not in_initializer:
            depths[char] += 1
        elif char in closing and depths[closing[char]] > 0:
            depths[closing[char]] -= 1
        elif char == "=" and not any(depths.values()):
            in_initializer = True
        elif char in "{;" and not any(depths.values()):
            if char == ";":
                return offset + 1
            body_end = matching_delimiter(text, offset, "{", "}")
            return len(text) if body_end is None else body_end + 1
        offset += 1
    return None


def find_test_ranges(text: str) -> List[Tuple[int, int]]:
    cleaned = clean_rust_code(text, scrub_attributes=False)
    ranges: List[Tuple[int, int]] = []

    for match in TEST_MODULE_RE.finditer(cleaned):
        opening = cleaned.find("{", match.start(), match.end())
        closing = matching_delimiter(cleaned, opening, "{", "}")
        if closing is not None:
            ranges.append((match.start(), closing + 1))

    for match in CFG_TEST_RE.finditer(cleaned):
        item_start = skip_attributes_and_whitespace(cleaned, match.end())
        item_end = find_item_end(cleaned, item_start)
        if item_end is not None:
            ranges.append((match.start(), item_end))

    if not ranges:
        return []
    ranges.sort()
    merged = [ranges[0]]
    for start, end in ranges[1:]:
        prev_start, prev_end = merged[-1]
        if start <= prev_end:
            merged[-1] = (prev_start, max(prev_end, end))
        else:
            merged.append((start, end))
    return merged


# ---------------------------------------------------------------------------
# Extraction
# ---------------------------------------------------------------------------


def extract_allows(
    text: str, rel_path: str, test_ranges: List[Tuple[int, int]], whole_file_test: bool
) -> List[AllowEntry]:
    """Extracts lint allowances declared via #[allow(...)] attributes."""
    return extract_allows_expects(text, rel_path, test_ranges, whole_file_test, kinds=("allow",))


def extract_allows_expects(
    text: str,
    rel_path: str,
    test_ranges: List[Tuple[int, int]],
    whole_file_test: bool,
    *,
    kinds: Tuple[str, ...] = ("allow", "expect"),
) -> List[AllowEntry]:
    cleaned = clean_rust_code(text, scrub_attributes=False)
    entries: List[AllowEntry] = []
    for match in ALLOW_EXPECT_RE.finditer(cleaned):
        attr_kind = match.group("kind")
        if attr_kind not in kinds:
            continue
        opening = cleaned.rfind("(", match.start(), match.end())
        closing = matching_delimiter(cleaned, opening, "(", ")")
        if closing is None:
            continue
        parts = split_top_level(cleaned[opening + 1 : closing])
        lints = tuple(
            part.strip()
            for part in parts
            if part.strip() and part.split("=", maxsplit=1)[0].strip() != "reason"
        )
        if not lints:
            continue
        start = match.start()
        prefix = match.group("prefix")
        if prefix == "#!":
            kind = f"inner #![{attr_kind}]"
        else:
            kind = f"outer #[{attr_kind}]"
        entries.append(
            AllowEntry(
                kind=kind,
                path=rel_path,
                line=cleaned.count("\n", 0, start) + 1,
                lints=lints,
                in_test=(
                    whole_file_test
                    or any(range_start <= start < range_end for range_start, range_end in test_ranges)
                ),
            )
        )
    return entries


# ---------------------------------------------------------------------------
# Filtering
# ---------------------------------------------------------------------------


def select_lints(
    entries: List[AllowEntry],
    *,
    lint: Optional[str],
    mode: str,
    production_only: bool,
    test_only: bool,
) -> List[AllowEntry]:
    wanted = None
    if lint is not None:
        wanted = {lint}
        if "::" not in lint:
            wanted.add(f"clippy::{lint}")

    selected: List[AllowEntry] = []
    for entry in entries:
        if production_only and entry.in_test:
            continue
        if test_only and not entry.in_test:
            continue
        lints = entry.lints
        if mode == "clippy":
            lints = tuple(name for name in lints if name.startswith("clippy::"))
        elif mode == "non_clippy":
            lints = tuple(name for name in lints if not name.startswith("clippy::"))
        if wanted is not None:
            lints = tuple(name for name in lints if name in wanted)
        if lints:
            selected.append(AllowEntry(entry.kind, entry.path, entry.line, lints, entry.in_test))
    return selected


# ---------------------------------------------------------------------------
# Summaries
# ---------------------------------------------------------------------------


@dataclass
class Summary:
    entries: List[AllowEntry]
    prod_counts: Counter[str]
    test_counts: Counter[str]
    prod_files: Dict[str, Set[str]]
    test_files: Dict[str, Set[str]]
    kind_counts: Counter[str]
    all_lints: List[str]
    total_allow: int
    total_expect: int


def summarize(entries: List[AllowEntry]) -> Summary:
    prod_counts: Counter[str] = Counter()
    test_counts: Counter[str] = Counter()
    prod_files: Dict[str, Set[str]] = defaultdict(set)
    test_files: Dict[str, Set[str]] = defaultdict(set)
    kind_counts: Counter[str] = Counter()

    for entry in entries:
        # Normalizes attribute descriptor to base attribute identifier ("allow" or "expect").
        attr = "expect" if "expect" in entry.kind else "allow"
        kind_counts[attr] += 1
        kind_counts[entry.kind] += 1
        counts = test_counts if entry.in_test else prod_counts
        files = test_files if entry.in_test else prod_files
        for lint in entry.lints:
            counts[lint] += 1
            files[lint].add(entry.path)

    all_lints = sorted(set(prod_counts) | set(test_counts))
    total_allow = sum(1 for e in entries if "allow" in e.kind)
    total_expect = sum(1 for e in entries if "expect" in e.kind)
    return Summary(
        entries=entries,
        total_allow=total_allow,
        total_expect=total_expect,
        prod_counts=prod_counts,
        test_counts=test_counts,
        prod_files=prod_files,
        test_files=test_files,
        kind_counts=kind_counts,
        all_lints=all_lints,
    )


# ---------------------------------------------------------------------------
# Rendering
# ---------------------------------------------------------------------------


def render_text(summary: Summary, kinds: Tuple[str, ...], mode: str) -> str:
    lines: List[str] = []
    lines.append(f"Inventory ({mode}) — kinds: {', '.join(kinds)}")
    lines.append(f"Entries: {len(summary.entries)}  (allow: {summary.total_allow}, expect: {summary.total_expect} — AGENTS.md Section 3 mandates #[expect] with reason)")
    lines.append(f"Prod lints: {sum(summary.prod_counts.values())}  Test lints: {sum(summary.test_counts.values())}")
    lines.append("")

    if not summary.all_lints:
        lines.append("No matching entries.")
        return "\n".join(lines)

    # Formats hierarchical output grouped by lint identifier and associated file paths.
    for lint in summary.all_lints:
        prod_n = summary.prod_counts.get(lint, 0)
        test_n = summary.test_counts.get(lint, 0)
        files = sorted(summary.prod_files.get(lint, set()) | summary.test_files.get(lint, set()))
        # Renders production and test occurrence counts inline.
        header = f"{lint}  (prod:{prod_n} test:{test_n} files:{len(files)})"
        lines.append(header)
        for path in files:
            # Annotates paths whose lint occurrences reside exclusively within test targets.
            only_test = path in summary.test_files.get(lint, set()) and path not in summary.prod_files.get(lint, set())
            suffix = "  [test]" if only_test else ""
            lines.append(f"  | {path}{suffix}")
        lines.append("")

    # Renders aggregate counts partitioned by attribute descriptor kind.
    lines.append("-" * 80)
    lines.append(f"Kind breakdown: {dict(summary.kind_counts)}")
    return "\n".join(lines)


def render_json(summary: Summary, kinds: Tuple[str, ...], mode: str) -> str:
    by_lint = {
        lint: {
            "prod_count": summary.prod_counts.get(lint, 0),
            "test_count": summary.test_counts.get(lint, 0),
            "prod_files": sorted(summary.prod_files.get(lint, set())),
            "test_files": sorted(summary.test_files.get(lint, set())),
        }
        for lint in summary.all_lints
    }
    payload = {
        "mode": mode,
        "kinds": list(kinds),
        "total_entries": len(summary.entries),
        "total_allows": summary.total_allow,
        "total_expects": summary.total_expect,
        "prod_total": sum(summary.prod_counts.values()),
        "test_total": sum(summary.test_counts.values()),
        "kind_counts": dict(summary.kind_counts),
        "by_lint": by_lint,
    }
    return json.dumps(payload, indent=2)


# ---------------------------------------------------------------------------
# CLI
# ---------------------------------------------------------------------------


def build_parser() -> argparse.ArgumentParser:
    parser = argparse.ArgumentParser(description="Inventory Rust lint allowances (#[allow]) and expectations (#[expect]).")
    parser.add_argument("--json", action="store_true", help="Output JSON")
    parser.add_argument("--lint", help="Filter to a single lint (e.g. clippy::as_conversions)")
    loc = parser.add_mutually_exclusive_group()
    loc.add_argument("--prod-only", action="store_true", help="Only production code (src/)")
    loc.add_argument("--test-only", action="store_true", help="Only test code")
    lint_mode = parser.add_mutually_exclusive_group()
    lint_mode.add_argument("--non-clippy", action="store_true", help="Only non-Clippy lints")
    lint_mode.add_argument("--all-lints", action="store_true", help="Both Clippy and non-Clippy lints")
    kind_grp = parser.add_mutually_exclusive_group()
    kind_grp.add_argument("--allow-only", action="store_true", help="Only #[allow]")
    kind_grp.add_argument("--expect-only", action="store_true", help="Only #[expect]")
    return parser


def run_check_allows(argv: Optional[List[str]] = None) -> int:
    parser = build_parser()
    args = parser.parse_args(argv)

    kinds: Tuple[str, ...]
    if args.allow_only:
        kinds = ("allow",)
    elif args.expect_only:
        kinds = ("expect",)
    else:
        kinds = ("allow", "expect")

    entries: List[AllowEntry] = []
    for path in rust_source_paths(production_only=args.prod_only):
        text = path.read_text(encoding="utf-8")
        rel = str(path.relative_to(ROOT))
        entries.extend(
            extract_allows_expects(text, rel, find_test_ranges(text), is_whole_file_test(rel), kinds=kinds)
        )

    mode = "all" if args.all_lints else "non_clippy" if args.non_clippy else "clippy"
    entries = select_lints(entries, lint=args.lint, mode=mode, production_only=args.prod_only, test_only=args.test_only)
    summary = summarize(entries)

    if args.json:
        print(render_json(summary, kinds, mode))
    else:
        print(render_text(summary, kinds, mode))
    return 0
