#!/usr/bin/env python3
"""Audits Rust import paths, grouping order, visibility boundaries, and module-registry constraints."""

import sys
from pathlib import Path

# Appends tools root directory to sys.path.
_TOOLS_DIR = Path(__file__).resolve().parent
if str(_TOOLS_DIR) not in sys.path:
    sys.path.insert(0, str(_TOOLS_DIR))

from audit.import_rules import run_import_audit

if __name__ == "__main__":
    sys.exit(run_import_audit())
