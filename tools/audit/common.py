"""Common configuration and diagnostic models for repository audits."""

from __future__ import annotations

from dataclasses import dataclass
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
SRC = ROOT / "src"

CONFIG = {
    "whitelisted_crate_root_imports": set(),
    "whitelisted_alias_imports": {
        ("FmtResult", "core::fmt::Result"),
    },
    "whitelisted_alias_import_prefixes": {},
    "rust_primitive_types": {
        "f32", "f64", "f64x4", "i8", "i16", "i32", "i64", "i128", "isize",
        "u8", "u16", "u32", "u64", "u128", "usize", "str", "bool", "char"
    },
    "whitelisted_inline_qualified_paths": {
        "alloc::vec",
        "array::from_fn",
    },
    "architecture_wiring_basenames": {
        "runtime_dispatch.rs",
    },
    "architecture_wiring_paths": {
        "src/int/logic/unsigned/math/arch/backend_providers.rs",
        "src/int/logic/unsigned/math/arch/kernel_selection.rs",
        "src/int/logic/unsigned/math/arch/kernels.rs",
        "src/int/logic/unsigned/math/arch/x86_selectors.rs",
    },
}


@dataclass(frozen=True)
class Finding:
    kind: str
    path: str
    line: int
    detail: str


@dataclass(frozen=True)
class ImportEdge:
    source: str
    target: str
    raw: str
    path: str
    line: int
    internal: bool


@dataclass(frozen=True)
class AliasImport:
    path: str
    line: int
    source_module: str
    target: str
    alias: str
    usage_count: int
