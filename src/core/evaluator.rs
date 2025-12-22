//! Compiled expression evaluator for fast numerical evaluation
//!
//! Converts an expression tree to flat bytecode that can be evaluated
//! efficiently without tree traversal. Thread-safe for parallel evaluation.
//!
//! # Example
//! ```
//! use symb_anafis::parse;
//! use std::collections::HashSet;
//!
//! let expr = parse("sin(x) * cos(x) + x^2", &HashSet::new(), &HashSet::new(), None).unwrap();
//! let evaluator = expr.compile().unwrap();
//!
//! // Evaluate at x = 0.5
//! let result = evaluator.evaluate(&[0.5]);
//! assert!((result - (0.5_f64.sin() * 0.5_f64.cos() + 0.25)).abs() < 1e-10);
//! ```

use crate::core::traits::EPSILON;
use crate::{Expr, ExprKind};
use std::collections::HashMap;
use std::sync::Arc;

/// Error during expression compilation
#[derive(Debug, Clone)]
pub enum CompileError {
    /// Expression contains unsupported constructs for numeric evaluation
    UnsupportedExpression(String),
    /// Function not supported in compiled evaluation
    UnsupportedFunction(String),
    /// Variable not found in parameter list
    UnboundVariable(String),
}

impl std::fmt::Display for CompileError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            CompileError::UnsupportedExpression(msg) => {
                write!(f, "Unsupported expression: {}", msg)
            }
            CompileError::UnsupportedFunction(name) => {
                write!(f, "Unsupported function for evaluation: {}", name)
            }
            CompileError::UnboundVariable(name) => {
                write!(f, "Unbound variable: {}", name)
            }
        }
    }
}

impl std::error::Error for CompileError {}

/// Bytecode instruction for stack-based evaluation
#[derive(Clone, Copy, Debug)]
pub(crate) enum Instruction {
    /// Push a constant value onto the stack
    LoadConst(f64),
    /// Push a parameter value onto the stack (by index)
    LoadParam(usize),

    // Arithmetic operations (pop operands, push result)
    Add,
    Mul,
    Div,
    Neg,
    Pow,

    // Trigonometric functions (unary)
    Sin,
    Cos,
    Tan,
    Asin,
    Acos,
    Atan,
    Cot,
    Sec,
    Csc,
    Acot,
    Asec,
    Acsc,

    // Hyperbolic functions (unary)
    Sinh,
    Cosh,
    Tanh,
    Asinh,
    Acosh,
    Atanh,
    Coth,
    Sech,
    Csch,
    Acoth,
    Asech,
    Acsch,

    // Exponential/Logarithmic (unary)
    Exp,
    Ln,
    Log10,
    Log2,
    Sqrt,
    Cbrt,

    // Special functions (unary)
    Abs,
    Signum,
    Floor,
    Ceil,
    Round,
    Erf,
    Erfc,
    Gamma,
    Digamma,
    Trigamma,
    Tetragamma,
    Sinc,
    LambertW,
    EllipticK,
    EllipticE,
    Zeta,
    ExpPolar,

    // Two-argument functions
    Atan2,
    BesselJ,
    BesselY,
    BesselI,
    BesselK,
    Polygamma,
    Beta,
    ZetaDeriv,
    Hermite,

    // Three-argument functions
    AssocLegendre,

    // Four-argument functions
    SphericalHarmonic,
}

/// Compiled expression evaluator - thread-safe, reusable
///
/// The evaluator holds immutable bytecode that can be shared across threads.
/// Each call to `evaluate` uses a thread-local or per-call stack.
#[derive(Clone)]
pub struct CompiledEvaluator {
    /// Bytecode instructions (immutable after compilation)
    instructions: Arc<[Instruction]>,
    /// Required stack depth for evaluation
    stack_size: usize,
    /// Parameter names in order (for mapping HashMap -> array)
    param_names: Arc<[String]>,
}

impl CompiledEvaluator {
    /// Compile an expression to bytecode
    ///
    /// The `param_order` specifies the order of parameters in the evaluation array.
    /// All variables in the expression must be in this list.
    pub fn compile(expr: &Expr, param_order: &[&str]) -> Result<Self, CompileError> {
        let mut compiler = Compiler::new(param_order);
        compiler.compile_expr(expr)?;

        Ok(Self {
            instructions: compiler.instructions.into(),
            stack_size: compiler.max_stack,
            param_names: param_order.iter().map(|s| s.to_string()).collect(),
        })
    }

