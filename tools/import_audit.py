#!/usr/bin/env python3
"""Audit Rust imports and module-boundary usage.

Checks:
- inline qualified paths used in code bodies outside `use` statements
- deep relative imports such as `use super::super::...`
- cross-boundary imports into `logic/`
- surface bypasses that jump past a boundary's top-level `logic` re-export
- emits a Graphviz DOT graph with staircase-boundary clusters

Usage:
    python3 tools/import_audit.py
    python3 tools/import_audit.py --json
    python3 tools/import_audit.py --dot tools/import_graph.dot
"""

from __future__ import annotations

import argparse
import json
import keyword
import re
from collections import Counter
from dataclasses import asdict, dataclass
from pathlib import Path


ROOT = Path(__file__).resolve().parents[1]
SRC = ROOT / "src"
RUST_KEYWORDS = {
    "Self",
    "async",
    "await",
    "break",
    "const",
    "continue",
    "crate",
    "dyn",
    "else",
    "enum",
    "extern",
    "false",
    "fn",
    "for",
    "if",
    "impl",
    "in",
    "let",
    "loop",
    "match",
    "mod",
    "move",
    "mut",
    "pub",
    "ref",
    "return",
    "self",
    "static",
    "struct",
    "super",
    "trait",
    "true",
    "type",
    "unsafe",
    "use",
    "where",
    "while",
}
USE_RE = re.compile(r"^\s*(?:pub(?:\([^)]+\))?\s+)?use\s+(.+?);$")
DEEP_RELATIVE_RE = re.compile(r"^\s*(?:pub(?:\([^)]+\))?\s+)?use\s+super::super(?:::super)*::")
INLINE_MOD_RE = re.compile(r"^\s*(?:pub(?:\([^)]+\))?\s+)?mod\s+([A-Za-z_][A-Za-z0-9_]*)\s*\{")
MACRO_RULES_RE = re.compile(r"^\s*(?:#\[.*?\]\s*)*macro_rules!\s+[A-Za-z_][A-Za-z0-9_]*\s*\{")
QUALIFIED_PATH_RE = re.compile(r"\b([A-Za-z_][A-Za-z0-9_]*::[A-Za-z0-9_<>{}, ]+(?:::[A-Za-z0-9_<>{}, ]+)*)")
TEST_PATH_PARTS = {"tests", "benches"}
WHITELISTED_QUALIFIED_PREFIXES = {
    "Arc::",
    "Box::",
    "clippy::",
    "Cow::",
    "f32::",
    "f64::",
    "f64x4::",
    "i32::",
    "i64::",
    "u32::",
    "u64::",
    "Value::",
    "usize::",
    "HashMap::",
    "HashSet::",
    "LazyLock::",
    "None::",
    "Option::",
    "Result::",
    "Self::",
    "String::",
    "Vec::",
}


@dataclass
class Finding:
    kind: str
    path: str
    line: int
    text: str


@dataclass
class ImportEdge:
    source: str
    target: str
    raw: str
    path: str
    line: int
    internal: bool


def is_comment_or_doc(line: str) -> bool:
    stripped = line.lstrip()
    return (
        stripped.startswith("//")
        or stripped.startswith("///")
        or stripped.startswith("//!")
        or stripped.startswith("/*")
        or stripped.startswith("*")
    )


def is_non_code_noise(line: str) -> bool:
    stripped = line.lstrip()
    return (
        not stripped
        or stripped.startswith("#[")
        or stripped.startswith("pub(in ")
        or stripped.startswith("mod ")
        or stripped.startswith("pub mod ")
        or stripped.startswith("macro_rules!")
        or is_comment_or_doc(line)
    )


def module_name(path: Path) -> str:
    rel = path.relative_to(ROOT).with_suffix("")
    parts = list(rel.parts)
    if parts[-1] == "mod":
        parts.pop()
    return "::".join(parts)


def staircase_boundaries() -> set[str]:
    boundaries: set[str] = set()
    for directory in SRC.rglob("*"):
        if not directory.is_dir():
            continue
        if (directory / "api.rs").exists() and (directory / "logic").is_dir():
            rel = directory.relative_to(ROOT)
            boundaries.add("::".join(rel.parts))
    return boundaries


def is_test_path(path: Path) -> bool:
    if any(part in TEST_PATH_PARTS for part in path.parts):
        return True
    stem = path.stem
    return stem == "tests" or stem.endswith("_tests")


def is_test_module_name(name: str) -> bool:
    return name == "tests" or name.endswith("_tests")


