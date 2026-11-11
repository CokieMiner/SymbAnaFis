//! Zero-copy PEP 3118 buffers for real (`f64`/`f32`) and complex (`Complex<f64>`/`Complex<f32>`) scalars.

#![expect(
    unsafe_code,
    reason = "zero-copy PEP 3118 borrowing of contiguous native float and complex slices"
)]

use core::{ffi::CStr, mem::size_of, slice};

use pyo3::{
    Bound, IntoPyObjectExt, Py, PyAny, PyResult, Python,
    buffer::{Element, PyBuffer, PyUntypedBuffer},
    exceptions::{PyMemoryError, PyTypeError, PyValueError},
    sync::PyOnceLock,
    types::{PyAnyMethods, PyBool, PyComplex, PyComplexMethods, PyModule},
};
use symb_anafis::{Complex, EvaluationValue, Program, Workspace};

use super::EvaluationError;

static COMPLEX_ABC: PyOnceLock<Py<PyAny>> = PyOnceLock::new();
static REAL_ABC: PyOnceLock<Py<PyAny>> = PyOnceLock::new();

/// Transparent PEP 3118 buffer element matching `NumPy` `complex128` (`Zd`) and `complex64` (`Zf`).
///
/// `num_anafis::Complex<T>` is `#[repr(C)] { pub re: T, pub im: T }`, which has the exact
/// memory layout and alignment of C99 `_Complex` and `NumPy`'s complex dtypes.
#[repr(transparent)]
#[derive(Clone, Copy, Debug)]
pub struct ComplexBufferElem<T>(pub Complex<T>);

impl<T> ComplexBufferElem<T> {
    const fn as_complex_slice(items: &[Self]) -> &[Complex<T>] {
        // SAFETY: `Self` is `#[repr(transparent)]` over `Complex<T>`.
        unsafe { slice::from_raw_parts(items.as_ptr().cast::<Complex<T>>(), items.len()) }
    }

    const fn as_complex_slice_mut(items: &mut [Self]) -> &mut [Complex<T>] {
        // SAFETY: `Self` is `#[repr(transparent)]` over `Complex<T>`.
        unsafe { slice::from_raw_parts_mut(items.as_mut_ptr().cast::<Complex<T>>(), items.len()) }
    }
}

/// Native scalar backed by zero-copy PEP 3118 buffer borrowing (`f64`, `f32`, `Complex<f64>`, `Complex<f32>`).
pub trait NativeElement: Copy + EvaluationValue + Send + Sync + Sized {
    /// PEP 3118 buffer element type with identical layout to `Self`.
    type BufferItem: Element + Copy;

    /// Additive zero value used to initialize output buffers.
    const ZERO: Self;

    /// Whether this scalar accepts complex inputs.
    const IS_COMPLEX: bool;

    /// Reinterpret a borrowed buffer element slice as `&[Self]`.
    fn slice_from_items(items: &[Self::BufferItem]) -> &[Self];

    /// Reinterpret a mutable borrowed buffer element slice as `&mut [Self]`.
    fn slice_from_items_mut(items: &mut [Self::BufferItem]) -> &mut [Self];

