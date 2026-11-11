"""Identity-based direct evaluation and reusable scalar/multi-output register, JIT, and WASM programs."""

from __future__ import annotations

import array
from numbers import Complex as PyComplexABC, Real as PyRealABC
from typing import Any, Iterable, Literal, Mapping, Sequence

try:
    import numpy as np
except ImportError:
    np = None  # type: ignore[assignment]

from . import _native
from ._expression import Expr, Symbol
from ._number import Number, _BOOL_TYPES

BackendName = Literal["number", "f64", "f32", "complex128", "complex64"]

_REAL_BACKENDS = ("f64", "f32")
_COMPLEX_BACKENDS = ("complex128", "complex64")
_NATIVE_BACKENDS = ("f64", "f32", "complex128", "complex64")
_ALL_BACKENDS = ("number", "f64", "f32", "complex128", "complex64")
_BACKEND_ERROR = "backend must be 'number', 'f64', 'f32', 'complex128', or 'complex64'"

_NATIVE_DTYPES: dict[str, Any] = (
    {
        "f64": np.dtype(np.float64),
        "f32": np.dtype(np.float32),
        "complex128": np.dtype(np.complex128),
        "complex64": np.dtype(np.complex64),
    }
    if np is not None
    else {}
)
_SCALAR_TYPES: tuple[type, ...] = (
    (Number, int, float, complex, PyRealABC, PyComplexABC, np.generic)
    if np is not None
    else (Number, int, float, complex, PyRealABC, PyComplexABC)
)
_BUFFER_TYPES: tuple[type, ...] = (
    (np.ndarray, memoryview, array.array, bytes, bytearray)
    if np is not None
    else (memoryview, array.array, bytes, bytearray)
)


