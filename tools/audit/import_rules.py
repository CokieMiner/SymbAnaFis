"""Audits Rust import boundaries, grouping order, and facade re-export constraints."""

from __future__ import annotations

import argparse
import json
import re
from pathlib import Path
from typing import Dict, List, Optional, Set, Tuple

from .common import AliasImport, Finding, ImportEdge, ROOT, SRC
from .import_parser import (
    CODE_ITEM_RE,
    IMPORT_GROUPS,
    INLINE_MOD_RE,
    MACRO_INVOCATION_RE,
    MODULE_DECL_RE,
    boundary_for_module,
    count_alias_usage,
    expand_use_tree,
    import_groups,
    import_sort_key,
    is_internal_target,
    is_shallow_crate_import,
    is_test_module_name,
    is_test_path,
    is_whitelisted_alias_import_for_path,
    is_whitelisted_crate_root_import,
    module_name,
    project_import_violation,
    qualified_path_matches,
    resolve_relative_path,
    staircase_boundaries,
)
from .rust_source import clean_rust_code
from .import_checks import import_structure_findings


class FileAnalyzer:
    """Analyzes import declarations, grouping order, and module boundaries in a Rust source file."""

    def __init__(self, path: Path, boundaries: Set[str]) -> None:
        self.path = path
        self.rel_path = str(path.relative_to(ROOT)).replace("\\", "/")
        self.boundaries = boundaries
        self.raw_text = path.read_text(encoding="utf-8")
        self.raw_lines = self.raw_text.splitlines()
        self.lines = clean_rust_code(self.raw_text, scrub_attributes=False).splitlines()
        self.code_lines = clean_rust_code(self.raw_text, scrub_attributes=True).splitlines()
        self.base_module = module_name(path)
        self.local_modules = {
            m.group(1)
            for m in re.finditer(MODULE_DECL_RE.pattern, clean_rust_code(self.raw_text), re.MULTILINE)
            if not is_test_module_name(m.group(1))
        }

        self.finding_keys: Set[Tuple[str, str, int, str]] = set()
        self.edges: List[ImportEdge] = []
        self.alias_imports: List[AliasImport] = []
        self.private_imports: List[Tuple[int, int, str, int]] = []
        self.public_imports: List[Tuple[int, int, str]] = []
        self.module_declaration_lines: List[int] = []

        self.macro_definitions: Dict[str, Set[str]] = {}
        self.macro_invocations: List[Tuple[str, str, int]] = []
        self.file_imports: Set[str] = set()

        self.in_use_stmt = False
        self.use_stmt_lines: List[str] = []
        self.use_start_lineno = 0
        self.use_start_depth = 0
        self.brace_depths = {"{": 0, "(": 0, "[": 0}
        self.nested_modules: List[Tuple[str, int]] = []
        self.seen_code = False
        self.macro_body_depth: Optional[int] = None
        self.current_macro_name: Optional[str] = None
        self.macro_open_char: Optional[str] = None

    def analyze(self) -> None:
        parent = self.path.parent / "mod.rs"
        parent_text = parent.read_text(encoding="utf-8") if parent.is_file() and parent != self.path else None
        for finding in import_structure_findings(self.raw_text, self.rel_path, parent_text):
            self.finding_keys.add((finding.kind, finding.path, finding.line, finding.detail))
        for lineno, line in enumerate(self.lines, start=1):
            self._handle_line(line, lineno)
        self._check_import_order()
        self._check_reexport_order()
        if self.in_use_stmt:
            self.finding_keys.add(("unterminated_import", self.rel_path, self.use_start_lineno, "use declaration has no terminating semicolon"))

    def _handle_line(self, line: str, lineno: int) -> None:
        is_use_line = False
        if not self.in_use_stmt:
            m = re.match(r"^\s*(?:pub(?:\([^)]+\))?\s+)?use\s+(.*)", line)
            if m:
                is_use_line = True
                self.in_use_stmt = True
                self.use_start_lineno = lineno
                self.use_start_depth = self.brace_depths["{"]
                self.use_stmt_lines = [m.group(1)]
                if ";" in line:
                    self.in_use_stmt = False
                    self._process_collected_use_stmt(lineno)
        else:
            is_use_line = True
            self.use_stmt_lines.append(line)
            if ";" in line:
                self.in_use_stmt = False
                self._process_collected_use_stmt(lineno)

        mod_match = INLINE_MOD_RE.match(line)
        if mod_match:
            next_depth = self.brace_depths['{'] + line.count("{") - line.count("}")
            if next_depth > self.brace_depths['{']:
                self.nested_modules.append((mod_match.group(1), next_depth))

        mr_match = re.search(r"\bmacro_rules!\s+([A-Za-z_][A-Za-z0-9_]*)\s*([{\[(])", line)
        if self.macro_body_depth is None and mr_match:
            self.current_macro_name = mr_match.group(1)
            self.macro_open_char = mr_match.group(2)
            self.macro_body_depth = self.brace_depths[self.macro_open_char]

        inside_macro = (self.macro_body_depth is not None and self.macro_open_char and self.brace_depths[self.macro_open_char] > self.macro_body_depth)
        inside_test_module = any(is_test_module_name(name) for name, _depth in self.nested_modules)
        module_declaration = MODULE_DECL_RE.match(line)
        is_mod_line = module_declaration is not None
        if (
            module_declaration is not None
            and not is_test_module_name(module_declaration.group(1))
            and self.brace_depths["{"] == 0
        ):
            self.module_declaration_lines.append(lineno)

        is_pub_use = bool(re.match(r"^\s*pub(?:\([^)]+\))?\s+use\b", line))
        if is_use_line and not is_pub_use and self.seen_code and not inside_test_module and not inside_macro and not self.nested_modules:
            self.finding_keys.add(("use_not_at_top", self.rel_path, lineno, "import declaration follows non-header item; imports must reside in module header"))

        code_line = self.code_lines[lineno - 1] if lineno - 1 < len(self.code_lines) else ""
        if not is_use_line and not inside_test_module and not is_mod_line and not mr_match:
            stripped = code_line.strip()
            if self.brace_depths["{"] == 0 and CODE_ITEM_RE.match(code_line):
                self.seen_code = True
            is_ignored_prefix = stripped.startswith("pub(in ") or stripped.startswith("macro_rules!")
            if not is_ignored_prefix:
                self._handle_code_line(code_line, lineno, inside_macro)

        self._update_braces(line)

    def _check_import_order(self) -> None:
        previous: Optional[Tuple[int, int, str, int]] = None
        for current in self.private_imports:
            line, end_line, source, group = current
            if previous is not None:
                _previous_line, previous_end, previous_source, previous_group = previous
                if group < previous_group:
                    self.finding_keys.add(
                        ("import_group_order", self.rel_path, line,
                         f"{source} must precede {previous_source}; expected group order: core/std -> alloc -> external crates -> crate:: -> super:: -> self::/child")
                    )
                elif (group == previous_group and "{" not in source and "{" not in previous_source
                      and import_sort_key(source) < import_sort_key(previous_source)):
                    self.finding_keys.add(
                        ("import_path_order", self.rel_path, line, f"{source} must sort before {previous_source}")
                    )

                between = self.raw_lines[previous_end: line - 1]
                blank_lines = sum(not item.strip() for item in between)
                expected_blanks = 1 if group != previous_group else 0
                if blank_lines != expected_blanks:
                    self.finding_keys.add(
                        ("import_group_spacing", self.rel_path, line,
                         f"{IMPORT_GROUPS[group]} import follows {IMPORT_GROUPS[previous_group]} with {blank_lines} blank line(s); expected {expected_blanks}")
                    )
            previous = current

    def _check_reexport_order(self) -> None:
        if self.path.name not in {"mod.rs", "lib.rs"} or not self.public_imports:
            return

        last_module_line = max(self.module_declaration_lines, default=0)
        for line, _end_line, source in self.public_imports:
            if line < last_module_line:
                self.finding_keys.add(
                    ("reexport_before_module_declarations", self.rel_path, line, f"{source} must follow all module declarations")
                )

        previous_source: Optional[str] = None
        for line, _end_line, source in self.public_imports:
            if (previous_source is not None and "{" not in source and "{" not in previous_source
                and import_sort_key(source.split("::", 1)[0]) < import_sort_key(previous_source.split("::", 1)[0])):
                self.finding_keys.add(
                    ("reexport_path_order", self.rel_path, line, f"{source} must sort before {previous_source}")
                )
            previous_source = source

        first_reexport_line = self.public_imports[0][0]
        for line, _end_line, source, _group in self.private_imports:
            if line > first_reexport_line:
                self.finding_keys.add(
                    ("private_import_after_reexport", self.rel_path, line, f"{source} must precede module declarations and facade re-exports in mod.rs")
                )

    def _process_collected_use_stmt(self, end_lineno: int) -> None:
        full_use = " ".join(self.use_stmt_lines).split(";", 1)[0].strip()
        self.use_stmt_lines = []
        
        source_parts = self.base_module.split("::") + [name for name, _depth in self.nested_modules]
        if source_parts == [""]: source_parts = []
        source_mod = "::".join(source_parts)
        source_boundary = boundary_for_module(source_mod, self.boundaries)
        inside_test_module = any(is_test_module_name(name) for name, _depth in self.nested_modules)

        is_pub_use = bool(re.match(r"^\s*pub(?:\([^)]+\))?\s+use\b", self.lines[self.use_start_lineno - 1]))
        if not self.nested_modules and self.use_start_depth == 0 and self.macro_body_depth is None:
            if is_pub_use:
                self.public_imports.append((self.use_start_lineno, end_lineno, full_use))
            else:
                groups = import_groups(full_use)
                if len(groups) > 1:
                    names = ", ".join(IMPORT_GROUPS[group] for group in sorted(groups))
                    self.finding_keys.add(
                        ("mixed_import_groups", self.rel_path, self.use_start_lineno, f"use tree conflates distinct import groups ({names}); imports from separate groups require independent declarations")
                    )
                self.private_imports.append((self.use_start_lineno, end_lineno, full_use, min(groups)))
        
        if inside_test_module:
            return
            
        self._handle_use_statement(full_use, source_mod, source_boundary, self.use_start_lineno, end_lineno)

    def _handle_use_statement(
        self,
        raw_use: str,
        source_mod: str,
        source_boundary: Optional[str],
        lineno: int,
        end_lineno: int,
    ) -> None:
        is_pub_use = bool(re.match(r"^\s*pub(?:\([^)]+\))?\s+use\b", self.lines[lineno - 1]))
        for raw_target, alias in expand_use_tree(raw_use):
            generated_threshold_exports = (
                is_pub_use and self.rel_path == "src/int/logic/unsigned/math/mod.rs"
                and raw_target == "thresholds::*"
            )
            # A macro repetition such as `$($name),*` is not a glob import.
            if ("$" not in raw_use and not generated_threshold_exports
                    and (raw_target == "*" or raw_target.endswith("::*"))):
                self.finding_keys.add(("wildcard_import", self.rel_path, lineno, "import explicit names so subsystem dependencies remain visible"))
            normalized = resolve_relative_path(source_mod, raw_target)
            self.file_imports.add(normalized)
            self.edges.append(
                ImportEdge(source=source_mod, target=normalized, raw=raw_target,
                           path=self.rel_path, line=lineno, internal=is_internal_target(normalized))
            )

            if alias and alias != "_" and not is_whitelisted_alias_import_for_path(self.rel_path, alias, normalized):
                usage_count = count_alias_usage(self.lines, alias, end_lineno)
                if not (usage_count == 0 and is_pub_use):
                    self.alias_imports.append(
                        AliasImport(path=self.rel_path, line=lineno, source_module=source_mod,
                                    target=normalized, alias=alias, usage_count=usage_count)
                    )
                if usage_count == 0 and not is_pub_use:
                    self.finding_keys.add(("unused_alias_import", self.rel_path, lineno, f"{alias} -> {normalized}"))

            if raw_target.startswith("super::super"):
                self.finding_keys.add(("deep_relative_import", self.rel_path, lineno, raw_target))

            import_violation = project_import_violation(
                raw_target, source_path=self.rel_path, source_mod=source_mod,
                plumbing_file=Path(self.rel_path).name in {"mod.rs", "lib.rs"},
                local_modules=self.local_modules,
            )
            if import_violation is not None:
                self.finding_keys.add((import_violation, self.rel_path, lineno, raw_target))

            if raw_target.startswith("crate::"):
                if source_boundary is not None:
                    target_boundary = boundary_for_module(normalized, self.boundaries)
                    if target_boundary == source_boundary:
                        self.finding_keys.add(("self_referential_crate_import", self.rel_path, lineno, f"{raw_target}: subsystem-internal import must use relative path (super::)"))

            if is_shallow_crate_import(raw_target) and not is_whitelisted_crate_root_import(raw_target):
                self.finding_keys.add(("crate_root_import", self.rel_path, lineno, raw_target))

            if "::logic::" in normalized and normalized.startswith("src::"):
                logic_boundary = normalized.split("::logic::", 1)[0]
                if source_boundary != logic_boundary:
                    self.finding_keys.add(("cross_boundary_logic_import", self.rel_path, lineno, f"{source_mod} -> {normalized}"))
                elif not (source_mod == logic_boundary or "::api" in source_mod or "::tune_api" in source_mod or source_mod.startswith(f"{logic_boundary}::logic")):
                    after_logic = normalized.split("::logic::", 1)[1]
                    if "::" in after_logic:
                        self.finding_keys.add(("deep_logic_surface_bypass", self.rel_path, lineno, f"{source_mod} -> {normalized}"))
                    else:
                        self.finding_keys.add(("logic_surface_import", self.rel_path, lineno, f"{source_mod} -> {normalized}"))

    def _handle_code_line(self, line: str, lineno: int, inside_macro: bool) -> None:
        for inv_match in MACRO_INVOCATION_RE.findall(line):
            if inv_match not in {"macro_rules"}:
                self.macro_invocations.append((self.rel_path, inv_match, lineno))

        for match in qualified_path_matches(line):
            if inside_macro and self.current_macro_name:
                resolved_path = match
                if resolved_path.startswith("$crate::"):
                    resolved_path = "src::" + resolved_path[8:]
                elif resolved_path.startswith("crate::"):
                    resolved_path = "src::" + resolved_path[7:]
                self.macro_definitions.setdefault(self.current_macro_name, set()).add(resolved_path)
            else:
                self.finding_keys.add(("inline_qualified_path", self.rel_path, lineno, match))

    def _update_braces(self, line: str) -> None:
        self.brace_depths['{'] += line.count("{") - line.count("}")
        self.brace_depths['('] += line.count("(") - line.count(")")
        self.brace_depths['['] += line.count("[") - line.count("]")
        
        if self.macro_body_depth is not None and self.macro_open_char and self.brace_depths[self.macro_open_char] <= self.macro_body_depth:
            self.macro_body_depth = None
            self.current_macro_name = None
            self.macro_open_char = None
            
        while self.nested_modules and self.brace_depths['{'] < self.nested_modules[-1][1]:
            self.nested_modules.pop()