def split_top_level(text: str, delimiter: str = ",") -> list[str]:
    items: list[str] = []
    current: list[str] = []
    depth = 0
    for char in text:
        if char == "{":
            depth += 1
        elif char == "}":
            depth -= 1
        if char == delimiter and depth == 0:
            item = "".join(current).strip()
            if item:
                items.append(item)
            current = []
            continue
        current.append(char)
    tail = "".join(current).strip()
    if tail:
        items.append(tail)
    return items


def split_alias(path: str) -> str:
    parts = re.split(r"\s+as\s+", path, maxsplit=1)
    return parts[0].strip()


def expand_use_tree(tree: str) -> list[str]:
    tree = tree.strip()
    if tree.startswith("{") and tree.endswith("}"):
        paths: list[str] = []
        for item in split_top_level(tree[1:-1]):
            paths.extend(expand_use_tree(item))
        return paths

    depth = 0
    group_start = -1
    prefix_end = -1
    for idx, char in enumerate(tree):
        if char == "{":
            if depth == 0:
                group_start = idx
                prefix_end = idx - 2
            depth += 1
        elif char == "}":
            depth -= 1
            if depth == 0 and group_start != -1:
                prefix = tree[:prefix_end].strip()
                group = tree[group_start + 1 : idx]
                suffix = tree[idx + 1 :].strip()
                if suffix.startswith("::"):
                    suffix = suffix[2:]
                paths: list[str] = []
                for item in split_top_level(group):
                    expanded = expand_use_tree(item)
                    for part in expanded:
                        combined = prefix if part == "self" else f"{prefix}::{part}"
                        if suffix:
                            combined = f"{combined}::{suffix}"
                        paths.append(combined)
                return paths

    return [split_alias(tree)]


def resolve_relative_path(source: str, target: str) -> str:
    target = split_alias(target)
    if target == "self":
        return source
    if target == "super":
        return "::".join(source.split("::")[:-1])
    if target.startswith("crate::"):
        return f"src::{target[7:]}"
    if target.startswith("self::"):
        return f"{source}::{target[6:]}"
    if target.startswith("super::"):
        parts = source.split("::")
        rest = target
        while rest.startswith("super::"):
            rest = rest[7:]
            if len(parts) > 1:
                parts.pop()
        if rest:
            parts.extend(rest.split("::"))
        return "::".join(parts)
    return target


def boundary_for_module(module: str, boundaries: set[str]) -> str | None:
    candidates = [
        boundary
        for boundary in boundaries
        if module == boundary or module.startswith(f"{boundary}::")
    ]
    if not candidates:
        return None
    return max(candidates, key=lambda item: item.count("::"))


def is_internal_target(target: str) -> bool:
    return target.startswith("src::")


def qualified_path_matches(line: str) -> list[str]:
    matches: list[str] = []
    for match in QUALIFIED_PATH_RE.findall(line):
        candidate = match.strip()
        if candidate.startswith("$crate::"):
            continue
        # Skip double-underscore items — Rust convention for macro implementation helpers
        if any(seg.startswith("__") for seg in candidate.split("::")):
            continue
        if any(candidate.startswith(prefix) for prefix in WHITELISTED_QUALIFIED_PREFIXES):
            continue
        if re.match(r"^[A-Za-z_][A-Za-z0-9_]*::<", candidate):
            continue
        if "::{" in candidate:
            continue
        head = candidate.split("::", 1)[0]
        if head in RUST_KEYWORDS or keyword.iskeyword(head):
            matches.append(candidate)
            continue
        if head and head[0].islower():
            matches.append(candidate)
            continue
        if head in {"Arc", "Box", "Vec", "String", "Option", "Result", "HashMap", "HashSet"}:
            matches.append(candidate)
    return matches