class Evaluator:
    """Compile one or more expressions once and reuse a workspace with ordered Symbol parameters.

    Pass a single ``Expr`` (or parseable string) for a single-output evaluator, or a
    ``list``/``tuple`` of expressions for a joint multi-output evaluator that shares
    CSE, constants, shared denominators, and paired ``sin``/``cos`` calls across all
    outputs.

    Supported backends:
    - ``"number"``: exact integer/rational/complex arithmetic returning ``Number``.
    - ``"f64"`` / ``"f32"``: IEEE-754 floating-point VM/JIT/WASM returning ``float``
      (or NumPy ``float64``/``float32`` arrays for buffer batches).
    - ``"complex128"`` / ``"complex64"``: fixed-layout complex VM/JIT returning
      ``complex`` (or zero-copy NumPy ``complex128``/``complex64`` arrays for
      buffer batches).

    Call ``with_native()`` to upgrade ``f64``/``f32``/``complex128``/``complex64``
    programs to host machine code with lazy AVX2/AVX-512 SIMD batch kernels on
    ``x86_64``, ``to_wasm_bytes()`` to export standalone WebAssembly (``f64``/``f32``),
    and ``evaluate_batch(..., parallel=True)`` for Rayon multi-threaded batch
    execution. Each ``copy()`` owns an independent workspace.
    """

    __slots__ = ("_native", "_backend", "_dtype", "_parameters", "_is_multi", "_output_count")

    def __init__(
        self,
        expression: object,
        parameters: Iterable[Symbol | Expr] | None = None,
        *,
        backend: BackendName = "number",
        native: bool = False,
    ) -> None:
        if backend not in _ALL_BACKENDS:
            raise ValueError(_BACKEND_ERROR)
        if isinstance(expression, (list, tuple)):
            models = [Expr(item) for item in expression]
            if parameters is None:
                unique = {sym.id: sym for model in models for sym in model.symbols}
                chosen: tuple[Symbol | Expr, ...] = tuple(
                    sorted(unique.values(), key=lambda sym: (sym.name or "", sym.id))
                )
            else:
                chosen = tuple(parameters)
            compiled = _native._Evaluator.multi(
                [model._native for model in models],
                [Expr(value)._native for value in chosen],
                backend,
            )
            is_multi = True
            output_count = len(models)
        else:
            model = Expr(expression)
            chosen = model.symbols if parameters is None else tuple(parameters)
            compiled = _native._Evaluator(
                model._native, [Expr(value)._native for value in chosen], backend
            )
            is_multi = False
            output_count = 1
        if native:
            compiled = compiled.with_native()
        self._native = compiled
        self._backend: BackendName = backend
        self._dtype = _NATIVE_DTYPES.get(backend)
        self._parameters = tuple(Symbol._wrap(value) for value in self._native.parameters())  # type: ignore[misc]
        self._is_multi = is_multi
        self._output_count = output_count

    @staticmethod
    def _is_buffer(values: object) -> bool:
        return (
            isinstance(values, _BUFFER_TYPES)
            or hasattr(values, "__buffer__")
            or hasattr(values, "__array_interface__")
        )

    @staticmethod
    def _coerce_real(value: object) -> float:
        if isinstance(value, _BOOL_TYPES) or isinstance(value, (str, bytes, complex)):
            raise TypeError("floating backends require real numeric inputs")
        if isinstance(value, PyComplexABC) and not isinstance(value, (Number, PyRealABC)):
            raise TypeError("floating backends require real numeric inputs")
        if isinstance(value, (Number, PyRealABC)) or hasattr(value, "__float__"):
            return float(value)  # type: ignore[arg-type]
        raise TypeError("floating backends require real numeric inputs")

    @staticmethod
    def _coerce_complex(value: object) -> complex:
        if isinstance(value, _BOOL_TYPES) or isinstance(value, (str, bytes)):
            raise TypeError("complex backends require numeric inputs")
        if isinstance(value, (Number, PyComplexABC, PyRealABC)):
            return complex(value)  # type: ignore[arg-type]
        if hasattr(value, "__complex__"):
            return complex(value)  # type: ignore[arg-type]
        if hasattr(value, "__float__"):
            return complex(float(value), 0.0)  # type: ignore[arg-type]
        raise TypeError("complex backends require numeric inputs")

    @classmethod
    def _prepare_ndarray(cls, arr: Any, backend: str) -> Any:
        target_dtype = _NATIVE_DTYPES[backend]
        if arr.dtype == target_dtype and arr.flags.c_contiguous:
            return arr
        if arr.dtype == np.bool_:
            raise TypeError("booleans are not scalar numeric inputs")
        if backend in _REAL_BACKENDS:
            if np.issubdtype(arr.dtype, np.complexfloating) or not np.issubdtype(arr.dtype, np.number):
                raise TypeError("floating backends require real numeric inputs")
        elif not np.issubdtype(arr.dtype, np.number):
            raise TypeError("complex backends require numeric inputs")
        return np.ascontiguousarray(arr, dtype=target_dtype)

    @classmethod
    def _values(cls, values: Any, backend: str) -> Any:
        if backend == "number":
            if np is not None and isinstance(values, np.ndarray):
                if values.dtype == np.bool_:
                    raise TypeError("booleans are not scalar numeric inputs")
                if values.ndim != 1:
                    raise ValueError(f"expected a 1-dimensional sequence, got {values.ndim}D array")
                values = values.tolist()
            return [Number._native_for(value) for value in values]
        if backend in _NATIVE_BACKENDS:
            if np is not None and isinstance(values, np.ndarray):
                if values.ndim != 1:
                    raise ValueError(f"expected a 1-dimensional array, got {values.ndim}D array")
                return cls._prepare_ndarray(values, backend)
            if cls._is_buffer(values):
                return values
            if hasattr(values, "tolist") and not isinstance(values, (list, tuple)):
                values = values.tolist()
            coerce = cls._coerce_real if backend in _REAL_BACKENDS else cls._coerce_complex
            return [coerce(value) for value in values]
        raise ValueError(_BACKEND_ERROR)

    @classmethod
    def _direct(
        cls,
        expression: object,
        bindings: Mapping[Symbol | Expr, Any] | Iterable[tuple[Symbol | Expr, Any]] = (),
        *,
        backend: BackendName = "number",
    ) -> Number | float | complex:
        if backend not in _ALL_BACKENDS:
            raise ValueError(_BACKEND_ERROR)
        pairs = list(bindings.items() if isinstance(bindings, Mapping) else bindings)
        parameters = [Expr(symbol)._native for symbol, _ in pairs]
        numbers = cls._values((value for _, value in pairs), backend)
        result = _native._Evaluator.direct(Expr(expression)._native, parameters, numbers, backend)
        return Number._wrap(result) if backend == "number" else result

    @property
    def parameters(self) -> tuple[Symbol, ...]:
        return self._parameters

    @property
    def output_count(self) -> int:
        return self._output_count

    @property
    def backend(self) -> BackendName:
        return self._backend

    @property
    def instruction_count(self) -> int:
        return self._native.instruction_count()

    @property
    def workspace_size(self) -> int:
        return self._native.workspace_size()

    @property
    def native_bytes(self) -> bytes | None:
        """Position-independent host machine-code bytes when backed by a native JIT kernel."""
        return self._native.native_bytes()

    def _clone_with(self, native_eval: Any) -> Evaluator:
        value = object.__new__(type(self))
        value._native = native_eval
        value._backend = self._backend
        value._dtype = self._dtype
        value._parameters = self._parameters
        value._is_multi = self._is_multi
        value._output_count = self._output_count
        return value

    def with_native(self) -> Evaluator:
        """Return an Evaluator upgraded to host machine code (W^X JIT + lazy SIMD batches)."""
        return self._clone_with(self._native.with_native())

    def to_wasm_bytes(self) -> bytes:
        """Export a standalone WebAssembly (.wasm) module for ``f64`` or ``f32`` programs."""
        return self._native.to_wasm_bytes()

    def evaluate(
        self, values: Sequence[Any] | Any
    ) -> Number | float | complex | tuple[Number, ...] | tuple[float, ...] | tuple[complex, ...]:
        result = self._native.evaluate(self._values(values, self._backend))
        if self._backend == "number":
            if self._is_multi:
                return tuple(Number._wrap(item) for item in result)  # type: ignore[union-attr]
            return Number._wrap(result)
        return result

    @staticmethod
    def _flatten_out_ndarray(arr: Any, shape: tuple[int, ...] | None, count: int, rows: int | None) -> Any:
        if not arr.flags.writeable:
            raise ValueError("output buffer must be writable")
        if shape is None:
            if not arr.flags.c_contiguous:
                raise ValueError("output buffer must be C-contiguous and writable")
            if arr.size < count:
                raise ValueError(f"output buffer size ({arr.size}) is smaller than row_count ({count})")
            if arr.ndim > 1 and rows is not None and arr.size != rows:
                raise ValueError(f"output buffer shape {arr.shape} does not match rows={rows}")
            return arr.ravel() if arr.ndim > 1 else arr
        if len(shape) > 1:
            if arr.shape != shape:
                raise ValueError(f"output buffer shape {arr.shape} does not match expected shape {shape}")
            if not arr.flags.c_contiguous:
                raise ValueError("output buffer must be C-contiguous and writable")
            return arr.ravel()
        if arr.ndim != 1:
            raise ValueError("output buffer must be a 1-dimensional contiguous buffer")
        return arr

    def _prepare_out(self, out: Any, shape: tuple[int, ...] | None, count: int, rows: int | None) -> tuple[Any, Any]:
        if self._is_multi:
            if not isinstance(out, (list, tuple)):
                raise TypeError(
                    f"multi-output evaluator requires a list or tuple of {self._output_count} output buffers"
                )
            if len(out) != self._output_count:
                raise ValueError(f"expected {self._output_count} output buffers, received {len(out)}")
            flat = [
                self._flatten_out_ndarray(buf, shape, count, rows) if isinstance(buf, np.ndarray) else buf
                for buf in out
            ]
            return flat, tuple(out)
        if isinstance(out, np.ndarray):
            return self._flatten_out_ndarray(out, shape, count, rows), out
        return out, None

    def _prepare_numpy_batch(
        self,
        raw_columns: Sequence[Any],
        rows: int | None,
        out: Any,
        parallel: bool,
    ) -> Any:
        dtype = self._dtype
        if not raw_columns:
            if out is not None:
                first_arr = (
                    out[0] if self._is_multi and isinstance(out, (list, tuple)) and out else out
                )
                count = first_arr.size if rows is None and isinstance(first_arr, np.ndarray) else (rows or 0)
                flat_out, ret = self._prepare_out(out, None, count, rows)
                res = self._native.evaluate_batch([], count, flat_out, parallel)
                return res if ret is None else ret
            count = 0 if rows is None else rows
            if self._is_multi:
                results = tuple(np.empty((count,), dtype=dtype) for _ in range(self._output_count))
                self._native.evaluate_batch([], count, list(results), parallel)
                return results
            return self._native.evaluate_batch([], count, None, parallel)

        coerce = self._coerce_real if self._backend in _REAL_BACKENDS else self._coerce_complex
        arrays: list[Any] = []
        has_nd_or_scalar = False
        for col in raw_columns:
            if isinstance(col, np.ndarray):
                arr = self._prepare_ndarray(col, self._backend)
            elif isinstance(col, _SCALAR_TYPES):
                arr = np.asarray(coerce(col), dtype=dtype)
            elif self._is_buffer(col):
                arr = self._prepare_ndarray(np.asarray(col), self._backend)
            else:
                if hasattr(col, "tolist") and not isinstance(col, (list, tuple)):
                    col = col.tolist()
                arr = np.asarray([coerce(item) for item in col], dtype=dtype)
            if arr.ndim != 1:
                has_nd_or_scalar = True
            arrays.append(arr)

        if has_nd_or_scalar:
            shape = np.broadcast_shapes(*(arr.shape for arr in arrays)) or (1,)
            total = int(np.prod(shape))
            if rows is not None and rows != total:
                raise ValueError(f"rows ({rows}) does not match broadcast batch size ({total})")
            flat_cols = [
                np.ascontiguousarray(np.broadcast_to(arr, shape), dtype=dtype).ravel()
                for arr in arrays
            ]
        else:
            total = arrays[0].shape[0] if rows is None else rows
            shape = (total,)
            flat_cols = arrays

        if out is not None:
            flat_out, ret = self._prepare_out(out, shape, total, rows)
            res = self._native.evaluate_batch(flat_cols, total, flat_out, parallel)
            return res if ret is None else ret

        if self._is_multi:
            results = tuple(np.empty(shape, dtype=dtype) for _ in range(self._output_count))
            flat_results = [arr.ravel() if arr.ndim > 1 else arr for arr in results]
            self._native.evaluate_batch(flat_cols, total, flat_results, parallel)
            return results

        result = np.empty(shape, dtype=dtype)
        flat_res = result.ravel() if result.ndim > 1 else result
        self._native.evaluate_batch(flat_cols, total, flat_res, parallel)
        return result

    def evaluate_batch(
        self,
        columns: Sequence[Any] | Any,
        *,
        rows: int | None = None,
        out: Any | None = None,
        parallel: bool = False,
    ) -> list[Number] | list[float] | list[complex] | tuple[Any, ...] | Any:
        raw_columns = columns if isinstance(columns, (list, tuple)) else list(columns)

        if self._backend == "number":
            if out is not None:
                raise TypeError("out buffer is only supported for floating-point and complex backends")
            values = [self._values(column, self._backend) for column in raw_columns]
            count = (len(values[0]) if values else 0) if rows is None else rows
            result = self._native.evaluate_batch(values, count, None, parallel)
            if self._is_multi:
                return tuple([Number._wrap(value) for value in col] for col in result)
            return [Number._wrap(value) for value in result]

        # Fast path: 1D C-contiguous matching-dtype NumPy columns with matching 1D writable `out`.
        dtype = self._dtype
        if (
            dtype is not None
            and raw_columns
            and out is not None
            and all(
                isinstance(c, np.ndarray) and c.ndim == 1 and c.dtype == dtype and c.flags.c_contiguous
                for c in raw_columns
            )
        ):
            total = raw_columns[0].shape[0] if rows is None else rows
            if self._is_multi:
                if (
                    isinstance(out, (list, tuple))
                    and len(out) == self._output_count
                    and all(
                        isinstance(b, np.ndarray)
                        and b.ndim == 1
                        and b.dtype == dtype
                        and b.flags.c_contiguous
                        and b.flags.writeable
                        for b in out
                    )
                ):
                    self._native.evaluate_batch(raw_columns, total, out, parallel)
                    return out if isinstance(out, tuple) else tuple(out)
            elif (
                isinstance(out, np.ndarray)
                and out.ndim == 1
                and out.dtype == dtype
                and out.flags.c_contiguous
                and out.flags.writeable
            ):
                self._native.evaluate_batch(raw_columns, total, out, parallel)
                return out

        out_has_buffer = False
        if out is not None:
            if self._is_multi and isinstance(out, (list, tuple)):
                out_has_buffer = any(self._is_buffer(item) for item in out)
            else:
                out_has_buffer = self._is_buffer(out)
        if np is not None and (
            out_has_buffer
            or any(self._is_buffer(c) or isinstance(c, _SCALAR_TYPES) for c in raw_columns)
        ):
            return self._prepare_numpy_batch(raw_columns, rows, out, parallel)

        values = [self._values(column, self._backend) for column in raw_columns]
        count = (len(values[0]) if values else 0) if rows is None else rows
        return self._native.evaluate_batch(values, count, out, parallel)

    def copy(self) -> Evaluator:
        return self._clone_with(self._native.copy())

    def disassemble(self) -> str:
        return self._native.disassemble()

    def __repr__(self) -> str:
        params = tuple(param.name or f"${param.id}" for param in self._parameters)
        outputs_part = f", outputs={self._output_count}" if self._is_multi else ""
        return (
            f"Evaluator(backend={self._backend!r}, parameters={params!r}{outputs_part}, "
            f"instructions={self.instruction_count}, native={self.native_bytes is not None})"
        )
