"""Rust use-statement, token, and module hierarchy parser."""

from __future__ import annotations

import keyword
import re
import tomllib
from dataclasses import dataclass
from pathlib import Path
from typing import List, Optional, Set, Tuple

from .common import CONFIG, ROOT, SRC
from .rust_source import split_top_level

RUST_KEYWORDS = {
    "Self", "async", "await", "break", "const", "continue", "crate", "dyn",
    "else", "enum", "extern", "false", "fn", "for", "if", "impl", "in", "let",
    "loop", "match", "mod", "move", "mut", "pub", "ref", "return", "self",
    "static", "struct", "super", "trait", "true", "type", "unsafe", "use",
    "where", "while",
}

INLINE_MOD_RE = re.compile(r"^\s*(?:pub(?:\([^)]+\))?\s+)?mod\s+([A-Za-z_][A-Za-z0-9_]*)\s*\{")
MODULE_DECL_RE = re.compile(r"^\s*(?:pub(?:\([^)]+\))?\s+)?mod\s+([A-Za-z_][A-Za-z0-9_]*)\s*(?:\{|;)")
QUALIFIED_PATH_RE = re.compile(r"((?:\$crate|[A-Za-z_][A-Za-z0-9_]*)(?:::[A-Za-z_][A-Za-z0-9_]*)+)")
MACRO_INVOCATION_RE = re.compile(r"\b([A-Za-z_][A-Za-z0-9_]*)\s*!\s*[(\{\[]")
CODE_ITEM_RE = re.compile(r"^\s*(?:pub(?:\([^)]+\))?\s+)?(?:const|static|fn|struct|enum|trait|impl|type)\b")
TEST_PATH_PARTS = {"tests", "benches"}
IMPORT_GROUPS = ("standard", "alloc", "external", "crate", "super", "local")


@dataclass(frozen=True)
class UseStatement:
    line_number: int
    raw_text: str
    cleaned_text: str
    is_pub: bool
    imported_paths: Tuple[str, ...]
    bound_names: Tuple[str, ...]
    aliases: Tuple[Tuple[str, str], ...] = ()


def cargo_dependency_roots() -> Set[str]:
    manifest = ROOT / "Cargo.toml"
    if not manifest.exists():
        return set()
    data = tomllib.loads(manifest.read_text(encoding="utf-8"))
    deps: Set[str] = set()
    for table in ("dependencies", "dev-dependencies", "build-dependencies"):
        deps.update(data.get(table, {}).keys())
    for target in data.get("target", {}).values():
        for table in ("dependencies", "dev-dependencies", "build-dependencies"):
            deps.update(target.get(table, {}).keys())
    return {name.replace("-", "_") for name in deps}


EXTERNAL_CRATE_ROOTS = {"core", "std", "alloc"} | cargo_dependency_roots()


def is_test_path(path: Path) -> bool:
    if any(part in TEST_PATH_PARTS for part in path.parts):
        return True
    stem = path.stem
    return stem == "tests" or stem.endswith("_tests")


def is_test_module_name(name: str) -> bool:
    return name == "tests" or name.endswith("_tests")


def split_alias(path: str) -> str:
    parts = re.split(r"\s+as\s+", path, maxsplit=1)
    return parts[0].strip()


def parse_alias(path: str) -> Tuple[str, Optional[str]]:
    parts = re.split(r"\s+as\s+", path, maxsplit=1)
    target = parts[0].strip()
    alias = parts[1].strip() if len(parts) == 2 else None
    return target, alias


def expand_use_tree(tree: str) -> List[Tuple[str, Optional[str]]]:
    tree = tree.strip()
    if tree.startswith("{") and tree.endswith("}"):
        paths: List[Tuple[str, Optional[str]]] = []
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
                paths: List[Tuple[str, Optional[str]]] = []
                for item in split_top_level(group):
                    expanded = expand_use_tree(item)
                    for part, alias in expanded:
                        combined = prefix if part == "self" else f"{prefix}::{part}"
                        if suffix:
                            combined = f"{combined}::{suffix}"
                        paths.append((combined, alias))
                return paths

    target, alias = parse_alias(tree)
    return [(target, alias)]


def import_target_group(raw_target: str) -> int:
    target = raw_target.lstrip(":").strip()
    root = target.split("::", 1)[0].split("{", 1)[0].strip()
    if root in {"core", "std"}: return 0
    if root == "alloc": return 1
    if root in EXTERNAL_CRATE_ROOTS: return 2
    if root == "crate": return 3
    if root == "super": return 4
    return 5


def import_groups(raw_use: str) -> Set[int]:
    return {import_target_group(target) for target, _alias in expand_use_tree(raw_use)}


def import_sort_key(source: str) -> Tuple[Tuple[int, str | int], ...]:
    parts = [part for part in re.split(r"(\d+)", source) if part]
    # rustfmt ranks `self`, `super`, then `crate` roots ahead of plain paths
    # when reordering imports, so the audit must use the same root ranking.
    root_rank = {"self": -3, "super": -2, "crate": -1}.get(parts[0].casefold(), 0)
    natural = tuple(
        (1, int(part)) if part.isdigit() else (0, part.casefold()) for part in parts
    )
    return ((0, root_rank),) + natural