def collect_findings() -> tuple[list[Finding], list[ImportEdge], set[str]]:
    boundaries = staircase_boundaries()
    finding_keys: set[tuple[str, str, int, str]] = set()
    findings: list[Finding] = []
    edges: list[ImportEdge] = []

    for path in sorted(SRC.rglob("*.rs")):
        if is_test_path(path):
            continue
        file_mod = module_name(path)
        base_parts = file_mod.split("::")
        nested_modules: list[tuple[str, int]] = []
        brace_depth = 0
        macro_body_depth: int | None = None  # brace_depth at which current macro_rules! opened
        for lineno, line in enumerate(path.read_text(encoding="utf-8").splitlines(), start=1):
            inside_macro = macro_body_depth is not None and brace_depth > macro_body_depth
            source_parts = base_parts + [name for name, _depth in nested_modules]
            source_mod = "::".join(source_parts)
            source_boundary = boundary_for_module(source_mod, boundaries)
            inside_test_module = any(is_test_module_name(name) for name, _depth in nested_modules)
            use_match = USE_RE.match(line)
            if use_match and not inside_test_module:
                raw_use = use_match.group(1)
                for raw_target in expand_use_tree(raw_use):
                    normalized = resolve_relative_path(source_mod, raw_target)
                    edges.append(
                        ImportEdge(
                            source=source_mod,
                            target=normalized,
                            raw=raw_target,
                            path=str(path.relative_to(ROOT)),
                            line=lineno,
                            internal=is_internal_target(normalized),
                        )
                    )

                    if raw_target.startswith("super::super"):
                        finding_keys.add(
                            (
                                "deep_relative_import",
                                str(path.relative_to(ROOT)),
                                lineno,
                                raw_target,
                            )
                        )

                    # Check for self-referential crate imports (using crate:: for items in same boundary)
                    if raw_target.startswith("crate::") and source_boundary is not None:
                        target_boundary = boundary_for_module(normalized, boundaries)
                        if target_boundary == source_boundary:
                            finding_keys.add(
                                (
                                    "self_referential_crate_import",
                                    str(path.relative_to(ROOT)),
                                    lineno,
                                    f"{raw_target} (should be relative)",
                                )
                            )

                    if "::logic::" in normalized and normalized.startswith("src::"):
                        logic_boundary = normalized.split("::logic::", 1)[0]
                        if source_boundary != logic_boundary:
                            finding_keys.add(
                                (
                                    "cross_boundary_logic_import",
                                    str(path.relative_to(ROOT)),
                                    lineno,
                                    f"{source_mod} -> {normalized}",
                                )
                            )
                        elif not (
                            source_mod == logic_boundary
                            or source_mod.endswith("::api")
                            or source_mod.startswith(f"{logic_boundary}::logic")
                        ):
                            after_logic = normalized.split("::logic::", 1)[1]
                            if "::" in after_logic:
                                finding_keys.add(
                                    (
                                        "deep_logic_surface_bypass",
                                        str(path.relative_to(ROOT)),
                                        lineno,
                                        f"{source_mod} -> {normalized}",
                                    )
                                )
                            else:
                                finding_keys.add(
                                    (
                                        "logic_surface_import",
                                        str(path.relative_to(ROOT)),
                                        lineno,
                                        f"{source_mod} -> {normalized}",
                                    )
                                )
                line_for_braces = re.sub(r"//.*$", "", line)
                brace_depth += line_for_braces.count("{") - line_for_braces.count("}")
                while nested_modules and brace_depth < nested_modules[-1][1]:
                    nested_modules.pop()
                continue

            if is_non_code_noise(line):
                mod_match = INLINE_MOD_RE.match(line)
                if mod_match:
                    line_for_braces = re.sub(r"//.*$", "", line)
                    next_depth = brace_depth + line_for_braces.count("{") - line_for_braces.count("}")
                    nested_modules.append((mod_match.group(1), next_depth))
                    brace_depth = next_depth
                    continue
                # Detect macro_rules! entry here (before the continue below)
                if macro_body_depth is None and MACRO_RULES_RE.match(line):
                    macro_body_depth = brace_depth
                line_for_braces = re.sub(r"//.*$", "", line)
                brace_depth += line_for_braces.count("{") - line_for_braces.count("}")
                if macro_body_depth is not None and brace_depth <= macro_body_depth:
                    macro_body_depth = None
                while nested_modules and brace_depth < nested_modules[-1][1]:
                    nested_modules.pop()
                continue

            if not inside_test_module and not inside_macro:
                for match in qualified_path_matches(line):
                    finding_keys.add(
                        (
                            "inline_qualified_path",
                            str(path.relative_to(ROOT)),
                            lineno,
                            match,
                        )
                    )

            mod_match = INLINE_MOD_RE.match(line)
            line_for_braces = re.sub(r"//.*$", "", line)
            next_depth = brace_depth + line_for_braces.count("{") - line_for_braces.count("}")
            if mod_match:
                nested_modules.append((mod_match.group(1), next_depth))
            # Track macro_rules! body entry/exit
            if macro_body_depth is None and MACRO_RULES_RE.match(line):
                macro_body_depth = brace_depth
            brace_depth = next_depth
            if macro_body_depth is not None and brace_depth <= macro_body_depth:
                macro_body_depth = None
            while nested_modules and brace_depth < nested_modules[-1][1]:
                nested_modules.pop()

    findings = [
        Finding(kind=kind, path=path, line=line, text=text)
        for kind, path, line, text in sorted(finding_keys)
    ]
    return findings, edges, boundaries


