#!/usr/bin/env python3
"""Inventories Rust lint allowance (#[allow]) and expectation (#[expect]) attributes across production and test targets."""

import sys
from pathlib import Path

# Appends tools root directory to sys.path.
_TOOLS_DIR = Path(__file__).resolve().parent
if str(_TOOLS_DIR) not in sys.path:
    sys.path.insert(0, str(_TOOLS_DIR))

from audit.allow_rules import run_check_allows

if __name__ == "__main__":
    sys.exit(run_check_allows())
