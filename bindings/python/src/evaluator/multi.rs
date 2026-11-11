//! Multi-output zero-copy PEP 3118 buffer evaluation with input/output aliasing safety.

#![expect(
    unsafe_code,
    reason = "zero-copy PEP 3118 borrowing of disjoint writable native output slices"
)]

use core::slice;

use pyo3::{
    Bound, IntoPyObjectExt, Py, PyAny, PyResult, Python,
    buffer::PyBuffer,
    exceptions::PyValueError,
    types::{PyAnyMethods, PyTuple},
};
use symb_anafis::{EvaluationValue, MultiProgram, Number, Workspace};

use super::{
    ColumnBuffer, EvaluationError, NativeElement, NativeNumber, alloc_output, ptrs_overlap,
};

type OutputPtrs<T> = Vec<(*mut <T as NativeElement>::BufferItem, *mut T)>;

struct WritableOutputs<'py, T: NativeElement> {
    objects: Vec<Bound<'py, PyAny>>,
    _buffers: Vec<PyBuffer<T::BufferItem>>,
    ptrs: OutputPtrs<T>,
}

/// Evaluate a multi-output exact `Number` scalar row, returning a Python tuple.
pub fn eval_number_scalar_multi(
    program: &MultiProgram<Number>,
    workspace: &mut Workspace<Number>,
    numbers: &[Number],
    py: Python<'_>,
) -> PyResult<Py<PyAny>> {
    let mut out = alloc_output(Number::zero(), program.output_count())?;
    py.detach(|| program.eval_with(workspace, numbers, &mut out))
        .map_err(|error| EvaluationError::new_err(error.to_string()))?;
    let py_items = out
        .into_iter()
        .map(|value| NativeNumber::from(value).into_py_any(py))
        .collect::<PyResult<Vec<_>>>()?;
    Ok(PyTuple::new(py, py_items)?.into_any().unbind())
}

/// Evaluate a multi-output exact `Number` batch, returning a Python tuple of lists.
pub fn eval_number_batch_multi(
    program: &MultiProgram<Number>,
    workspace: &mut Workspace<Number>,
    slices: &[&[Number]],
    row_count: usize,
    parallel: bool,
    py: Python<'_>,
) -> PyResult<Py<PyAny>> {
    let mut outputs: Vec<Vec<Number>> = (0..program.output_count())
        .map(|_| alloc_output(Number::zero(), row_count))
        .collect::<PyResult<Vec<_>>>()?;
    run_multi_batch_vecs(program, workspace, slices, &mut outputs, parallel, py)?;
    let py_lists = outputs
        .into_iter()
        .map(|col| {
            col.into_iter()
                .map(NativeNumber::from)
                .collect::<Vec<_>>()
                .into_py_any(py)
        })
        .collect::<PyResult<Vec<_>>>()?;
    Ok(PyTuple::new(py, py_lists)?.into_any().unbind())
}

/// Evaluate a multi-output scalar row for any `NativeElement`, returning a Python tuple.
pub fn eval_typed_scalar_multi<T: NativeElement>(
    program: &MultiProgram<T>,
    workspace: &mut Workspace<T>,
    values: &Bound<'_, PyAny>,
    py: Python<'_>,
) -> PyResult<Py<PyAny>> {
    let buffer = ColumnBuffer::<T>::extract(values)?;
    let slice = buffer.as_slice();
    let mut outputs = alloc_output(T::ZERO, program.output_count())?;
    py.detach(|| program.eval_with(workspace, slice, &mut outputs))
        .map_err(|error| EvaluationError::new_err(error.to_string()))?;
    let py_values = outputs
        .into_iter()
        .map(|value| value.into_py_scalar(py))
        .collect::<PyResult<Vec<_>>>()?;
    Ok(PyTuple::new(py, py_values)?.into_any().unbind())
}

