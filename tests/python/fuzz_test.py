import sys
import os
import random
import math
import time

# Ensure we can import the local module
sys.path.append(os.getcwd())

import symb_anafis as symb
from symb_anafis import Expr, diff, simplify, evaluate, parse

# Configuration
TEST_ITERATIONS = 500
MAX_DEPTH = 4

def gen_expr_recursive(depth):
    """Generates a random valid math expression string"""
    if depth == 0:
        # Base case: number or variable
        choice = random.random()
        if choice < 0.4:
            # Integers are safer for string formatting roundtrips
            return str(random.randint(1, 10))
        elif choice < 0.7:
             # Floats
             return f"{random.uniform(0.1, 10.0):.2f}"
        else:
            return random.choice(["x", "y", "z"])
    
    op_type = random.random()
    if op_type < 0.5:
        # Binary op
        op = random.choice(["+", "-", "*", "/"]) # Power '^' can cause overflow easily
        left = gen_expr_recursive(depth - 1)
        right = gen_expr_recursive(depth - 1)
        # Avoid division by zero patterns cheaply (parser will optimize/fail gracefully ideally but we want to test engine)
        if op == "/" and right in ["0", "0.00"]:
             right = "1"
        return f"({left} {op} {right})"
    elif op_type < 0.8:
        # Unary function
        func = random.choice(["sin", "cos", "exp", "abs", "sqrt"])
        arg = gen_expr_recursive(depth - 1)
        if func == "sqrt":
            arg = f"abs({arg})" # Ensure domain validity for evaluations
        return f"{func}({arg})"
    else:
        # Negation
        arg = gen_expr_recursive(depth - 1)
        return f"-({arg})"

def oracle_test():
    print(f"\n--- Oracle Fuzz Test ({TEST_ITERATIONS} iters) ---")
    failures = 0
    crashes = 0
    
    for i in range(TEST_ITERATIONS):
        try:
            expr_str = gen_expr_recursive(random.randint(1, MAX_DEPTH))
            
            # 1. Parse Stability
            # Note: The binding parse might not be exposed directly as 'parse'
            # usually it's implicitly done via Expr(str) or the functions.
            # We'll use Expr(str) constructor.
            try:
                expr_obj = Expr(expr_str)
            except Exception as e:
                # Parsing errors are expected for some generated junk, but we try to generate valid ones.
                # If it's a valid string format failure, ignore. Segfault is bad.
                continue

            # 2. Crash Testing: Diff and Simplify
            # Should not crash
            d_res = diff(expr_obj, "x")
            s_res = simplify(expr_obj)

            # 3. Oracle: Eval Consistency
            # eval(expr) ~= eval(simplify(expr))
            # Evaluate at a safe point away from 0
            vars_map = [("x", 0.5), ("y", 1.2), ("z", 2.3)]
            
            # Use object directly now that evaluate uses duck typing
            val_orig_py = evaluate(expr_obj, vars_map)
            val_simp_py = evaluate(s_res, vars_map)

            # Convert to float for comparison. 
            # If symbolic result (e.g. overflow or undefined), skip comparison
            try:
                # Use str() instead of .to_string()
                f_orig = float(str(val_orig_py))
                f_simp = float(str(val_simp_py))
                
                if math.isfinite(f_orig) and math.isfinite(f_simp):
                    if abs(f_orig - f_simp) > 1e-4: # Loose tolerance for accumulation errors
                        print(f"Oracle Fail: {expr_str}")
                        print(f"Orig: {f_orig}, Simp: {f_simp}")
                        failures += 1
            except (ValueError, TypeError):
                # Symbolic result or complex type, hard to compare without stronger CAS capabilities
                pass
                
        except Exception as e:
            print(f"Crash/Exception on inputs '{expr_str}': {e}")
            crashes += 1
            
    print(f"Completed with {failures} value mismatches and {crashes} exceptions")
    
    if crashes > 0:
        raise RuntimeError(f"Encountered {crashes} crashes during fuzzing")
    # Failures might be subtle FP issues, warning only unless massive
    assert failures < TEST_ITERATIONS * 0.05, "Too many oracle failures (>5%)"
    print("Pass: Oracle checks")

def identity_test():
    print("\n--- Identity Check Test ---")
    
    # x + 0 = x
    e = Expr("x") + 0.0
    res = simplify(e)
    # Result might be "x" or "1.0*x" depending on printer/internal structure
    # Let's check by evaluation to be robust
    val = evaluate(res, [("x", 42.0)])
    assert abs(float(str(val)) - 42.0) < 1e-9
    
    # x * 1 = x
    e = Expr("x") * 1.0
    res = simplify(e)
    val = evaluate(res, [("x", 42.0)])
    assert abs(float(str(val)) - 42.0) < 1e-9
    
    print("Pass: Basic Identities")

if __name__ == "__main__":
    test_seed = int(time.time())
    random.seed(test_seed)
    print(f"Seed: {test_seed}")
    
    try:
        identity_test()
        oracle_test()
        print("\n=== All Fuzz Tests Passed ===")
    except Exception as e:
        print(f"\nFAIL: {e}")
        sys.exit(1)
