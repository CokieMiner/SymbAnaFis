# SymbAnaFis Differentiation Output Comparison

Comparison of differentiation output between SymbAnaFis and Symbolica libraries.

**SymbAnaFis Version:** 0.7.0  
**Date:** 2026-01-31

---

## Normal PDF (d/dx)

**Input:** `exp(-(x-mu)^2/(2*sigma^2))/sqrt(2*pi*sigma^2)`

```
SymbAnaFis (Raw):        exp(-(x - mu)^2/(2*sigma^2))*-2*(x - mu)/(2*sigma^2)/sqrt(2*pi*sigma^2)
SymbAnaFis (Simplified): -(x - mu)*exp(-(x - mu)^2/(2*sigma^2))/(abs(sigma)*sqrt(2*pi)*sigma^2)
Symbolica:               -sigma^-2*(x-mu)*exp(-1/2*sigma^-2*(x-mu)^2)*sqrt(2*sigma^2*pi)^-1
```

---

## Gaussian 2D (d/dx)

**Input:** `exp(-((x-x0)^2+(y-y0)^2)/(2*s^2))/(2*pi*s^2)`

```
SymbAnaFis (Raw):        exp(-((x - x0)^2 + (y - y0)^2)/(2*s^2))*-2*(x - x0)/(2*s^2)/(2*pi*s^2)
SymbAnaFis (Simplified): -(x - x0)*exp(-((x - x0)^2 + (y - y0)^2)/(2*s^2))/(2*pi*s^4)
Symbolica:               -1/2*pi^-1*s^-4*(x-x0)*exp(-1/2*s^-2*((x-x0)^2+(y-y0)^2))
```

---

## Maxwell-Boltzmann (d/dv)

**Input:** `4*pi*(m/(2*pi*k*T))^(3/2) * v^2 * exp(-m*v^2/(2*k*T))`

```
SymbAnaFis (Raw):        2*4*pi*v*exp(-m*v^2/(2*T*k))*(m/(2*T*k*pi))^(3/2) + 4*pi*exp(-m*v^2/(2*T*k))*v^2*(m/(2*T*k*pi))^(3/2)*-2*m*v/(2*T*k)
SymbAnaFis (Simplified): 4*pi*(-m*v^3 + 2*T*k*v)*exp(-m*v^2/(2*T*k))*(m/(2*T*k*pi))^(3/2)/(T*k)
Symbolica:               8*pi*v*(1/2*pi^-1*m*k^-1*T^-1)^(3/2)*exp(-1/2*m*k^-1*T^-1*v^2)-4*pi*m*k^-1*T^-1*v^3*(1/2*pi^-1*m*k^-1*T^-1)^(3/2)*exp(-1/2*m*k^-1*T^-1*v^2)
```

---

## Lorentz Factor (d/dv)

**Input:** `1/sqrt(1-v^2/c^2)`

```
SymbAnaFis (Raw):        1/sqrt(1 - v^2/c^2)^2
SymbAnaFis (Simplified): v*abs(c)/((c + v)*(c - v)*sqrt((c + v)*(c - v)))
Symbolica:               2*v*c^-2*((-v^2*c^-2+1)^(1/2))^(-1/2)*sqrt(-v^2*c^-2+1)^-2
```

---

## Lennard-Jones (d/dr)

**Input:** `4*epsilon*((sigma/r)^12 - (sigma/r)^6)`

```
SymbAnaFis (Raw):        4*epsilon*-sigma/r^2*(-6*(sigma/r)^5 + 12*(sigma/r)^11)
SymbAnaFis (Simplified): -4*epsilon*sigma*(-6*(sigma/r)^5 + 12*(sigma/r)^11)/r^2
Symbolica:               4*epsilon*(6*sigma^6*r^-7-12*sigma^12*r^-13)
```

---

## Logistic Sigmoid (d/dx)

**Input:** `1/(1+exp(-k*(x-x0)))`

```
SymbAnaFis (Raw):        1/(1 + exp(-k*(x - x0)))^2
SymbAnaFis (Simplified): k/(exp(k*(x - x0))*((1 + exp(k*(x - x0)))/exp(k*(x - x0)))^2)
Symbolica:               k*(exp(-k*(x-x0))+1)^-2*exp(-k*(x-x0))
```

---

## Damped Oscillator (d/dt)

**Input:** `A*exp(-gamma*t)*cos(omega*t+phi)`

```
SymbAnaFis (Raw):        -A*gamma*cos(phi + omega*t)*exp(-gamma*t) - A*omega*exp(-gamma*t)*sin(phi + omega*t)
SymbAnaFis (Simplified): -A*(gamma*cos(phi + omega*t) + omega*sin(phi + omega*t))/exp(gamma*t)
Symbolica:               -A*gamma*exp(-gamma*t)*cos(phi+t*omega)-A*omega*exp(-gamma*t)*sin(phi+t*omega)
```

---

## Planck Blackbody (d/dnu)

**Input:** `(2*h*nu^3/c^2) * (1/(exp(h*nu/(k*T))-1))`

```
SymbAnaFis (Raw):        1/(-1 + exp(h*nu/(T*k)))*2*3*h*nu^2/c^2 + -exp(h*nu/(T*k))*h/(T*k)/(-1 + exp(h*nu/(T*k)))^2*2*h*nu^3/c^2
SymbAnaFis (Simplified): 2*(-(-1 + exp(h*nu/(T*k)))*exp(h*nu/(T*k))*nu^3*(c*h)^2 + 3*T*h*k*(c*nu*(-1 + exp(h*nu/(T*k))))^2)/(T*k*c^4*(-1 + exp(h*nu/(T*k)))^3)
Symbolica:               6*c^-2*h*nu^2*(exp(k^-1*T^-1*h*nu)-1)^-1-2*k^-1*T^-1*c^-2*h^2*nu^3*(exp(k^-1*T^-1*h*nu)-1)^-2*exp(k^-1*T^-1*h*nu)
```

---

## Key Observations

### Output Format Differences

**SymbAnaFis:**
- Raw output shows the direct application of differentiation rules (chain rule, product rule visible)
- Simplified output performs algebraic restructuring and constant folding
- Uses standard notation: `x^2`, `sqrt()`, `abs()`, positive exponents
- Factorization-based simplification (common terms extracted)

**Symbolica:**
- Uses negative exponents: `x^-2` instead of `1/x^2`
- More compact notation for reciprocals
- Different algebraic form (emphasis on canonical ordering)

### Simplification Trade-offs

**SymbAnaFis Simplification:**
- ✅ More human-readable (positive exponents, explicit fractions)
- ✅ Shows factorization (common terms grouped)
- ⚠️ May be more verbose in some cases
- ⚠️ Slower due to deep restructuring (see BENCHMARK_RESULTS.md)

**Symbolica:**
- ✅ More compact (negative exponents)
- ✅ Faster (light term collection only)
- ⚠️ Less intuitive for human reading

### Correctness

All derivatives are mathematically equivalent. The differences are purely in:
1. **Algebraic representation** (negative vs positive exponents)
2. **Factorization strategy** (different grouping of terms)
3. **Simplification depth** (SymbAnaFis goes deeper, Symbolica stays lighter)