def collect_findings() -> Tuple[List[Finding], List[ImportEdge], Set[str], List[AliasImport]]:
    boundaries = staircase_boundaries()
    finding_keys: Set[Tuple[str, str, int, str]] = set()
    edges: List[ImportEdge] = []
    alias_imports: List[AliasImport] = []
    
    macro_definitions: Dict[str, Set[str]] = {}
    macro_invocations: List[Tuple[str, str, int]] = []
    file_imports: Dict[str, Set[str]] = {}

    for path in sorted(SRC.rglob("*.rs")):
        if is_test_path(path):
            continue
            
        analyzer = FileAnalyzer(path, boundaries)
        analyzer.analyze()
        
        finding_keys.update(analyzer.finding_keys)
        edges.extend(analyzer.edges)
        alias_imports.extend(analyzer.alias_imports)
        
        macro_invocations.extend(analyzer.macro_invocations)
        for macro, reqs in analyzer.macro_definitions.items():
            macro_definitions.setdefault(macro, set()).update(reqs)
        
        file_imports[analyzer.rel_path] = analyzer.file_imports

    findings = [Finding(kind=k, path=p, line=l, detail=d) for k, p, l, d in sorted(finding_keys)]
    return findings, edges, boundaries, alias_imports


def run_import_audit(argv: Optional[List[str]] = None) -> int:
    parser = argparse.ArgumentParser(description="Audit Rust import paths, grouping order, visibility boundaries, and module-registry constraints.")
    parser.add_argument("--json", action="store_true", help="Print findings as JSON.")
    args = parser.parse_args(argv)

    findings, edges, boundaries, alias_imports = collect_findings()
    unused_aliases = [item for item in alias_imports if item.usage_count == 0]

    if args.json:
        payload = {
            "boundaries": sorted(boundaries),
            "findings": [Finding(f.kind, f.path, f.line, f.detail).__dict__ for f in findings],
            "alias_imports": [a.__dict__ for a in alias_imports],
            "unused_alias_imports": [a.__dict__ for a in unused_aliases],
            "edges": [e.__dict__ for e in edges],
        }
        print(json.dumps(payload, indent=2))
        return 0 if not findings else 1

    print(f"Boundaries detected: {len(boundaries)}")
    print(f"Findings: {len(findings)}")
    print(f"Aliased imports: {len(alias_imports)}")
    print(f"Unused aliased imports: {len(unused_aliases)}")

    if findings:
        print("\nFindings:")
        for finding in findings:
            print(f"  {finding.path}:{finding.line} [{finding.kind}] {finding.detail}")
        return 1

    return 0