    /// Extract one Python scalar, rejecting booleans (and complex values for real backends).
    fn extract_scalar(value: &Bound<'_, PyAny>) -> PyResult<Self>;

    /// Convert one scalar into its Python representation (`float` or `complex`).
    fn into_py_scalar(self, py: Python<'_>) -> PyResult<Py<PyAny>>;

    /// Convert an owned result vector into a Python list.
    fn into_py_list(values: Vec<Self>, py: Python<'_>) -> PyResult<Py<PyAny>>;
}

/// Borrowed zero-copy PEP 3118 buffer or materialized contiguous slice for one column/vector.
#[derive(Debug)]
pub enum ColumnBuffer<T: NativeElement> {
    /// Zero-copy borrowed Python buffer export.
    Buffer(PyBuffer<T::BufferItem>),
    /// Fallback materialized vector for Python sequences or non-matching buffer formats.
    Owned(Vec<T>),
}

/// Evaluate a batch for any `NativeElement` (`f64`, `f32`, `Complex<f64>`, `Complex<f32>`),
/// supporting zero-copy inputs, writable `out=` buffers, read-only rejection, and aliasing safety.
pub fn eval_typed_batch<T: NativeElement>(
    program: &Program<T>,
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

    if let Some(out_obj) = out {
        let out_buf = PyBuffer::<T::BufferItem>::get(out_obj)?;
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
        let raw_items_ptr = ptr.as_ptr().cast::<T::BufferItem>();
        let raw_t_ptr = raw_items_ptr.cast::<T>();
        let aliased = slices
            .iter()
            .any(|col| ptrs_overlap(col.as_ptr(), col.len(), raw_t_ptr, row_count));
        if aliased {
            let mut temp = alloc_output(T::ZERO, row_count)?;
            py.detach(|| {
                if parallel {
                    program.eval_batch_par(&slices, &mut temp)
                } else {
                    program.eval_batch_with(workspace, &slices, &mut temp)
                }
            })
            .map_err(|error| EvaluationError::new_err(error.to_string()))?;
            drop(slices);
            drop(col_buffers);
            // SAFETY: `out_buf` is an active, writable, aligned, C-contiguous buffer with at
            // least `row_count` elements, and all input buffer borrows have ended.
            let target_items = unsafe { slice::from_raw_parts_mut(raw_items_ptr, row_count) };
            T::slice_from_items_mut(target_items).copy_from_slice(&temp);
        } else {
            // SAFETY: `out_buf` is an active, writable, aligned, C-contiguous buffer with at
            // least `row_count` elements, and `slices_overlap` proved no input slice aliases it.
            let target_items = unsafe { slice::from_raw_parts_mut(raw_items_ptr, row_count) };
            let target = T::slice_from_items_mut(target_items);
            py.detach(|| {
                if parallel {
                    program.eval_batch_par(&slices, target)
                } else {
                    program.eval_batch_with(workspace, &slices, target)
                }
            })
            .map_err(|error| EvaluationError::new_err(error.to_string()))?;
        }
        return Ok(out_obj.clone().unbind());
    }

    let mut output = alloc_output(T::ZERO, row_count)?;
    py.detach(|| {
        if parallel {
            program.eval_batch_par(&slices, &mut output)
        } else {
            program.eval_batch_with(workspace, &slices, &mut output)
        }
    })
    .map_err(|error| EvaluationError::new_err(error.to_string()))?;
    T::into_py_list(output, py)
}

/// Fallibly reserve and zero-initialize an output vector of `row_count` elements.
pub fn alloc_output<Value: Clone>(zero: Value, row_count: usize) -> PyResult<Vec<Value>> {
    let mut output = Vec::new();
    output
        .try_reserve_exact(row_count)
        .map_err(|error| PyMemoryError::new_err(error.to_string()))?;
    output.resize(row_count, zero);
    Ok(output)
}

impl<T: NativeElement> ColumnBuffer<T> {
    /// Borrow a 1D C-contiguous matching buffer zero-copy, or materialize from an iterable.
    pub fn extract(column: &Bound<'_, PyAny>) -> PyResult<Self> {
        if let Ok(untyped) = PyUntypedBuffer::get(column) {
            let fmt = untyped.format().to_bytes();
            if fmt.contains(&b'?') {
                return Err(PyTypeError::new_err(
                    "booleans are not scalar numeric inputs",
                ));
            }
            if !T::IS_COMPLEX && fmt.contains(&b'Z') {
                return Err(PyTypeError::new_err(
                    "floating backends require real numeric inputs",
                ));
            }
            if untyped.dimensions() != 1 {
                return Err(PyValueError::new_err(
                    "native buffer input must be 1-dimensional",
                ));
            }
            if let Ok(buf) = untyped.into_typed::<T::BufferItem>()
                && buf.as_slice_ptr().is_some()
            {
                return Ok(Self::Buffer(buf));
            }
        }
        let mut owned = Vec::new();
        for item in column.try_iter()? {
            owned.push(T::extract_scalar(&item?)?);
        }
        Ok(Self::Owned(owned))
    }

    /// Borrow the underlying contiguous elements as `&[T]`.
    pub fn as_slice(&self) -> &[T] {
        match self {
            Self::Buffer(buf) => buf.as_slice_ptr().map_or(&[], |ptr| {
                // SAFETY: `buf` holds an active Python buffer export for the lifetime of `self`,
                // keeping memory valid, and `as_slice_ptr` verified alignment, item size, and
                // C-contiguity for `T::BufferItem`.
                let items = unsafe {
                    slice::from_raw_parts(ptr.as_ptr().cast::<T::BufferItem>(), buf.item_count())
                };
                T::slice_from_items(items)
            }),
            Self::Owned(vec) => vec.as_slice(),
        }
    }
}

// SAFETY: `ComplexBufferElem<f64>` is `#[repr(transparent)]` over `#[repr(C)] Complex<f64>`
// (16 bytes, alignment 8), matching native-endian PEP 3118 format `Zd` (`complex128`).
unsafe impl Element for ComplexBufferElem<f64> {
    fn is_compatible_format(format: &CStr) -> bool {
        is_native_complex_format(format, b'd')
    }
}

// SAFETY: `ComplexBufferElem<f32>` is `#[repr(transparent)]` over `#[repr(C)] Complex<f32>`
// (8 bytes, alignment 4), matching native-endian PEP 3118 format `Zf` (`complex64`).
unsafe impl Element for ComplexBufferElem<f32> {
    fn is_compatible_format(format: &CStr) -> bool {
        is_native_complex_format(format, b'f')
    }
}

impl NativeElement for f64 {
    type BufferItem = Self;
    const ZERO: Self = 0.0;
    const IS_COMPLEX: bool = false;

    fn slice_from_items(items: &[Self]) -> &[Self] {
        items
    }

    fn slice_from_items_mut(items: &mut [Self]) -> &mut [Self] {
        items
    }

    fn extract_scalar(value: &Bound<'_, PyAny>) -> PyResult<Self> {
        validate_real_scalar(value)?;
        value.extract::<Self>()
    }

    fn into_py_scalar(self, py: Python<'_>) -> PyResult<Py<PyAny>> {
        self.into_py_any(py)
    }

    fn into_py_list(values: Vec<Self>, py: Python<'_>) -> PyResult<Py<PyAny>> {
        values.into_py_any(py)
    }
}

impl NativeElement for f32 {
    type BufferItem = Self;
    const ZERO: Self = 0.0;
    const IS_COMPLEX: bool = false;

    fn slice_from_items(items: &[Self]) -> &[Self] {
        items
    }

    fn slice_from_items_mut(items: &mut [Self]) -> &mut [Self] {
        items
    }

    fn extract_scalar(value: &Bound<'_, PyAny>) -> PyResult<Self> {
        validate_real_scalar(value)?;
        value.extract::<Self>()
    }

    fn into_py_scalar(self, py: Python<'_>) -> PyResult<Py<PyAny>> {
        self.into_py_any(py)
    }

    fn into_py_list(values: Vec<Self>, py: Python<'_>) -> PyResult<Py<PyAny>> {
        values.into_py_any(py)
    }
}

impl NativeElement for Complex<f64> {
    type BufferItem = ComplexBufferElem<f64>;
    const ZERO: Self = Self { re: 0.0, im: 0.0 };
    const IS_COMPLEX: bool = true;

    fn slice_from_items(items: &[ComplexBufferElem<f64>]) -> &[Self] {
        ComplexBufferElem::as_complex_slice(items)
    }

    fn slice_from_items_mut(items: &mut [ComplexBufferElem<f64>]) -> &mut [Self] {
        ComplexBufferElem::as_complex_slice_mut(items)
    }

    fn extract_scalar(value: &Bound<'_, PyAny>) -> PyResult<Self> {
        extract_complex64(value)
    }

    fn into_py_scalar(self, py: Python<'_>) -> PyResult<Py<PyAny>> {
        Ok(PyComplex::from_doubles(py, self.re, self.im)
            .into_any()
            .unbind())
    }

    fn into_py_list(values: Vec<Self>, py: Python<'_>) -> PyResult<Py<PyAny>> {
        values
            .into_iter()
            .map(|z| PyComplex::from_doubles(py, z.re, z.im).into_any().unbind())
            .collect::<Vec<_>>()
            .into_py_any(py)
    }
}

impl NativeElement for Complex<f32> {
    type BufferItem = ComplexBufferElem<f32>;
    const ZERO: Self = Self { re: 0.0, im: 0.0 };
    const IS_COMPLEX: bool = true;

    fn slice_from_items(items: &[ComplexBufferElem<f32>]) -> &[Self] {
        ComplexBufferElem::as_complex_slice(items)
    }

    fn slice_from_items_mut(items: &mut [ComplexBufferElem<f32>]) -> &mut [Self] {
        ComplexBufferElem::as_complex_slice_mut(items)
    }

    fn extract_scalar(value: &Bound<'_, PyAny>) -> PyResult<Self> {
        let z = extract_complex64(value)?;
        #[expect(
            clippy::cast_possible_truncation,
            reason = "complex64 backend explicitly narrows Python complex128 scalars to f32 components"
        )]
        Ok(Self {
            re: z.re as f32,
            im: z.im as f32,
        })
    }

    fn into_py_scalar(self, py: Python<'_>) -> PyResult<Py<PyAny>> {
        Ok(
            PyComplex::from_doubles(py, f64::from(self.re), f64::from(self.im))
                .into_any()
                .unbind(),
        )
    }

    fn into_py_list(values: Vec<Self>, py: Python<'_>) -> PyResult<Py<PyAny>> {
        values
            .into_iter()
            .map(|z| {
                PyComplex::from_doubles(py, f64::from(z.re), f64::from(z.im))
                    .into_any()
                    .unbind()
            })
            .collect::<Vec<_>>()
            .into_py_any(py)
    }
}

/// Report whether two raw memory ranges of `T` overlap in address space.
pub fn ptrs_overlap<T>(
    lhs_ptr: *const T,
    lhs_len: usize,
    rhs_ptr: *const T,
    rhs_len: usize,
) -> bool {
    if lhs_len == 0 || rhs_len == 0 {
        return false;
    }
    let lhs_start = lhs_ptr.addr();
    let lhs_end = lhs_start.saturating_add(lhs_len.saturating_mul(size_of::<T>()));
    let rhs_start = rhs_ptr.addr();
    let rhs_end = rhs_start.saturating_add(rhs_len.saturating_mul(size_of::<T>()));
    lhs_start < rhs_end && rhs_start < lhs_end
}

fn is_native_complex_format(format: &CStr, code: u8) -> bool {
    let rest = match format.to_bytes() {
        [b'@' | b'=', tail @ ..] => tail,
        #[cfg(target_endian = "little")]
        [b'<', tail @ ..] => tail,
        #[cfg(target_endian = "big")]
        [b'>', tail @ ..] => tail,
        other => other,
    };
    rest == [b'Z', code]
}

fn validate_real_scalar(value: &Bound<'_, PyAny>) -> PyResult<()> {
    if value.is_instance_of::<PyBool>() {
        return Err(PyTypeError::new_err(
            "booleans are not scalar numeric inputs",
        ));
    }
    if value.is_instance_of::<PyComplex>() {
        return Err(PyTypeError::new_err(
            "floating backends require real numeric inputs",
        ));
    }
    // Reject non-real `numbers.Complex` instances (e.g. `np.complex128`, `np.complex64`),
    // which would otherwise silently discard their imaginary part via `__float__`.
    let py = value.py();
    let complex_abc = COMPLEX_ABC
        .get_or_try_init(py, || {
            PyModule::import(py, "numbers")?
                .getattr("Complex")
                .map(Bound::unbind)
        })?
        .bind(py);
    if value.is_instance(complex_abc)? {
        let real_abc = REAL_ABC
            .get_or_try_init(py, || {
                PyModule::import(py, "numbers")?
                    .getattr("Real")
                    .map(Bound::unbind)
            })?
            .bind(py);
        if !value.is_instance(real_abc)? {
            return Err(PyTypeError::new_err(
                "floating backends require real numeric inputs",
            ));
        }
    }
    Ok(())
}

fn extract_complex64(value: &Bound<'_, PyAny>) -> PyResult<Complex<f64>> {
    if value.is_instance_of::<PyBool>() {
        return Err(PyTypeError::new_err(
            "booleans are not scalar numeric inputs",
        ));
    }
    if let Ok(z) = value.cast::<PyComplex>() {
        return Ok(Complex {
            re: z.real(),
            im: z.imag(),
        });
    }
    if let Ok(real) = value.extract::<f64>() {
        return Ok(Complex { re: real, im: 0.0 });
    }
    let complex_obj = value.call_method0("__complex__")?;
    let z = complex_obj.cast::<PyComplex>()?;
    Ok(Complex {
        re: z.real(),
        im: z.imag(),
    })
}
