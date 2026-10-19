"""Small, position-preserving Rust source scanner shared by repository audits."""

from __future__ import annotations


def is_char_literal_start(text: str, offset: int) -> bool:
    """Distinguish Rust character literals from lifetimes and labels."""
    value_offset = offset + 1
    if value_offset >= len(text):
        return False
    if text[value_offset] == "\\":
        return True
    return value_offset + 1 < len(text) and text[value_offset + 1] == "'"


def clean_rust_code(text: str, *, scrub_attributes: bool = True) -> str:
    """Replace comments, literals, and optionally attributes while preserving offsets."""
    result: list[str] = []
    offset = 0
    state = "code"
    block_depth = 0
    raw_hashes = 0

    while offset < len(text):
        if state == "code":
            if text.startswith("//", offset):
                state = "line_comment"
                result.extend("  ")
                offset += 2
            elif text.startswith("/*", offset):
                state = "block_comment"
                block_depth = 1
                result.extend("  ")
                offset += 2
            elif text[offset] == "r":
                cursor = offset + 1
                while cursor < len(text) and text[cursor] == "#":
                    cursor += 1
                if cursor < len(text) and text[cursor] == '"':
                    state = "raw_string"
                    raw_hashes = cursor - offset - 1
                    result.extend(" " * (cursor - offset + 1))
                    offset = cursor + 1
                else:
                    result.append(text[offset])
                    offset += 1
            elif text[offset] == '"':
                state = "string"
                result.append(" ")
                offset += 1
            elif text[offset] == "'" and is_char_literal_start(text, offset):
                state = "char"
                result.append(" ")
                offset += 1
            else:
                result.append(text[offset])
                offset += 1
        elif state == "line_comment":
            char = text[offset]
            result.append("\n" if char == "\n" else " ")
            if char == "\n":
                state = "code"
            offset += 1
        elif state == "block_comment":
            if text.startswith("/*", offset):
                block_depth += 1
                result.extend("  ")
                offset += 2
            elif text.startswith("*/", offset):
                block_depth -= 1
                result.extend("  ")
                offset += 2
                if block_depth == 0:
                    state = "code"
            else:
                char = text[offset]
                result.append("\n" if char == "\n" else " ")
                offset += 1
        elif state in {"string", "char"}:
            terminator = '"' if state == "string" else "'"
            if text[offset] == "\\" and offset + 1 < len(text):
                result.extend("\n" if char == "\n" else " " for char in text[offset:offset + 2])
                offset += 2
            else:
                char = text[offset]
                result.append("\n" if char == "\n" else " ")
                offset += 1
                if char == terminator:
                    state = "code"
        else:
            terminator = '"' + "#" * raw_hashes
            if text.startswith(terminator, offset):
                result.extend(" " * len(terminator))
                offset += len(terminator)
                state = "code"
            else:
                result.append("\n" if text[offset] == "\n" else " ")
                offset += 1

    cleaned = "".join(result)
    if not scrub_attributes:
        return cleaned

    result = []
    offset = 0
    attribute_depth = 0
    while offset < len(cleaned):
        if attribute_depth == 0:
            if cleaned.startswith("#![", offset):
                attribute_depth = 1
                result.extend("   ")
                offset += 3
            elif cleaned.startswith("#[", offset):
                attribute_depth = 1
                result.extend("  ")
                offset += 2
            else:
                result.append(cleaned[offset])
                offset += 1
        else:
            char = cleaned[offset]
            if char == "[":
                attribute_depth += 1
            elif char == "]":
                attribute_depth -= 1
            result.append("\n" if char == "\n" else " ")
            offset += 1
    return "".join(result)


def matching_delimiter(text: str, opening: int, left: str, right: str) -> int | None:
    """Return the matching delimiter offset in already-cleaned source."""
    depth = 0
    for offset in range(opening, len(text)):
        if text[offset] == left:
            depth += 1
        elif text[offset] == right:
            depth -= 1
            if depth == 0:
                return offset
    return None


def split_top_level(text: str, delimiter: str = ",") -> list[str]:
    """Split at delimiters outside nested parentheses, brackets, and braces."""
    items: list[str] = []
    current: list[str] = []
    depths = {"(": 0, "[": 0, "{": 0}
    closing = {")": "(", "]": "[", "}": "{"}
    for char in text:
        if char in depths:
            depths[char] += 1
        elif char in closing:
            depths[closing[char]] -= 1
        if char == delimiter and not any(depths.values()):
            item = "".join(current).strip()
            if item:
                items.append(item)
            current = []
        else:
            current.append(char)
    tail = "".join(current).strip()
    if tail:
        items.append(tail)
    return items
