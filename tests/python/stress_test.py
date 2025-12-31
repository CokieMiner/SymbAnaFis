import sys
import os
import resource
import time
import math

# Ensure we can import the local module
sys.path.append(os.getcwd())

import symb_anafis as symb
from symb_anafis import Expr, diff, simplify, evaluate_str

def get_memory_usage():
    """Returns RSS memory usage in MB"""
    usage = resource.getrusage(resource.RUSAGE_SELF).ru_maxrss
    # Linux reports in KB, MacOS in bytes. Assuming Linux based on environment context.
    return usage / 1024.0

def test_deep_recursion():
    print("\n--- Deep Recursion Test ---")
    depth = 50
    expr_str = "x"
    # Create sin(sin(sin(...(x)...)))
    for _ in range(depth):
        expr_str = f"sin({expr_str})"
    
    print(f"Created expression with depth {depth}")
    
    start_time = time.time()
    # Differentiate deeply nested expression
    res = diff(expr_str, "x")
    duration = time.time() - start_time
    
    print(f"Differentiation took {duration:.4f}s")
    # Result should be huge product of cos terms
    assert "cos" in str(res)
    print("Pass: Deep recursion handled without crash")

def test_large_expression():
    print("\n--- Large Expression Test ---")
    size = 200
    # x + x + x ...
    terms = ["x"] * size
    expr_str = " + ".join(terms)
    
    print(f"Created expression with {size} terms")
    
    start_time = time.time()
    # This should simplify to "200 * x" or similar
    res = simplify(expr_str)
    duration = time.time() - start_time
    
    print(f"Simplification took {duration:.4f}s")
    s_res = str(res)
    print(f"Result (truncated): {s_res[:50]}...")
    
    # Check if it simplified reasonably
    # Depending on simplification rules, might be "200*x" or "x*200" or similar
    assert str(size) in s_res and "x" in s_res
    print("Pass: Large expression simplified")

def test_memory_stability():
    print("\n--- Memory Stability Test ---")
    iterations = 5000
    print(f"Running {iterations} iterations of create-diff-destroy cycle...")
    
    initial_mem = get_memory_usage()
    print(f"Initial Memory: {initial_mem:.2f} MB")
    
    for i in range(iterations):
        # Create a non-trivial expression to exercise allocation
        # (x + y)^2 * sin(z)
        e = Expr("x") + Expr("y")
        e2 = e * e * Expr("z").sin()
        d = e2.diff("x")  # Use Expr.diff() method
        
        if i % 1000 == 0:
            print(f"Iteration {i}, Current Mem: {get_memory_usage():.2f} MB")
            
    final_mem = get_memory_usage()
    print(f"Final Memory: {final_mem:.2f} MB")
    
    # Allow some growth due to python internal caching/arenas, but not linear growth
    growth = final_mem - initial_mem
    print(f"Memory Growth: {growth:.2f} MB")
    
    # Strict check: < 20MB growth for 5000 iterations is generous for python overhead
    # but small enough to catch major C++ leaks per object
    assert growth < 20.0, f"Significant memory leak detected: {growth:.2f} MB"
    print("Pass: Memory stable within tolerances")

def test_variable_exhaustion():
    print("\n--- Variable Exhaustion Test ---")
    count = 2000
    print(f"Creating {count} unique variables...")
    
    # This stresses the global symbol registry and Symbol::anon if used
    vars_list = []
    for i in range(count):
        vars_list.append(f"var_{i}")
        
    s_expr = " + ".join(vars_list)
    # Parse/Process this giant sum using string API
    res = evaluate_str(s_expr, [(v, 1.0) for v in vars_list])
    
    print(f"Evaluated sum of {count} ones: {res}")
    # Should be roughly equal to count (floating point errors possible but not for integer sums usually)
    
    val = float(res)
    assert abs(val - count) < 1e-5
    print("Pass: Massive variable count handled")

if __name__ == "__main__":
    try:
        test_deep_recursion()
        test_large_expression()
        test_memory_stability()
        test_variable_exhaustion()
        print("\n=== All Stress Tests Passed ===")
    except Exception as e:
        print(f"\nFAIL: {e}")
        sys.exit(1)
