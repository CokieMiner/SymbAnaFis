#!/usr/bin/env python3
"""
Run all Python tests for symb_anafis.

Usage:
    python run_python_tests.py           # Run all tests
    python run_python_tests.py -v        # Run with verbose output
    python run_python_tests.py -k foo    # Run tests matching 'foo'
"""

import sys
import os
import subprocess
import argparse

def main():
    parser = argparse.ArgumentParser(description='Run symb_anafis Python tests')
    parser.add_argument('-v', '--verbose', action='store_true', help='Verbose output')
    parser.add_argument('-k', '--keyword', type=str, help='Run tests matching keyword')
    parser.add_argument('--failfast', action='store_true', help='Stop on first failure')
    args = parser.parse_args()

    # Get the directory of this script (tests/)
    script_dir = os.path.dirname(os.path.abspath(__file__))
    test_dir = os.path.join(script_dir, 'python')
    
    # Change to project root so imports work
    project_root = os.path.dirname(script_dir)
    os.chdir(project_root)
    
    # Build the pytest command
    cmd = [sys.executable, '-m', 'pytest', test_dir]
    
    if args.verbose:
        cmd.append('-v')
    
    if args.keyword:
        cmd.extend(['-k', args.keyword])
    
    if args.failfast:
        cmd.append('-x')
    
    # Add color output
    cmd.append('--color=yes')
    
    print(f"Running: {' '.join(cmd)}")
    print("=" * 60)
    
    result = subprocess.run(cmd)
    sys.exit(result.returncode)

if __name__ == '__main__':
    main()
