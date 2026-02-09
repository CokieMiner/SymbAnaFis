"""
View API Demo - Pattern Matching on Expression Structure

Demonstrates the new View API for inspecting expression structure
without exposing internal implementation details.
"""

import symb_anafis as sa


def print_structure(expr, indent=0):
    """Recursively print expression structure using View API"""
    view = expr.view()
    prefix = "  " * indent
    
    if view.is_number:
        print(f"{prefix}Number: {view.value}")
    elif view.is_symbol:
        print(f"{prefix}Symbol: {view.name}")
    elif view.is_sum:
        print(f"{prefix}Sum ({len(view.children)} terms):")
        for child in view.children:
            print_structure(child, indent + 1)
    elif view.is_product:
        print(f"{prefix}Product ({len(view.children)} factors):")
        for child in view.children:
            print_structure(child, indent + 1)
    elif view.is_div:
        print(f"{prefix}Division:")
        print(f"{prefix}  Numerator:")
        print_structure(view.children[0], indent + 2)
        print(f"{prefix}  Denominator:")
        print_structure(view.children[1], indent + 2)
    elif view.is_pow:
        print(f"{prefix}Power:")
        print(f"{prefix}  Base:")
        print_structure(view.children[0], indent + 2)
        print(f"{prefix}  Exponent:")
        print_structure(view.children[1], indent + 2)
    elif view.is_function:
        print(f"{prefix}Function: {view.name} ({len(view.children)} args)")
        for i, child in enumerate(view.children):
            print(f"{prefix}  Arg {i}:")
            print_structure(child, indent + 2)
    elif view.is_derivative:
        print(f"{prefix}Derivative: d^{view.derivative_order}/d{view.derivative_var}^{view.derivative_order}")
        print_structure(view.children[0], indent + 1)


def main():
    print("=" * 70)
    print("VIEW API DEMO - Expression Structure Inspection")
    print("=" * 70)
    
    # Example 1: Polynomial (might be optimized internally as Poly)
    print("\n1. POLYNOMIAL: x^2 + 2*x + 1")
    print("-" * 70)
    x = sa.Expr("x")
    poly = x**2 + 2*x + 1
    print(f"Expression: {poly}")
    print(f"View kind: {poly.view().kind}")
    print("\nStructure:")
    print_structure(poly)
    
    # Example 2: Trigonometric expression
    print("\n\n2. TRIGONOMETRIC: sin(x)^2 + cos(x)^2")
    print("-" * 70)
    trig = x.sin()**2 + x.cos()**2
    print(f"Expression: {trig}")
    print(f"View kind: {trig.view().kind}")
    print("\nStructure:")
    print_structure(trig)
    
    # Example 3: Rational function
    print("\n\n3. RATIONAL FUNCTION: (x + 1) / (x - 1)")
    print("-" * 70)
    rational = (x + 1) / (x - 1)
    print(f"Expression: {rational}")
    print(f"View kind: {rational.view().kind}")
    print("\nStructure:")
    print_structure(rational)
    
    # Example 4: View API properties
    print("\n\n4. VIEW API PROPERTIES")
    print("-" * 70)
    expr = x**2 + 3*x + 5
    view = expr.view()
    
    print(f"Expression: {expr}")
    print(f"Kind:       {view.kind}")
    print(f"Is Sum:     {view.is_sum}")
    print(f"Is Product: {view.is_product}")
    print(f"# Children: {len(view)}")
    print(f"First term: {view[0]}")
    print(f"Last term:  {view[-1]}")
    
    # Example 5: Converting to custom format
    print("\n\n5. CUSTOM CONVERSION (Example: to dict)")
    print("-" * 70)
    
    def to_dict(expr):
        """Convert expression to dictionary representation"""
        view = expr.view()
        result = {"kind": view.kind}
        
        if view.is_number:
            result["value"] = view.value
        elif view.is_symbol:
            result["name"] = view.name
        elif view.is_sum or view.is_product:
            result["children"] = [to_dict(child) for child in view.children]
        elif view.is_div or view.is_pow:
            result["left"] = to_dict(view.children[0])
            result["right"] = to_dict(view.children[1])
        elif view.is_function:
            result["name"] = view.name
            result["args"] = [to_dict(child) for child in view.children]
        elif view.is_derivative:
            result["var"] = view.derivative_var
            result["order"] = view.derivative_order
            result["inner"] = to_dict(view.children[0])
        
        return result
    
    expr = x.sin() + x**2
    print(f"Expression: {expr}")
    print(f"As dict:    {to_dict(expr)}")
    
    # Example 6: Anonymous symbols
    print("\n\n6. ANONYMOUS SYMBOLS")
    print("-" * 70)
    from symb_anafis import Symbol
    anon = Symbol.anon()
    expr = anon + 1
    view = expr.view()
    print(f"Expression:   {expr}")
    print(f"View kind:    {view.kind}")
    print(f"# Children:   {len(view.children)}")
    
    # Find the symbol child (not the number)
    for i, child in enumerate(view.children):
        child_view = child.view()
        if child_view.is_symbol:
            print(f"Symbol child: {child}")
            print(f"Symbol name:  {child_view.name}")
            break
    else:
        print("(No symbol found in children)")
    print("             (Note: anonymous symbols show as '$ID')")
    
    print("\n" + "=" * 70)
    print("Demo complete! View API allows safe pattern matching on expressions.")
    print("=" * 70)


if __name__ == "__main__":
    main()