    /// Compile an expression, automatically determining parameter order from variables
    pub fn compile_auto(expr: &Expr) -> Result<Self, CompileError> {
        let vars = expr.variables();
        let mut param_order: Vec<String> = vars
            .into_iter()
            .filter(|v| !matches!(v.as_str(), "pi" | "PI" | "Pi" | "e" | "E"))
            .collect();
        param_order.sort(); // Consistent ordering

        let param_refs: Vec<&str> = param_order.iter().map(|s| s.as_str()).collect();
        Self::compile(expr, &param_refs)
    }

    /// Fast evaluation - no allocations in hot path, no tree traversal
    ///
    /// # Parameters
    /// `params` - Parameter values in the same order as `param_names()`
    ///
    /// # Panics
    /// Panics if stack underflow (indicates compiler bug)
    #[inline]
    pub fn evaluate(&self, params: &[f64]) -> f64 {
        // Use a small inline buffer for common cases, heap for large expressions
        let mut stack: Vec<f64> = Vec::with_capacity(self.stack_size);

        for instr in self.instructions.iter() {
            match *instr {
                Instruction::LoadConst(c) => stack.push(c),
                Instruction::LoadParam(i) => stack.push(params[i]),

                // Binary operations
                Instruction::Add => {
                    let b = stack.pop().unwrap();
                    *stack.last_mut().unwrap() += b;
                }
                Instruction::Mul => {
                    let b = stack.pop().unwrap();
                    *stack.last_mut().unwrap() *= b;
                }
                Instruction::Div => {
                    let b = stack.pop().unwrap();
                    *stack.last_mut().unwrap() /= b;
                }
                Instruction::Pow => {
                    let exp = stack.pop().unwrap();
                    let base = stack.last_mut().unwrap();
                    *base = base.powf(exp);
                }

                // Unary operations
                Instruction::Neg => {
                    let top = stack.last_mut().unwrap();
                    *top = -*top;
                }

                // Trigonometric
                Instruction::Sin => *stack.last_mut().unwrap() = stack.last().unwrap().sin(),
                Instruction::Cos => *stack.last_mut().unwrap() = stack.last().unwrap().cos(),
                Instruction::Tan => *stack.last_mut().unwrap() = stack.last().unwrap().tan(),
                Instruction::Asin => *stack.last_mut().unwrap() = stack.last().unwrap().asin(),
                Instruction::Acos => *stack.last_mut().unwrap() = stack.last().unwrap().acos(),
                Instruction::Atan => *stack.last_mut().unwrap() = stack.last().unwrap().atan(),
                Instruction::Cot => *stack.last_mut().unwrap() = 1.0 / stack.last().unwrap().tan(),
                Instruction::Sec => *stack.last_mut().unwrap() = 1.0 / stack.last().unwrap().cos(),
                Instruction::Csc => *stack.last_mut().unwrap() = 1.0 / stack.last().unwrap().sin(),
                Instruction::Acot => {
                    let x = *stack.last().unwrap();
                    *stack.last_mut().unwrap() = if x.abs() < EPSILON {
                        std::f64::consts::PI / 2.0
                    } else if x > 0.0 {
                        (1.0 / x).atan()
                    } else {
                        (1.0 / x).atan() + std::f64::consts::PI
                    };
                }
                Instruction::Asec => {
                    let x = *stack.last().unwrap();
                    *stack.last_mut().unwrap() = (1.0 / x).acos();
                }
                Instruction::Acsc => {
                    let x = *stack.last().unwrap();
                    *stack.last_mut().unwrap() = (1.0 / x).asin();
                }

                // Hyperbolic
                Instruction::Sinh => *stack.last_mut().unwrap() = stack.last().unwrap().sinh(),
                Instruction::Cosh => *stack.last_mut().unwrap() = stack.last().unwrap().cosh(),
                Instruction::Tanh => *stack.last_mut().unwrap() = stack.last().unwrap().tanh(),
                Instruction::Asinh => *stack.last_mut().unwrap() = stack.last().unwrap().asinh(),
                Instruction::Acosh => *stack.last_mut().unwrap() = stack.last().unwrap().acosh(),
                Instruction::Atanh => *stack.last_mut().unwrap() = stack.last().unwrap().atanh(),
                Instruction::Coth => {
                    *stack.last_mut().unwrap() = 1.0 / stack.last().unwrap().tanh()
                }
                Instruction::Sech => {
                    *stack.last_mut().unwrap() = 1.0 / stack.last().unwrap().cosh()
                }
                Instruction::Csch => {
                    *stack.last_mut().unwrap() = 1.0 / stack.last().unwrap().sinh()
                }
                Instruction::Acoth => {
                    let x = *stack.last().unwrap();
                    *stack.last_mut().unwrap() = 0.5 * ((x + 1.0) / (x - 1.0)).ln();
                }
                Instruction::Asech => {
                    let x = *stack.last().unwrap();
                    *stack.last_mut().unwrap() = (1.0 / x).acosh();
                }
                Instruction::Acsch => {
                    let x = *stack.last().unwrap();
                    *stack.last_mut().unwrap() = (1.0 / x).asinh();
                }

                // Exponential/Logarithmic
                Instruction::Exp => *stack.last_mut().unwrap() = stack.last().unwrap().exp(),
                Instruction::Ln => *stack.last_mut().unwrap() = stack.last().unwrap().ln(),
                Instruction::Log10 => *stack.last_mut().unwrap() = stack.last().unwrap().log10(),
                Instruction::Log2 => *stack.last_mut().unwrap() = stack.last().unwrap().log2(),
                Instruction::Sqrt => *stack.last_mut().unwrap() = stack.last().unwrap().sqrt(),
                Instruction::Cbrt => *stack.last_mut().unwrap() = stack.last().unwrap().cbrt(),

                // Special functions (unary)
                Instruction::Abs => *stack.last_mut().unwrap() = stack.last().unwrap().abs(),
                Instruction::Signum => *stack.last_mut().unwrap() = stack.last().unwrap().signum(),
                Instruction::Floor => *stack.last_mut().unwrap() = stack.last().unwrap().floor(),
                Instruction::Ceil => *stack.last_mut().unwrap() = stack.last().unwrap().ceil(),
                Instruction::Round => *stack.last_mut().unwrap() = stack.last().unwrap().round(),

                Instruction::Erf => {
                    *stack.last_mut().unwrap() = crate::math::eval_erf(*stack.last().unwrap())
                }
                Instruction::Erfc => {
                    *stack.last_mut().unwrap() = 1.0 - crate::math::eval_erf(*stack.last().unwrap())
                }
                Instruction::Gamma => {
                    *stack.last_mut().unwrap() =
                        crate::math::eval_gamma(*stack.last().unwrap()).unwrap_or(f64::NAN)
                }
                Instruction::Digamma => {
                    *stack.last_mut().unwrap() =
                        crate::math::eval_digamma(*stack.last().unwrap()).unwrap_or(f64::NAN)
                }
                Instruction::Trigamma => {
                    *stack.last_mut().unwrap() =
                        crate::math::eval_trigamma(*stack.last().unwrap()).unwrap_or(f64::NAN)
                }
                Instruction::Tetragamma => {
                    *stack.last_mut().unwrap() =
                        crate::math::eval_polygamma(3, *stack.last().unwrap()).unwrap_or(f64::NAN)
                }
                Instruction::Sinc => {
                    let x = *stack.last().unwrap();
                    *stack.last_mut().unwrap() = if x.abs() < EPSILON { 1.0 } else { x.sin() / x };
                }
                Instruction::LambertW => {
                    *stack.last_mut().unwrap() =
                        crate::math::eval_lambert_w(*stack.last().unwrap()).unwrap_or(f64::NAN)
                }
                Instruction::EllipticK => {
                    *stack.last_mut().unwrap() =
                        crate::math::eval_elliptic_k(*stack.last().unwrap()).unwrap_or(f64::NAN)
                }
                Instruction::EllipticE => {
                    *stack.last_mut().unwrap() =
                        crate::math::eval_elliptic_e(*stack.last().unwrap()).unwrap_or(f64::NAN)
                }
                Instruction::Zeta => {
                    *stack.last_mut().unwrap() =
                        crate::math::eval_zeta_deriv(0, *stack.last().unwrap()).unwrap_or(f64::NAN)
                }
                Instruction::ExpPolar => {
                    *stack.last_mut().unwrap() = crate::math::eval_exp_polar(*stack.last().unwrap())
                }

                // Two-argument functions
                Instruction::Atan2 => {
                    let x = stack.pop().unwrap();
                    let y = stack.last_mut().unwrap();
                    *y = y.atan2(x);
                }
                Instruction::BesselJ => {
                    let x = stack.pop().unwrap();
                    let n = stack.last_mut().unwrap();
                    *n = crate::math::bessel_j((*n).round() as i32, x).unwrap_or(f64::NAN);
                }
                Instruction::BesselY => {
                    let x = stack.pop().unwrap();
                    let n = stack.last_mut().unwrap();
                    *n = crate::math::bessel_y((*n).round() as i32, x).unwrap_or(f64::NAN);
                }
                Instruction::BesselI => {
                    let x = stack.pop().unwrap();
                    let n = stack.last_mut().unwrap();
                    *n = crate::math::bessel_i((*n).round() as i32, x).unwrap_or(f64::NAN);
                }
                Instruction::BesselK => {
                    let x = stack.pop().unwrap();
                    let n = stack.last_mut().unwrap();
                    *n = crate::math::bessel_k((*n).round() as i32, x).unwrap_or(f64::NAN);
                }
                Instruction::Polygamma => {
                    let x = stack.pop().unwrap();
                    let n = stack.last_mut().unwrap();
                    *n = crate::math::eval_polygamma((*n).round() as i32, x).unwrap_or(f64::NAN);
                }
                Instruction::Beta => {
                    let b = stack.pop().unwrap();
                    let a = stack.last_mut().unwrap();
                    let ga = crate::math::eval_gamma(*a);
                    let gb = crate::math::eval_gamma(b);
                    let gab = crate::math::eval_gamma(*a + b);
                    *a = match (ga, gb, gab) {
                        (Some(ga), Some(gb), Some(gab)) => ga * gb / gab,
                        _ => f64::NAN,
                    };
                }
                Instruction::ZetaDeriv => {
                    let s = stack.pop().unwrap();
                    let n = stack.last_mut().unwrap();
                    *n = crate::math::eval_zeta_deriv((*n).round() as i32, s).unwrap_or(f64::NAN);
                }
                Instruction::Hermite => {
                    let x = stack.pop().unwrap();
                    let n = stack.last_mut().unwrap();
                    *n = crate::math::eval_hermite((*n).round() as i32, x).unwrap_or(f64::NAN);
                }

                // Three-argument functions
                Instruction::AssocLegendre => {
                    let x = stack.pop().unwrap();
                    let m = stack.pop().unwrap();
                    let l = stack.last_mut().unwrap();
                    *l = crate::math::eval_assoc_legendre((*l).round() as i32, m.round() as i32, x)
                        .unwrap_or(f64::NAN);
                }

                // Four-argument functions
                Instruction::SphericalHarmonic => {
                    let phi = stack.pop().unwrap();
                    let theta = stack.pop().unwrap();
                    let m = stack.pop().unwrap();
                    let l = stack.last_mut().unwrap();
                    *l = crate::math::eval_spherical_harmonic(
                        (*l).round() as i32,
                        m.round() as i32,
                        theta,
                        phi,
                    )
                    .unwrap_or(f64::NAN);
                }
            }
        }

        stack.pop().unwrap_or(f64::NAN)
    }