/// Evaluate a multi-output batch for any `NativeElement` (`f64`, `f32`, `Complex<f64>`, `Complex<f32>`),
/// writing zero-copy into a sequence of writable `out` buffers when supplied and checking both
/// input-output and output-output aliasing before constructing mutable Rust slices.
pub fn eval_typed_batch_multi<T: NativeElement>(
    program: &MultiProgram<T>,
    workspace: &mut Workspace<T>,
    columns: &Bound<'_, PyAny>,
    row_count: usize,
    out: Option<&Bound<'_, PyAny>>,
    parallel: bool,
    py: Python<'_>,
) -> PyResult<Py<PyAny>> {
    let col_buffers = columns
        .try_iter()?
        .map(|col| ColumnBuffer::<T>::extract(&col?))
        .collect::<PyResult<Vec<_>>>()?;
    let slices: Vec<&[T]> = col_buffers.iter().map(ColumnBuffer::as_slice).collect();
    let n_outputs = program.output_count();

    if let Some(out_seq) = out {
        let writable = extract_writable_outputs::<T>(out_seq, n_outputs, row_count)?;
        if has_output_aliasing(&slices, &writable.ptrs, row_count) {
            let mut temps: Vec<Vec<T>> = (0..n_outputs)
                .map(|_| alloc_output(T::ZERO, row_count))
                .collect::<PyResult<Vec<_>>>()?;
            run_multi_batch_vecs(program, workspace, &slices, &mut temps, parallel, py)?;
            drop(slices);
            drop(col_buffers);
            for (&(items_ptr, _), temp) in writable.ptrs.iter().zip(&temps) {
                // SAFETY: each `out_buf` is an active, writable, aligned, C-contiguous buffer
                // with at least `row_count` elements, input borrows have ended, and outputs
                // are copied sequentially without simultaneous mutable aliasing.
                let target_items = unsafe { slice::from_raw_parts_mut(items_ptr, row_count) };
                T::slice_from_items_mut(target_items).copy_from_slice(temp);
            }
        } else {
            let mut targets: Vec<&mut [T]> = writable
                .ptrs
                .iter()
                .map(|&(items_ptr, _)| {
                    // SAFETY: each `out_buf` is active, writable, aligned, and C-contiguous
                    // with at least `row_count` elements, and `has_output_aliasing` proved all
                    // output ranges are pairwise disjoint and disjoint from every input slice.
                    let target_items = unsafe { slice::from_raw_parts_mut(items_ptr, row_count) };
                    T::slice_from_items_mut(target_items)
                })
                .collect();
            run_multi_batch_slices(program, workspace, &slices, &mut targets, parallel, py)?;
        }
        return Ok(PyTuple::new(py, writable.objects)?.into_any().unbind());
    }

    let mut outputs: Vec<Vec<T>> = (0..n_outputs)
        .map(|_| alloc_output(T::ZERO, row_count))
        .collect::<PyResult<Vec<_>>>()?;
    run_multi_batch_vecs(program, workspace, &slices, &mut outputs, parallel, py)?;
    let py_lists = outputs
        .into_iter()
        .map(|vec| T::into_py_list(vec, py))
        .collect::<PyResult<Vec<_>>>()?;
    Ok(PyTuple::new(py, py_lists)?.into_any().unbind())
}

fn run_multi_batch_vecs<T: EvaluationValue + Send + Sync>(
    program: &MultiProgram<T>,
    workspace: &mut Workspace<T>,
    slices: &[&[T]],
    outputs: &mut [Vec<T>],
    parallel: bool,
    py: Python<'_>,
) -> PyResult<()> {
    let mut out_slices: Vec<&mut [T]> = outputs.iter_mut().map(Vec::as_mut_slice).collect();
    run_multi_batch_slices(program, workspace, slices, &mut out_slices, parallel, py)
}

fn run_multi_batch_slices<T: EvaluationValue + Send + Sync>(
    program: &MultiProgram<T>,
    workspace: &mut Workspace<T>,
    slices: &[&[T]],
    targets: &mut [&mut [T]],
    parallel: bool,
    py: Python<'_>,
) -> PyResult<()> {
    py.detach(|| {
        if parallel {
            program.eval_batch_par(slices, targets)
        } else {
            program.eval_batch_with(workspace, slices, targets)
        }
    })
    .map_err(|error| EvaluationError::new_err(error.to_string()))
}

fn extract_writable_outputs<'py, T: NativeElement>(
    out_seq: &Bound<'py, PyAny>,
    n_outputs: usize,
    row_count: usize,
) -> PyResult<WritableOutputs<'py, T>> {
    let objects: Vec<Bound<'py, PyAny>> = out_seq.try_iter()?.collect::<PyResult<Vec<_>>>()?;
    if objects.len() != n_outputs {
        return Err(EvaluationError::new_err(format!(
            "expected {n_outputs} outputs, received {}",
            objects.len()
        )));
    }
    let mut buffers = Vec::with_capacity(n_outputs);
    let mut ptrs = Vec::with_capacity(n_outputs);
    for obj in &objects {
        let out_buf = PyBuffer::<T::BufferItem>::get(obj)?;
        if out_buf.readonly() {
            return Err(PyValueError::new_err("output buffer must be writable"));
        }
        if out_buf.dimensions() != 1 {
            return Err(PyValueError::new_err(
                "output buffer must be a 1-dimensional contiguous buffer",
            ));
        }
        let Some(ptr) = out_buf.as_slice_ptr() else {
            return Err(PyValueError::new_err(
                "output buffer must be C-contiguous and writable",
            ));
        };
        let out_count = out_buf.item_count();
        if out_count < row_count {
            return Err(PyValueError::new_err(format!(
                "output buffer size ({out_count}) is smaller than row_count ({row_count})"
            )));
        }
        let items_ptr = ptr.as_ptr().cast::<T::BufferItem>();
        ptrs.push((items_ptr, items_ptr.cast::<T>()));
        buffers.push(out_buf);
    }
    Ok(WritableOutputs {
        objects,
        _buffers: buffers,
        ptrs,
    })
}

fn has_output_aliasing<T: NativeElement>(
    inputs: &[&[T]],
    outputs: &OutputPtrs<T>,
    row_count: usize,
) -> bool {
    for (i, &(_, ptr_i)) in outputs.iter().enumerate() {
        if inputs
            .iter()
            .any(|col| ptrs_overlap(col.as_ptr(), col.len(), ptr_i, row_count))
        {
            return true;
        }
        if outputs
            .iter()
            .skip(i.saturating_add(1))
            .any(|&(_, ptr_j)| ptrs_overlap(ptr_i, row_count, ptr_j, row_count))
        {
            return true;
        }
    }
    false
}