def resolve_relative_path(source: str, target: str) -> str:
    target = split_alias(target)
    if target == "self": return source
    if target == "super": return "::".join(source.split("::")[:-1])
    if target.startswith("crate::"): return f"src::{target[7:]}"
    if target.startswith("self::"): return f"{source}::{target[6:]}"
    if target.startswith("super::"):
        parts = source.split("::")
        rest = target
        while rest.startswith("super::"):
            rest = rest[7:]
            if len(parts) > 1: parts.pop()
        if rest: parts.extend(rest.split("::"))
        return "::".join(parts)
    root = target.lstrip(":").split("::", maxsplit=1)[0]
    if root not in EXTERNAL_CRATE_ROOTS:
        return f"{source}::{target}"
    return target


def boundary_for_module(module: str, boundaries: Set[str]) -> Optional[str]:
    candidates = [b for b in boundaries if module == b or module.startswith(f"{b}::")]
    return max(candidates, key=lambda item: item.count("::")) if candidates else None


def is_internal_target(target: str) -> bool:
    return target.startswith("src::")


def is_shallow_crate_import(raw_target: str) -> bool:
    if not raw_target.startswith("crate::"): return False
    return "::" not in raw_target[7:]


def is_whitelisted_crate_root_import(raw_target: str) -> bool:
    return raw_target in CONFIG["whitelisted_crate_root_imports"]


def top_level_subsystem(module: str) -> Optional[str]:
    parts = module.split("::")
    if len(parts) >= 2 and parts[0] == "src":
        return parts[1]
    return None


def is_architecture_wiring_file(path: str) -> bool:
    architecture_root = "src/int/logic/unsigned/math/arch/"
    return path.startswith(architecture_root) and (
        Path(path).name in CONFIG["architecture_wiring_basenames"]
        or path in CONFIG["architecture_wiring_paths"]
    )


def project_import_violation(
    raw_target: str,
    *,
    source_path: str,
    source_mod: str,
    plumbing_file: bool,
    local_modules: Optional[Set[str]] = None,
) -> Optional[str]:
    target = split_alias(raw_target)
    parts = [part for part in target.split("::") if part]
    if not parts or parts[0] in EXTERNAL_CRATE_ROOTS:
        return None

    if parts[0] == "crate":
        if len(parts) != 3:
            return "deep_project_import"
        source_subsystem = top_level_subsystem(source_mod)
        target_subsystem = parts[1]
        if source_subsystem == target_subsystem:
            return "non_parent_plumbing_import" if plumbing_file else "non_parent_implementation_import"
        return None

    if is_architecture_wiring_file(source_path):
        return None

    if len(parts) == 2 and parts[0] in (local_modules or set()):
        return None

    if plumbing_file:
        if parts[0] == "super":
            return None if len(parts) == 2 else "non_parent_plumbing_import"
        if parts[0] == "self":
            return None if len(parts) <= 3 else "deep_project_import"
        return None if len(parts) <= 2 else "deep_project_import"

    if parts[0] != "super" or len(parts) != 2:
        return "non_parent_implementation_import"
    return None


def is_whitelisted_alias_import(alias: str, target: str) -> bool:
    return (alias, target) in CONFIG["whitelisted_alias_imports"]


def is_whitelisted_alias_import_for_path(path: str, alias: str, target: str) -> bool:
    if is_whitelisted_alias_import(alias, target):
        return True
    for prefix, allowed in CONFIG["whitelisted_alias_import_prefixes"].items():
        if path.startswith(prefix) and (alias, target) in allowed:
            return True
    return False


def qualified_path_matches(line: str) -> List[str]:
    matches: List[str] = []
    for qp in QUALIFIED_PATH_RE.finditer(line):
        candidate = qp.group(1).strip()
        parts = candidate.split("::")
        if len(parts) == 2:
            head = parts[0]
            if head not in ("crate", "super", "self"):
                is_pascal_case = head and head[0].isupper()
                is_primitive = head in CONFIG["rust_primitive_types"]
                if is_pascal_case or is_primitive:
                    continue
            if candidate in CONFIG.get("whitelisted_inline_qualified_paths", set()):
                continue

        if any(seg.startswith("__") for seg in parts):
            continue
        if re.match(r"^[A-Za-z_][A-Za-z0-9_]*::<", candidate):
            continue
        if "::{" in candidate:
            continue
            
        head = parts[0]
        if head in RUST_KEYWORDS or keyword.iskeyword(head):
            matches.append(candidate)
            continue
            
        matches.append(candidate)
    return matches


def count_alias_usage(lines: List[str], alias: str, end_line: int) -> int:
    pattern = re.compile(rf"\b{re.escape(alias)}\b")
    return sum(len(pattern.findall(line)) for line in lines[end_line:])


def module_name(path: Path) -> str:
    rel = path.relative_to(ROOT).with_suffix("")
    parts = list(rel.parts)
    if parts and parts[-1] == "mod":
        parts.pop()
    return "::".join(parts)


def staircase_boundaries() -> Set[str]:
    boundaries: Set[str] = set()
    for directory in SRC.rglob("*"):
        if not directory.is_dir():
            continue
        has_api = (directory / "api.rs").exists() or (directory / "api" / "mod.rs").exists()
        if has_api and (directory / "logic").is_dir():
            rel = directory.relative_to(ROOT)
            boundaries.add("::".join(rel.parts))
    return boundaries
