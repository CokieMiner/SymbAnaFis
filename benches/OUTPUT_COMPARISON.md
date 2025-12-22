# Output Quality Comparison

**SymbAnaFis Version:** 0.4.0  
**Symbolica Version:** 1.2.1  
**Date:** 2025-12-22

This document compares the **output quality and readability** of symbolic differentiation results.

---

## Normal PDF: d/dx[exp(-(x-μ)²/(2σ²))/√(2πσ²)]

### SymbAnaFis
```
-exp(-(x - mu)^2/(2*sigma^2))*(x - mu)/(sqrt(2*pi)*sigma^2*abs(sigma))
```

### Symbolica
```
-sigma^-2*(x-mu)*exp(-1/2*sigma^-2*(x-mu)^2)*sqrt(2*sigma^2*pi)^-1
```

### Analysis
| Metric | SymbAnaFis | Symbolica |
|--------|------------|-----------|
| Fraction display | ✓ (standard division) | ✗ (negative powers) |
| Grouping | ✓ (compact fraction) | ✗ (spread out terms) |
| Readability | ★★★★★ | ★★★☆☆ |

---

## Gaussian 2D: d/dx[exp(-((x-x₀)²+(y-y₀)²)/(2s²))/(2πs²)]

### SymbAnaFis
```
-exp(-((x - x0)^2 + (y - y0)^2)/(2*s^2))*(x - x0)/(2*pi*s^4)
```

### Symbolica
```
-1/2*pi^-1*s^-4*(x-x0)*exp(-1/2*s^-2*((x-x0)^2+(y-y0)^2))
```

### Analysis
| Metric | SymbAnaFis | Symbolica |
|--------|------------|-----------|
| Format | Single Fraction | Product string |
| Constants | Gathered (2*pi*s^4) | Split (1/2*pi^-1*s^-4) |
| Readability | ★★★★★ | ★★★☆☆ |

---

## Lennard-Jones: d/dr[4ε((σ/r)¹² - (σ/r)⁶)]

### SymbAnaFis
```
24*epsilon*sigma*(1 - 2*(sigma/r)^6)*(sigma/r)^5/r^2
```

### Symbolica
```
4*epsilon*(6*sigma^6*r^-7-12*sigma^12*r^-13)
```

### Analysis
| Metric | SymbAnaFis | Symbolica |
|--------|------------|-----------|
| Structure preservation | ✓ (factors out common terms) | ✗ (expands to canonical polynomial) |
| Compactness | ★★★★☆ | ★★★★★ (Canonical form is very explicit) |
| Readability | ★★★★☆ | ★★★☆☆ |

---

## Lorentz Factor: d/dv[1/√(1-v²/c²)]

### SymbAnaFis
```
v*abs(c)/((-v + c)*(v + c)*sqrt((-v + c)*(v + c)))
```

### Symbolica
```
2*v*c^-2*((-v^2*c^-2+1)^(1/2))^(-1/2)*sqrt(-v^2*c^-2+1)^-2
```

### Analysis
| Metric | SymbAnaFis | Symbolica |
|--------|------------|-----------|
| Factorization | High (Difference of squares) | Mathematical canon |
| Integer simplification | ✓ | ✗ (Keeps (-v^2*c^-2+1)^(1/2))^(-1/2)) |
| Readability | ★★★★☆ | ★★☆☆☆ |

---

## Logistic Sigmoid: d/dx[1/(1+exp(-k(x-x₀)))]

### SymbAnaFis
```
exp(-k*(x - x0))*k/(1 + exp(-k*(x - x0)))^2
```

### Symbolica
```
k*(exp(-k*(x-x0))+1)^-2*exp(-k*(x-x0))
```

### Analysis
| Metric | SymbAnaFis | Symbolica |
|--------|------------|-----------|
| Form | Standard Ratio | Product with Negative Power |
| Readability | ★★★★★ | ★★★★☆ |

---

## Damped Oscillator: d/dt[A·exp(-γt)·cos(ωt+φ)]

