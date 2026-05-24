#!/usr/bin/env python3
"""
Generate ULP precision charts and domain analysis from verify_results.json.
"""
import sys
from pathlib import Path

tools_dir = Path(__file__).resolve().parent
if str(tools_dir) not in sys.path:
    sys.path.insert(0, str(tools_dir))

from precision_suite.viz import run_viz_cli # type: ignore

if __name__ == "__main__":
    run_viz_cli()
