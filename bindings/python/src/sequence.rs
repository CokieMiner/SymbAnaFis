//! Cached native plans with per-invocation Python mapping and scoring bridges.

use std::sync::Arc;

use pyo3::{
    Py, PyAny, PyResult, PyTraverseError, PyVisit, Python,
    exceptions::{PyTypeError, PyValueError},
    pyclass, pymethods,
    types::PyAnyMethods,
};
use symb_anafis::{
    DomainSafety, SimplificationSequence, SimplificationStep, SimplificationTarget,
    SimplificationTermination,
};

use super::{Callbacks, NativeExpr, NativeRule, SimplifyError};

/// Presets and template rules retain their compiled plans across calls.
#[pyclass(name = "_Sequence", module = "symb_anafis._native")]
#[derive(Debug)]
pub struct NativeSequence {
    configuration: SimplificationSequence,
    steps: Vec<Step>,
    cached: Option<SimplificationSequence>,
    score: Option<Py<PyAny>>,
}

#[derive(Debug)]
enum Step {
    Ready(SimplificationStep),
    Rule(NativeRule),
    Map(Py<PyAny>),
}

impl NativeSequence {
    fn builtin(name: &str) -> PyResult<SimplificationStep> {
        match name {
            "algebraic" => Ok(SimplificationStep::Algebraic),
            "trigonometric" => Ok(SimplificationStep::Trigonometric),
            "hyperbolic" => Ok(SimplificationStep::Hyperbolic),
            "exponential" => Ok(SimplificationStep::Exponential),
            "roots" => Ok(SimplificationStep::Roots),
            "expand" => Ok(SimplificationStep::Expand),
            "factor" => Ok(SimplificationStep::Factor),
            "compilation" => Ok(SimplificationStep::Compilation),
            _ => Err(PyValueError::new_err("unknown simplification step")),
        }
    }

    fn prepare(&self, state: &Arc<Callbacks>, py: Python<'_>) -> PyResult<SimplificationSequence> {
        let mut sequence = if let Some(cached) = &self.cached {
            cached.clone()
        } else {
            let mut sequence = self.configuration.clone();
            for step in &self.steps {
                let operation = match step {
                    Step::Ready(ready) => ready.clone(),
                    Step::Rule(rule) => SimplificationStep::Rewrite(rule.prepare(state, py)?),
                    Step::Map(callback) => {
                        let callable = callback.clone_ref(py);
                        let errors = Arc::clone(state);
                        SimplificationStep::Map(Arc::new(move |expression| {
                            errors.mapping(&callable, expression)
                        }))
                    }
                };
                sequence = sequence.then(operation);
            }
            sequence
        };
        if let Some(score) = &self.score {
            let callable = score.clone_ref(py);
            let errors = Arc::clone(state);
            sequence = sequence.score_with(move |expression| errors.score(&callable, expression));
        }
        Ok(sequence)
    }
}

#[pymethods]
impl NativeSequence {
    #[new]
    fn new(
        preset: &str,
        target: &str,
        domain_safety: &str,
        limits: (usize, usize, usize, usize, usize),
        match_limit: Option<usize>,
    ) -> PyResult<Self> {
        let initial = match preset {
            "empty" => SimplificationSequence::new(),
            "compact" => SimplificationSequence::compact(),
            "expanded" => SimplificationSequence::expanded(),
            "factored" => SimplificationSequence::factored(),
            "compilation" => SimplificationSequence::compilation(),
            _ => return Err(PyValueError::new_err("unknown simplification preset")),
        };
        let objective = match target {
            "compact" => SimplificationTarget::Compact,
            "expanded" => SimplificationTarget::Expanded,
            "factored" => SimplificationTarget::Factored,
            "compilation" => SimplificationTarget::Compilation,
            _ => return Err(PyValueError::new_err("unknown simplification target")),
        };
        let safety = match domain_safety {
            "preserving" => DomainSafety::Preserving,
            "assumption_dependent" => DomainSafety::AssumptionDependent,
            _ => return Err(PyValueError::new_err("unknown domain safety contract")),
        };
        let configuration = SimplificationSequence::new()
            .target(objective)
            .domain_safety(safety)
            .max_iterations(limits.0)
            .max_nodes(limits.1)
            .max_depth(limits.2)
            .expansion_limit(limits.3)
            .polynomial_limit(limits.4)
            .match_limit(match_limit);
        let mut cached = configuration.clone();
        for step in initial.steps() {
            cached = cached.then(step.clone());
        }
        Ok(Self {
            configuration,
            steps: initial.steps().iter().cloned().map(Step::Ready).collect(),
            cached: Some(cached),
            score: None,
        })
    }

