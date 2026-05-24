#!/usr/bin/env python3
"""
Run precision verification tests for num-anafis special-function implementations.
"""
import sys
from pathlib import Path

tools_dir = Path(__file__).resolve().parent
if str(tools_dir) not in sys.path:
    sys.path.insert(0, str(tools_dir))

from precision_suite.verification_cli import run_verification_cli # type: ignore

if __name__ == "__main__":
    run_verification_cli()
