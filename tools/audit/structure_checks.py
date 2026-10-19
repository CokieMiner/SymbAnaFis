"""Structural rules that require item boundaries rather than line regexes."""

from __future__ import annotations

import re

from .common import Finding
from .items import attributes, cfg_requires_test, top_level_items
from .rust_source import clean_rust_code, matching_delimiter


def structural_findings(text: str, path: str, *, registry: bool, test_file: bool) -> list[Finding]:
    findings = []
    def add(kind: str, offset: int, detail: str) -> None:
        findings.append(Finding(kind, path, text.count("\n", 0, offset) + 1, detail))

    try:
        attrs = attributes(text)
        items = top_level_items(text)
    except ValueError as error:
        add("unparsed_rust_structure", 0, str(error))
        return findings
    for attribute in attrs:
        attribute_code = clean_rust_code(attribute.raw, scrub_attributes=False)
        for expectation in re.finditer(r"\bexpect\s*\(", attribute_code):
            end = matching_delimiter(attribute_code, expectation.end() - 1, "(", ")")
            body = attribute_code[expectation.end():end]
            reason_key = re.search(r"\breason\s*=", body)
            reason = None
            if reason_key:
                value = attribute.raw[expectation.end() + reason_key.end():end].lstrip()
                literal = re.match(r'"((?:\\.|[^"\\])*)"', value, re.DOTALL)
                if literal:
                    reason = literal[1]
                else:
                    raw_literal = re.fullmatch(r'r(\#*)"(.*)"\1\s*,?\s*', value, re.DOTALL)
                    reason = raw_literal[2] if raw_literal else None
            if not reason or not reason.strip():
                add("expect_without_reason", attribute.start, "lint expectations, including cfg_attr branches, require a nonempty reason")
        if not test_file and re.fullmatch(r"(?:test|(?:\w+::)+test)(?:\s*\(.*\))?", attribute.code, re.DOTALL):
            add("test_in_production_file", attribute.start, "test functions belong in a dedicated tests.rs or tests/ directory")
    if registry:
        stage = 0
        for item in items:
            is_test_module = item.kind == "mod" and (item.name == "tests" or item.name.endswith("_tests"))
            if is_test_module:
                if not any(cfg_requires_test(attribute.code) for attribute in item.attributes):
                    add("test_module_without_test_gate", item.start, "test module must be disabled outside cfg(test)")
                current = 3
            elif item.kind == "use":
                current = 2 if re.match(r"pub\b", item.header) else 0
            elif item.kind in {"mod", "extern crate"}:
                current = 1
            elif (item.kind == "macro_invocation"
                  and item.header.strip() == "select_arch_kernel!"
                  and path.startswith("src/int/logic/unsigned/math/arch/")):
                # The architecture registry DSL declares cfg-selected modules
                # and reexports. Its implementation lives in dedicated source
                # files; arbitrary macros remain forbidden in registries.
                current = 1
            else:
                add("implementation_in_module_registry", item.start, item.header)
                continue
            if current < stage:
                add("module_registry_order", item.start, "required order: parent imports, modules, reexports, test modules")
            stage = max(stage, current)
            if item.kind == "mod" and not text[item.start:item.end].rstrip().endswith(";"):
                add("inline_module_in_registry", item.start, "module registries contain external module declarations only")
    if not test_file:
        cleaned = clean_rust_code(text, scrub_attributes=False)
        # Dedicated type owners declare descendants directly so their fields stay
        # private without requiring an intermediate forwarding module.
        type_owner = any(item.kind in {"struct", "enum", "union"} for item in items)
        for index, item in enumerate(items):
            if item.kind != "mod":
                continue
            named_test = item.name == "tests" or item.name.endswith("_tests")
            gated = any(cfg_requires_test(attribute.code) for attribute in item.attributes)
            if not (named_test or gated):
                continue
            if not registry:
                if not type_owner:
                    add("test_module_outside_owner", item.start, "declare tests in mod.rs or a dedicated type owner")
                if not gated:
                    add("test_module_without_test_gate", item.start, "test module must be disabled outside cfg(test)")
                if index != len(items) - 1:
                    add("test_module_not_last", item.start, "test declaration must be the final item of its type owner")
            if cleaned[item.start:item.end].rstrip().endswith("}"):
                add("inline_test_module", item.start, "test implementations must be in separate files")
    return findings