def write_dot(edges: list[ImportEdge], boundaries: set[str], dot_path: Path) -> None:
    dot_path.parent.mkdir(parents=True, exist_ok=True)
    nodes = {edge.source for edge in edges}
    nodes.update(edge.target for edge in edges if edge.internal)

    boundary_map: dict[str, list[str]] = {boundary: [] for boundary in sorted(boundaries)}
    loose_nodes: list[str] = []
    for node in sorted(nodes):
        boundary = boundary_for_module(node, boundaries)
        if boundary:
            boundary_map.setdefault(boundary, []).append(node)
        else:
            loose_nodes.append(node)

    with dot_path.open("w", encoding="utf-8") as fh:
        fh.write("digraph imports {\n")
        fh.write('  rankdir="LR";\n')
        fh.write('  graph [fontname="monospace"];\n')
        fh.write('  node [shape="box", fontname="monospace"];\n')
        fh.write('  edge [fontname="monospace"];\n')

        cluster_index = 0
        for boundary, boundary_nodes in boundary_map.items():
            if not boundary_nodes:
                continue
            fh.write(f"  subgraph cluster_{cluster_index} {{\n")
            fh.write(f'    label="{boundary}";\n')
            fh.write('    color="lightgrey";\n')
            for node in sorted(boundary_nodes):
                fill = "white"
                if node == boundary:
                    fill = "lightblue"
                elif "::logic" in node:
                    fill = "lightyellow"
                elif node.endswith("::api"):
                    fill = "honeydew"
                fh.write(f'    "{node}" [style="filled", fillcolor="{fill}"];\n')
            fh.write("  }\n")
            cluster_index += 1

        for node in loose_nodes:
            fh.write(f'  "{node}";\n')

        for edge in edges:
            target = edge.target
            if not edge.internal:
                if "::" in target:
                    target = target.split("::", 1)[0]
                fh.write(f'  "{target}" [shape="ellipse", style="dashed"];\n')
            safe_source = edge.source.replace('"', '\\"')
            safe_target = target.replace('"', '\\"')
            fh.write(f'  "{safe_source}" -> "{safe_target}";\n')

        fh.write("}\n")


def structure_findings(boundaries: set[str]) -> list[Finding]:
    findings: list[Finding] = []
    for boundary in sorted(boundaries):
        rel = ROOT.joinpath(*boundary.split("::"))
        if not (rel / "mod.rs").exists():
            findings.append(
                Finding(
                    kind="missing_boundary_mod",
                    path=str(rel.relative_to(ROOT)),
                    line=1,
                    text=f"{boundary} has api.rs and logic/ but no mod.rs",
                )
            )
        if not (rel / "logic" / "mod.rs").exists():
            findings.append(
                Finding(
                    kind="missing_logic_mod",
                    path=str((rel / 'logic').relative_to(ROOT)),
                    line=1,
                    text=f"{boundary}::logic is missing mod.rs",
                )
            )
    return findings


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("--json", action="store_true", help="Emit JSON findings")
    parser.add_argument(
        "--dot",
        default=str(ROOT / "tools" / "import_graph.dot"),
        help="Path to write Graphviz DOT output",
    )
    parser.add_argument(
        "--limit",
        type=int,
        default=200,
        help="Maximum number of findings to print in text mode",
    )
    args = parser.parse_args()

    findings, edges, boundaries = collect_findings()
    findings.extend(structure_findings(boundaries))
    findings.sort(key=lambda item: (item.kind, item.path, item.line, item.text))
    write_dot(edges, boundaries, Path(args.dot))
    counts = Counter(finding.kind for finding in findings)

    if args.json:
        print(
            json.dumps(
                {
                    "root": str(ROOT),
                    "dot": str(args.dot),
                    "boundary_count": len(boundaries),
                    "boundaries": sorted(boundaries),
                    "counts": dict(counts),
                    "findings": [asdict(finding) for finding in findings],
                },
                indent=2,
            )
        )
        return 0

    print(f"DOT graph written to: {args.dot}")
    print(f"Boundaries detected: {len(boundaries)}")
    print(f"Findings: {len(findings)}")
    for kind, count in sorted(counts.items()):
        print(f"  {kind}: {count}")
    for finding in findings[: args.limit]:
        print(f"{finding.kind}: {finding.path}:{finding.line}: {finding.text}")
    if len(findings) > args.limit:
        print(f"... truncated {len(findings) - args.limit} more findings")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
