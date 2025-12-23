# SymbAnaFis Roadmap

## v0.5.0 - Performance Optimization

### Goal
Improve differentiation performance through targeted optimizations especially on the simplification engine

---

## v0.6.0 - Equation Solving

### Goal
Symbolic equation solving and algebraic manipulation.

### Planned Features
- [ ] Linear equation solver
- [ ] Polynomial root finding
- [ ] System of equations
- [ ] Substitution and isolation

---

## v0.7.0 - Extended Compiled Evaluation

### Goal
Extend `eval_batch` to support additional special functions in the bytecode VM.

### Planned Instruction Support
- [ ] `Factorial`, `DoubleFactorial` - Factorial operations

### Planned Special Function Support in eval_batch
- [ ] Exponential integrals: `Ei`, `Li`
- [ ] Trigonometric integrals: `Si`, `Ci`
- [ ] Fresnel integrals: `FresnelS`, `FresnelC`
- [ ] Airy functions: `AiryAi`, `AiryBi`
- [ ] Extended Bessel functions (currently partial)
- [ ] Orthogonal polynomials: `Legendre`, `Chebyshev`, `Laguerre`

---

## Ideas / Backlog

- [ ] Series expansion (Taylor, Laurent)
- [ ] Limits
- [ ] Definite/indefinite integration (basic cases)
- [ ] Hypercomplex number support
- [ ] Matrix expressions
- [ ] Code generation (C, LLVM, WASM)

---

## Contributing

Contributions welcome! Priority areas:
1. Performance benchmarks and profiling
2. Additional simplification rules
3. Integration tests for edge cases