    /// Get parameter names in order
    #[inline]
    pub fn param_names(&self) -> &[String] {
        &self.param_names
    }

    /// Get number of parameters
    #[inline]
    pub fn param_count(&self) -> usize {
        self.param_names.len()
    }

    /// Get number of bytecode instructions (for debugging/profiling)
    #[inline]
    pub fn instruction_count(&self) -> usize {
        self.instructions.len()
    }
}

/// Internal compiler state
struct Compiler<'a> {
    instructions: Vec<Instruction>,
    param_map: HashMap<&'a str, usize>,
    current_stack: usize,
    max_stack: usize,
}

impl<'a> Compiler<'a> {
    fn new(param_order: &[&'a str]) -> Self {
        let param_map: HashMap<&str, usize> = param_order
            .iter()
            .enumerate()
            .map(|(i, name)| (*name, i))
            .collect();

        Self {
            instructions: Vec::with_capacity(64),
            param_map,
            current_stack: 0,
            max_stack: 0,
        }
    }

    fn push(&mut self) {
        self.current_stack += 1;
        self.max_stack = self.max_stack.max(self.current_stack);
    }

    fn pop(&mut self) {
        self.current_stack = self.current_stack.saturating_sub(1);
    }

    fn emit(&mut self, instr: Instruction) {
        self.instructions.push(instr);
    }

    fn compile_expr(&mut self, expr: &Expr) -> Result<(), CompileError> {
        match &expr.kind {
            ExprKind::Number(n) => {
                self.emit(Instruction::LoadConst(*n));
                self.push();
            }

            ExprKind::Symbol(s) => {
                let name = s.as_str();
                // Handle known constants
                match name {
                    "pi" | "PI" | "Pi" => {
                        self.emit(Instruction::LoadConst(std::f64::consts::PI));
                        self.push();
                    }
                    "e" | "E" => {
                        self.emit(Instruction::LoadConst(std::f64::consts::E));
                        self.push();
                    }
                    _ => {
                        // Look up in parameter map
                        if let Some(&idx) = self.param_map.get(name) {
                            self.emit(Instruction::LoadParam(idx));
                            self.push();
                        } else {
                            return Err(CompileError::UnboundVariable(name.to_string()));
                        }
                    }
                }
            }

            ExprKind::Sum(terms) => {
                if terms.is_empty() {
                    self.emit(Instruction::LoadConst(0.0));
                    self.push();
                } else {
                    // Compile first term
                    self.compile_expr(&terms[0])?;
                    // Add remaining terms
                    for term in &terms[1..] {
                        self.compile_expr(term)?;
                        self.emit(Instruction::Add);
                        self.pop(); // Two operands -> one result
                    }
                }
            }

            ExprKind::Product(factors) => {
                if factors.is_empty() {
                    self.emit(Instruction::LoadConst(1.0));
                    self.push();
                } else {
                    // Check for negation pattern: Product([-1, x]) = -x
                    if factors.len() == 2
                        && let ExprKind::Number(n) = &factors[0].kind
                        && *n == -1.0
                    {
                        self.compile_expr(&factors[1])?;
                        self.emit(Instruction::Neg);
                        return Ok(());
                    }

                    // Compile first factor
                    self.compile_expr(&factors[0])?;
                    // Multiply remaining factors
                    for factor in &factors[1..] {
                        self.compile_expr(factor)?;
                        self.emit(Instruction::Mul);
                        self.pop();
                    }
                }
            }

            ExprKind::Div(num, den) => {
                self.compile_expr(num)?;
                self.compile_expr(den)?;
                self.emit(Instruction::Div);
                self.pop();
            }

            ExprKind::Pow(base, exp) => {
                self.compile_expr(base)?;
                self.compile_expr(exp)?;
                self.emit(Instruction::Pow);
                self.pop();
            }

            ExprKind::FunctionCall { name, args } => {
                let func_name = name.as_str();

                // Compile arguments first (in order for proper stack layout)
                for arg in args {
                    self.compile_expr(arg)?;
                }

                // Emit function instruction
                let instr = match (func_name, args.len()) {
                    // Trigonometric (unary)
                    ("sin", 1) => Instruction::Sin,
                    ("cos", 1) => Instruction::Cos,
                    ("tan", 1) => Instruction::Tan,
                    ("asin", 1) => Instruction::Asin,
                    ("acos", 1) => Instruction::Acos,
                    ("atan", 1) => Instruction::Atan,
                    ("cot", 1) => Instruction::Cot,
                    ("sec", 1) => Instruction::Sec,
                    ("csc", 1) => Instruction::Csc,
                    ("acot", 1) => Instruction::Acot,
                    ("asec", 1) => Instruction::Asec,
                    ("acsc", 1) => Instruction::Acsc,

                    // Hyperbolic (unary)
                    ("sinh", 1) => Instruction::Sinh,
                    ("cosh", 1) => Instruction::Cosh,
                    ("tanh", 1) => Instruction::Tanh,
                    ("asinh", 1) => Instruction::Asinh,
                    ("acosh", 1) => Instruction::Acosh,
                    ("atanh", 1) => Instruction::Atanh,
                    ("coth", 1) => Instruction::Coth,
                    ("sech", 1) => Instruction::Sech,
                    ("csch", 1) => Instruction::Csch,
                    ("acoth", 1) => Instruction::Acoth,
                    ("asech", 1) => Instruction::Asech,
                    ("acsch", 1) => Instruction::Acsch,

                    // Exponential/Logarithmic (unary)
                    ("exp", 1) => Instruction::Exp,
                    ("ln", 1) | ("log", 1) => Instruction::Ln,
                    ("log10", 1) => Instruction::Log10,
                    ("log2", 1) => Instruction::Log2,
                    ("sqrt", 1) => Instruction::Sqrt,
                    ("cbrt", 1) => Instruction::Cbrt,

                    // Special functions (unary)
                    ("abs", 1) => Instruction::Abs,
                    ("signum", 1) => Instruction::Signum,
                    ("floor", 1) => Instruction::Floor,
                    ("ceil", 1) => Instruction::Ceil,
                    ("round", 1) => Instruction::Round,
                    ("erf", 1) => Instruction::Erf,
                    ("erfc", 1) => Instruction::Erfc,
                    ("gamma", 1) => Instruction::Gamma,
                    ("digamma", 1) => Instruction::Digamma,
                    ("trigamma", 1) => Instruction::Trigamma,
                    ("tetragamma", 1) => Instruction::Tetragamma,
                    ("sinc", 1) => Instruction::Sinc,
                    ("lambertw", 1) => Instruction::LambertW,
                    ("elliptic_k", 1) => Instruction::EllipticK,
                    ("elliptic_e", 1) => Instruction::EllipticE,
                    ("zeta", 1) => Instruction::Zeta,
                    ("exp_polar", 1) => Instruction::ExpPolar,

                    // Two-argument functions
                    ("atan2", 2) => {
                        self.pop(); // Two args -> one result
                        Instruction::Atan2
                    }
                    ("besselj", 2) => {
                        self.pop();
                        Instruction::BesselJ
                    }
                    ("bessely", 2) => {
                        self.pop();
                        Instruction::BesselY
                    }
                    ("besseli", 2) => {
                        self.pop();
                        Instruction::BesselI
                    }
                    ("besselk", 2) => {
                        self.pop();
                        Instruction::BesselK
                    }
                    ("polygamma", 2) => {
                        self.pop();
                        Instruction::Polygamma
                    }
                    ("beta", 2) => {
                        self.pop();
                        Instruction::Beta
                    }
                    ("zeta_deriv", 2) => {
                        self.pop();
                        Instruction::ZetaDeriv
                    }
                    ("hermite", 2) => {
                        self.pop();
                        Instruction::Hermite
                    }

                    // Three-argument functions
                    ("assoc_legendre", 3) => {
                        self.pop();
                        self.pop();
                        Instruction::AssocLegendre
                    }

                    // Four-argument functions
                    ("spherical_harmonic", 4) | ("ynm", 4) => {
                        self.pop();
                        self.pop();
                        self.pop();
                        Instruction::SphericalHarmonic
                    }

                    _ => {
                        return Err(CompileError::UnsupportedFunction(func_name.to_string()));
                    }
                };

                self.emit(instr);
            }

            ExprKind::Poly(poly) => {
                // Polynomial evaluation using Horner's method for efficiency
                // P(x) = a_n*x^n + a_{n-1}*x^{n-1} + ... + a_0
                // Horner: ((a_n*x + a_{n-1})*x + ...)*x + a_0

                let terms = poly.terms();
                if terms.is_empty() {
                    self.emit(Instruction::LoadConst(0.0));
                    self.push();
                    return Ok(());
                }

                // Sort terms by power descending for Horner's method
                let mut sorted_terms: Vec<_> = terms.to_vec();
                sorted_terms.sort_by(|a, b| b.0.cmp(&a.0));

                // Get max power
                let max_pow = sorted_terms[0].0;

                // For simple polynomial with Symbol base, we can use the param directly
                // For complex bases, we'd need to compile them and store/reload
                // (Currently only handle simple Symbol case)
                let base_param_idx = if let ExprKind::Symbol(s) = &poly.base().kind {
                    let name = s.as_str();
                    match name {
                        "pi" | "PI" | "Pi" | "e" | "E" => None, // Constants, not params
                        _ => self.param_map.get(name).copied(),
                    }
                } else {
                    None
                };

                match base_param_idx {
                    Some(idx) => {
                        // Fast path: base is a simple parameter, use Horner's method
                        // Start with highest coefficient
                        self.emit(Instruction::LoadConst(sorted_terms[0].1));
                        self.push();

                        let mut term_iter = sorted_terms.iter().skip(1).peekable();

                        for pow in (0..max_pow).rev() {
                            // Multiply by x
                            self.emit(Instruction::LoadParam(idx));
                            self.push();
                            self.emit(Instruction::Mul);
                            self.pop();

                            // Add coefficient if this power exists
                            if term_iter.peek().is_some_and(|(p, _)| *p == pow) {
                                let (_, coeff) = term_iter.next().unwrap();
                                self.emit(Instruction::LoadConst(*coeff));
                                self.push();
                                self.emit(Instruction::Add);
                                self.pop();
                            }
                        }
                    }
                    None => {
                        // Slow path: expand the polynomial explicitly
                        // Evaluate as sum of coeff * base^power
                        let base = poly.base();

                        // First term
                        let (first_pow, first_coeff) = sorted_terms[0];
                        self.emit(Instruction::LoadConst(first_coeff));
                        self.push();
                        self.compile_expr(base)?;
                        self.emit(Instruction::LoadConst(first_pow as f64));
                        self.push();
                        self.emit(Instruction::Pow);
                        self.pop();
                        self.emit(Instruction::Mul);
                        self.pop();

                        // Remaining terms
                        for &(pow, coeff) in &sorted_terms[1..] {
                            self.emit(Instruction::LoadConst(coeff));
                            self.push();
                            self.compile_expr(base)?;
                            self.emit(Instruction::LoadConst(pow as f64));
                            self.push();
                            self.emit(Instruction::Pow);
                            self.pop();
                            self.emit(Instruction::Mul);
                            self.pop();
                            self.emit(Instruction::Add);
                            self.pop();
                        }
                    }
                }
            }

            ExprKind::Derivative { .. } => {
                return Err(CompileError::UnsupportedExpression(
                    "Derivatives cannot be numerically evaluated - simplify first".to_string(),
                ));
            }
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::parser;
    use std::collections::HashSet;

    fn parse_expr(s: &str) -> Expr {
        parser::parse(s, &HashSet::new(), &HashSet::new(), None).unwrap()
    }

    #[test]
    fn test_simple_arithmetic() {
        let expr = parse_expr("x + 2");
        let eval = CompiledEvaluator::compile(&expr, &["x"]).unwrap();
        assert!((eval.evaluate(&[3.0]) - 5.0).abs() < 1e-10);
    }

    #[test]
    fn test_polynomial() {
        let expr = parse_expr("x^2 + 2*x + 1");
        let eval = CompiledEvaluator::compile(&expr, &["x"]).unwrap();
        assert!((eval.evaluate(&[3.0]) - 16.0).abs() < 1e-10);
    }

    #[test]
    fn test_trig() {
        let expr = parse_expr("sin(x)^2 + cos(x)^2");
        let eval = CompiledEvaluator::compile(&expr, &["x"]).unwrap();
        // Should always equal 1
        assert!((eval.evaluate(&[0.5]) - 1.0).abs() < 1e-10);
        assert!((eval.evaluate(&[1.23]) - 1.0).abs() < 1e-10);
    }

    #[test]
    fn test_constants() {
        let expr = parse_expr("pi * e");
        let eval = CompiledEvaluator::compile(&expr, &[]).unwrap();
        let expected = std::f64::consts::PI * std::f64::consts::E;
        assert!((eval.evaluate(&[]) - expected).abs() < 1e-10);
    }

    #[test]
    fn test_multi_var() {
        let expr = parse_expr("x * y + z");
        let eval = CompiledEvaluator::compile(&expr, &["x", "y", "z"]).unwrap();
        assert!((eval.evaluate(&[2.0, 3.0, 4.0]) - 10.0).abs() < 1e-10);
    }

    #[test]
    fn test_unbound_variable_error() {
        let expr = parse_expr("x + y");
        let result = CompiledEvaluator::compile(&expr, &["x"]);
        assert!(matches!(result, Err(CompileError::UnboundVariable(_))));
    }

    #[test]
    fn test_compile_auto() {
        let expr = parse_expr("x^2 + y");
        let eval = CompiledEvaluator::compile_auto(&expr).unwrap();
        // Auto compilation sorts parameters alphabetically
        assert_eq!(eval.param_names(), &["x", "y"]);
    }
}