### SymbAnaFis
```
-A*exp(-gamma*t)*(gamma*cos(omega*t + phi) + omega*sin(omega*t + phi))
```

### Symbolica
```
-A*gamma*exp(-gamma*t)*cos(phi+t*omega)-A*omega*exp(-gamma*t)*sin(phi+t*omega)
```

### Analysis
| Metric | SymbAnaFis | Symbolica |
|--------|------------|-----------|
| Factoring | ✓ (Factors out exponential) | ✗ (Fully expanded) |
| Grouping | ✓ (Groups sine/cosine) | ✗ |
| Readability | ★★★★★ | ★★★☆☆ |

---

## Maxwell-Boltzmann: d/dv[4π(m/(2πkT))^(3/2) v² exp(-mv²/(2kT))]

### SymbAnaFis
```
4*exp(-m*v^2/(2*k*T))*pi*(-m*v^3 + 2*T*k*v)*(m/(2*pi*k*T))^(3/2)/(T*k)
```

### Symbolica
```
8*pi*v*(1/2*pi^-1*m*k^-1*T^-1)^(3/2)*exp(-1/2*m*k^-1*T^-1*v^2)-4*pi*m*k^-1*T^-1*v^3*(1/2*pi^-1*m*k^-1*T^-1)^(3/2)*exp(-1/2*m*k^-1*T^-1*v^2)
```

### Analysis
| Metric | SymbAnaFis | Symbolica |
|--------|------------|-----------|
| Compactness | High (Factored common terms) | Low (Expanded into two large terms) |
| Readability | ★★★★☆ | ★★☆☆☆ |

---

## Planck Blackbody: d/dν[(2hν³/c²) / (exp(hν/kT)-1)]

### SymbAnaFis
```
2*(3*T*h*k*(-1 + exp(h*nu/(k*T)))^2*(c^2*nu)^2 - c^4*exp(h*nu/(k*T))*h^2*nu^3*(-1 + exp(h*nu/(k*T))))/(T*c^6*k*(-1 + exp(h*nu/(k*T)))^3)
```

### Symbolica
```
6*c^-2*h*nu^2*(exp(k^-1*T^-1*h*nu)-1)^-1-2*k^-1*T^-1*c^-2*h^2*nu^3*(exp(k^-1*T^-1*h*nu)-1)^-2*exp(k^-1*T^-1*h*nu)
```

### Analysis
| Metric | SymbAnaFis | Symbolica |
|--------|------------|-----------|
| Strategy | Quotient Rule (Common Denominator) | Product Rule (Sum of terms) |
| Expansion | High (Combined fraction is large) | Moderate (Two separate terms) |
| Readability | ★★☆☆☆ | ★★★☆☆ (Slightly cleaner due to separation) |

---

## Summary: Output Quality

| Expression | SymbAnaFis Style | Symbolica Style | Preference |
|------------|------------------|-----------------|------------|
| Normal PDF | Fraction | Negative Powers | SymbAnaFis |
| Gaussian 2D | Fraction | Negative Powers | SymbAnaFis |
| Lennard-Jones | Factored | Expanded Poly | SymbAnaFis |
| Lorentz Factor | Factored (Squares) | Canonical | Hybrid |
| Logistic Sigmoid | Standard | Product | SymbAnaFis |
| Damped Oscillator | Factored | Expanded | SymbAnaFis |
| Maxwell-Boltzmann | Factored | Expanded | SymbAnaFis |
| Planck Blackbody | Single Large Fraction | Separate Terms | Symbolica |

## Conclusion

SymbAnaFis consistently produces output closer to "textbook" format by:
1. Preferring **fractions** over negative powers (e.g., `a/b` vs `a*b^-1`).
2. **Factoring out** common terms (especially exponentials in physics formulas).
3. Maintaining **grouped structures** rather than fully expanding to canonical forms.

However, for very complex quotients (like Planck's Law), SymbAnaFis's tendency to combine everything into a single numerator/denominator can lead to verbose expressions compared to summing separate terms.
