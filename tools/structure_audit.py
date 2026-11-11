#!/usr/bin/env python3
"""Audits Rust source-file structure, visibility gates, module registry purity, and production completeness."""

import sys
from pathlib import Path

# Appends tools root directory to sys.path.
_TOOLS_DIR = Path(__file__).resolve().parent
if str(_TOOLS_DIR) not in sys.path:
    sys.path.insert(0, str(_TOOLS_DIR))

from audit.structure_rules import run_structure_audit

if __name__ == "__main__":
    sys.exit(run_structure_audit())