    fn copy(&self, py: Python<'_>) -> Self {
        // Every tracked owner gets a real Python reference. Sharing an Arc<Py>
        // across GC-tracked containers would overcount edges during traversal.
        let steps = self
            .steps
            .iter()
            .map(|step| match step {
                Step::Ready(ready) => Step::Ready(ready.clone()),
                Step::Rule(rule) => Step::Rule(rule.clone_ref(py)),
                Step::Map(callback) => Step::Map(callback.clone_ref(py)),
            })
            .collect();
        Self {
            configuration: self.configuration.clone(),
            steps,
            cached: self.cached.clone(),
            score: self.score.as_ref().map(|callback| callback.clone_ref(py)),
        }
    }

    fn steps(&self) -> Vec<&'static str> {
        self.steps
            .iter()
            .map(|step| match step {
                Step::Ready(SimplificationStep::Algebraic) => "algebraic",
                Step::Ready(SimplificationStep::Trigonometric) => "trigonometric",
                Step::Ready(SimplificationStep::Hyperbolic) => "hyperbolic",
                Step::Ready(SimplificationStep::Exponential) => "exponential",
                Step::Ready(SimplificationStep::Roots) => "roots",
                Step::Ready(SimplificationStep::Expand) => "expand",
                Step::Ready(SimplificationStep::Factor) => "factor",
                Step::Ready(SimplificationStep::Compilation) => "compilation",
                Step::Ready(SimplificationStep::Rewrite(_)) | Step::Rule(_) => "rewrite",
                Step::Ready(SimplificationStep::Map(_)) | Step::Map(_) => "map",
                Step::Ready(_) => "unknown",
            })
            .collect()
    }

    fn then_builtin(&mut self, name: &str) -> PyResult<()> {
        let step = Self::builtin(name)?;
        if let Some(sequence) = self.cached.take() {
            self.cached = Some(sequence.then(step.clone()));
        }
        self.steps.push(Step::Ready(step));
        Ok(())
    }

    fn then_rule(&mut self, rule: &NativeRule, py: Python<'_>) {
        if let Some(sequence) = self.cached.take() {
            self.cached = rule.compiled().map(|operation| sequence.then(operation));
        }
        self.steps.push(Step::Rule(rule.clone_ref(py)));
    }

    fn then_map(&mut self, callback: Py<PyAny>, py: Python<'_>) -> PyResult<()> {
        if !callback.bind(py).is_callable() {
            return Err(PyTypeError::new_err("mapping step must be callable"));
        }
        self.cached = None;
        self.steps.push(Step::Map(callback));
        Ok(())
    }

    fn score_with(&mut self, callback: Py<PyAny>, py: Python<'_>) -> PyResult<()> {
        if !callback.bind(py).is_callable() {
            return Err(PyTypeError::new_err("score must be callable"));
        }
        self.score = Some(callback);
        Ok(())
    }

    fn simplify(
        &self,
        expression: &NativeExpr,
        py: Python<'_>,
    ) -> PyResult<(NativeExpr, &'static str, usize)> {
        let state = Arc::new(Callbacks::default());
        let sequence = self.prepare(&state, py)?;
        let result = py.detach(|| sequence.simplify(expression.value()));
        state.finish()?;
        let result = result.map_err(|error| SimplifyError::new_err(error.to_string()))?;
        let termination = match result.termination() {
            SimplificationTermination::FixedPoint => "fixed_point",
            SimplificationTermination::Cycle => "cycle",
            SimplificationTermination::IterationLimit | _ => "iteration_limit",
        };
        let iterations = result.iterations();
        Ok((result.into_expression().into(), termination, iterations))
    }

    fn __traverse__(&self, visitor: PyVisit<'_>) -> Result<(), PyTraverseError> {
        visitor.call(&self.score)?;
        for step in &self.steps {
            match step {
                Step::Rule(rule) => rule.visit(&visitor)?,
                Step::Map(callback) => visitor.call(callback)?,
                Step::Ready(_) => {}
            }
        }
        Ok(())
    }

    fn __clear__(&mut self) {
        self.score = None;
        self.steps.clear();
        self.cached = Some(self.configuration.clone());
    }
}
