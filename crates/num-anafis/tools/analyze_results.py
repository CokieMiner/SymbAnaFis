#!/usr/bin/env python3
"""
Analyze ULP results from verify_results.json and generate a precision report.
"""
import sys
from pathlib import Path

tools_dir = Path(__file__).resolve().parent
if str(tools_dir) not in sys.path:
    sys.path.insert(0, str(tools_dir))

from precision_suite.report_cli import run_report_cli

if __name__ == "__main__":
    run_report_cli()
