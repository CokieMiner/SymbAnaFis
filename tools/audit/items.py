"""Position-preserving outer attributes and Rust item boundaries for structural checks.

This scanner does not expand macros or evaluate target cfgs. It inspects every
source branch and keeps literals masked while matching balanced delimiters.
"""

from __future__ import annotations

import re
from dataclasses import dataclass

from .rust_source import clean_rust_code, matching_delimiter, split_top_level


@dataclass(frozen=True)
class Attribute:
    start: int
    end: int
    inner: bool
    code: str
    raw: str


@dataclass(frozen=True)
class Item:
    start: int
    end: int
    kind: str
    name: str
    header: str
    attributes: tuple[Attribute, ...]


def attributes(text: str) -> list[Attribute]:
    cleaned = clean_rust_code(text, scrub_attributes=False)
    result = []
    cursor = 0
    pattern = re.compile(r"#\s*(!?)\s*\[")
    while match := pattern.search(cleaned, cursor):
        opening = match.end() - 1
        closing = matching_delimiter(cleaned, opening, "[", "]")
        if closing is None:
            raise ValueError(f"unclosed attribute at offset {match.start()}")
        result.append(Attribute(match.start(), closing + 1, bool(match[1]),
                                cleaned[opening + 1:closing].strip(), text[opening + 1:closing]))
        cursor = closing + 1
    return result


def top_level_items(text: str) -> list[Item]:
    cleaned = clean_rust_code(text, scrub_attributes=False)
    attrs = {attribute.start: attribute for attribute in attributes(text)}
    result = []
    pending = []
    cursor = 0
    while cursor < len(cleaned):
        if cleaned[cursor].isspace() or cleaned[cursor] == ";":
            cursor += 1
            continue
        if cursor in attrs:
            attribute = attrs[cursor]
            if not attribute.inner:
                pending.append(attribute)
            cursor = attribute.end
            continue
        start = cursor
        # A use tree's braces do not delimit an item body.
        prefix = re.match(r"(?:pub\s*(?:\([^)]*\)\s*)?)?\s*(use|mod|extern\s+crate)\b", cleaned[cursor:])
        is_use = prefix and prefix[1] in {"use", "extern crate"}
        while cursor < len(cleaned):
            char = cleaned[cursor]
            if char in "([":
                closing = matching_delimiter(cleaned, cursor, char, ")" if char == "(" else "]")
                if closing is None:
                    raise ValueError(f"unclosed delimiter at offset {cursor}")
                cursor = closing + 1
            elif char == "{" and not is_use:
                closing = matching_delimiter(cleaned, cursor, "{", "}")
                if closing is None:
                    raise ValueError(f"unclosed item body at offset {cursor}")
                header = cleaned[start:cursor].strip()
                cursor = closing + 1
                break
            elif char == ";":
                header = cleaned[start:cursor].strip()
                cursor += 1
                break
            else:
                cursor += 1
        else:
            raise ValueError(f"unterminated Rust item at offset {start}")
        declaration = re.search(r"\b(extern\s+crate|use|mod|fn|impl|struct|enum|union|trait|type|static|const|macro_rules|macro)\b(?:\s+([A-Za-z_]\w*))?", header)
        kind = declaration[1] if declaration else "macro_invocation"
        name = declaration[2] or "" if declaration else ""
        result.append(Item(start, cursor, kind, name, header, tuple(pending)))
        pending = []
    return result


def cfg_requires_test(code: str) -> bool:
    """Prove that a cfg expression cannot enable an item when test=false."""
    def possible(expression: str) -> set[bool]:
        expression = expression.strip()
        if expression == "test":
            return {False}
        match = re.fullmatch(r"(all|any|not)\s*\((.*)\)", expression, re.DOTALL)
        if not match:
            return {False, True}
        values = [possible(part) for part in split_top_level(match[2])]
        if match[1] == "not":
            return {not value for value in values[0]} if len(values) == 1 else {False, True}
        if match[1] == "all":
            return ({True} if all(True in value for value in values) else set()) | ({False} if any(False in value for value in values) else set())
        return ({True} if any(True in value for value in values) else set()) | ({False} if all(False in value for value in values) else set())

    match = re.fullmatch(r"cfg\s*\((.*)\)", code, re.DOTALL)
    return bool(match) and True not in possible(match[1])
