//! Mat(2^{⌈n/2⌉}, ℂ) embedding for **n ≥ 4** Clifford generators.
//!
//! Uses a custom dense matrix type and a basic QR algorithm to apply analytic
//! functions via the spectral mapping theorem, avoiding external dependencies.

use alloc::vec;
use alloc::vec::Vec;

use super::super::scalar::Scalar;
use super::super::traits::Number;
use super::matrix::Cmplx;
use super::spectral::nan_clifford;
use super::types::CliffordNumber;
use super::types::{GeneratorSet, coeff_count_unchecked, scalar_one, scalar_zero};

// ---------------------------------------------------------------------------
// Custom Complex Matrix
// ---------------------------------------------------------------------------

#[derive(Clone, Debug)]
pub struct MatC {
    pub dim: usize,
    pub data: Vec<Cmplx>,
}

impl MatC {
    pub fn zeros(dim: usize) -> Self {
        let mut data = Vec::with_capacity(dim * dim);
        let z = Cmplx::zero();
        for _ in 0..dim * dim {
            data.push(z.clone());
        }
        Self { dim, data }
    }

    pub fn identity(dim: usize) -> Self {
        let mut mat = Self::zeros(dim);
        let one = Cmplx::one();
        for i in 0..dim {
            mat.set(i, i, one.clone());
        }
        mat
    }

    pub fn get(&self, row: usize, col: usize) -> &Cmplx {
        &self.data[row * self.dim + col]
    }

    pub fn set(&mut self, row: usize, col: usize, val: Cmplx) {
        self.data[row * self.dim + col] = val;
    }

    pub fn add(&self, other: &Self) -> Self {
        let mut result = Self::zeros(self.dim);
        for i in 0..self.data.len() {
            result.data[i] = self.data[i].add(&other.data[i]);
        }
        result
    }

    pub fn mul(&self, other: &Self) -> Self {
        let dim = self.dim;
        let mut result = Self::zeros(dim);
        for i in 0..dim {
            for j in 0..dim {
                let mut sum = Cmplx::zero();
                for k in 0..dim {
                    sum = sum.add(&self.get(i, k).mul(other.get(k, j)));
                }
                result.set(i, j, sum);
            }
        }
        result
    }

    pub fn scale(&self, s: &Cmplx) -> Self {
        let mut result = Self::zeros(self.dim);
        for i in 0..self.data.len() {
            result.data[i] = self.data[i].mul(s);
        }
        result
    }

    pub fn dagger(&self) -> Self {
        let mut result = Self::zeros(self.dim);
        for i in 0..self.dim {
            for j in 0..self.dim {
                result.set(j, i, self.get(i, j).conj());
            }
        }
        result
    }

    pub fn trace(&self) -> Cmplx {
        let mut tr = Cmplx::zero();
        for i in 0..self.dim {
            tr = tr.add(self.get(i, i));
        }
        tr
    }
}

// ---------------------------------------------------------------------------
// Kronecker product
// ---------------------------------------------------------------------------

fn kron(a: &MatC, b: &MatC) -> MatC {
    let dim = a.dim * b.dim;
    let mut result = MatC::zeros(dim);
    for i in 0..a.dim {
        for j in 0..a.dim {
            let a_ij = a.get(i, j);
            if a_ij.is_zero() {
                continue;
            }
            let base_row = i * b.dim;
            let base_col = j * b.dim;
            for p in 0..b.dim {
                for q in 0..b.dim {
                    let b_pq = b.get(p, q);
                    if b_pq.is_zero() {
                        continue;
                    }
                    result.set(base_row + p, base_col + q, a_ij.mul(b_pq));
                }
            }
        }
    }
    result
}

fn kron_list(factors: &[MatC]) -> MatC {
    if factors.is_empty() {
        let mut m = MatC::zeros(1);
        m.set(0, 0, Cmplx::one());
        return m;
    }
    let mut result = factors[0].clone();
    for f in &factors[1..] {
        result = kron(&result, f);
    }
    result
}

// ---------------------------------------------------------------------------
// Pauli matrices
// ---------------------------------------------------------------------------

fn pauli_sx() -> MatC {
    let mut m = MatC::zeros(2);
    m.set(0, 1, Cmplx::one());
    m.set(1, 0, Cmplx::one());
    m
}

fn pauli_sy() -> MatC {
    let mut m = MatC::zeros(2);
    let i = Cmplx::i();
    let neg_i = i.neg();
    m.set(0, 1, neg_i);
    m.set(1, 0, i);
    m
}

fn pauli_sz() -> MatC {
    let mut m = MatC::zeros(2);
    m.set(0, 0, Cmplx::one());
    m.set(1, 1, Cmplx::one().neg());
    m
}

fn identity2() -> MatC {
    MatC::identity(2)
}

// ---------------------------------------------------------------------------
// Euclidean gamma matrices (Weyl / chiral representation)
// ---------------------------------------------------------------------------

fn euclidean_gammas(n: usize) -> Vec<MatC> {
    let k = n.div_ceil(2);
    let sx = pauli_sx();
    let sy = pauli_sy();
    let sz = pauli_sz();
    let i2 = identity2();

    let mut gammas = Vec::with_capacity(n);
    #[allow(clippy::integer_division, reason = "exact half count is intentional")]
    for pair in 0..(n / 2) {
        let mut x_factors: Vec<MatC> = Vec::with_capacity(k);
        let mut y_factors: Vec<MatC> = Vec::with_capacity(k);
        for _ in 0..pair {
            x_factors.push(sz.clone());
            y_factors.push(sz.clone());
        }
        x_factors.push(sx.clone());
        y_factors.push(sy.clone());
        for _ in 0..(k - pair - 1) {
            x_factors.push(i2.clone());
            y_factors.push(i2.clone());
        }
        gammas.push(kron_list(&x_factors));
        gammas.push(kron_list(&y_factors));
    }
    if n % 2 == 1 {
        let factors = vec![sz; k];
        gammas.push(kron_list(&factors));
    }
    gammas
}

// ---------------------------------------------------------------------------
// Metric-adjusted generator matrices
// ---------------------------------------------------------------------------

fn generator_matrices(gens: &GeneratorSet) -> Option<Vec<MatC>> {
    let euclid = euclidean_gammas(gens.len());
    let mut result = Vec::with_capacity(gens.len());
    for (pos, gm) in euclid.iter().enumerate() {
        match gens.metric_at(pos) {
            1 => result.push(gm.clone()),
            -1 => result.push(gm.scale(&Cmplx::i())),
            _ => return None,
        }
    }
    Some(result)
}

// ---------------------------------------------------------------------------
// Blade basis
// ---------------------------------------------------------------------------

fn blade_basis(gens: &GeneratorSet) -> Option<Vec<MatC>> {
    let gen_mats = generator_matrices(gens)?;
    let n = gen_mats.len();
    let count = 1_usize << n;
    let dim = gen_mats[0].dim;
    let mut basis = Vec::with_capacity(count);
    basis.push(MatC::identity(dim));
    for i in 1..count {
        let mut blade = MatC::identity(dim);
        for (g, gm) in gen_mats.iter().enumerate() {
            if (i >> g) & 1 == 1 {
                blade = blade.mul(gm);
            }
        }
        basis.push(blade);
    }
    Some(basis)
}

// ---------------------------------------------------------------------------
// CliffordNumber ↔ MatC
// ---------------------------------------------------------------------------

fn to_matrix(mv: &CliffordNumber) -> Option<MatC> {
    let gens = mv.generator_set();
    let basis = blade_basis(gens)?;
    let dim = basis[0].dim;
    let mut mat = MatC::zeros(dim);
    for (blade, bm) in basis.iter().enumerate() {
        let coeff = mv.coeff(blade);
        if coeff.is_zero() {
            continue;
        }
        let term = bm.scale(&Cmplx::new(coeff.clone(), scalar_zero()));
        mat = mat.add(&term);
    }
    Some(mat)
}

fn from_matrix(mat: &MatC, gens: &GeneratorSet) -> CliffordNumber {
    let Some(basis) = blade_basis(gens) else {
        return CliffordNumber::zero_unchecked(gens.clone());
    };
    let count = coeff_count_unchecked(u8::try_from(gens.len()).unwrap_or(255));
    let mut mv = CliffordNumber::zero_unchecked(gens.clone());
    for (blade, bm) in basis.iter().enumerate().take(count) {
        let bm_dag = bm.dagger();
        let numer = mat.mul(&bm_dag).trace().0;
        let denom = bm.mul(&bm_dag).trace().0;

        let c = if denom.is_zero() {
            scalar_zero()
        } else {
            numer / denom
        };
        if !c.is_zero() {
            mv.coeffs_mut_slice()[blade] = c;
        }
    }
    mv
}

// ---------------------------------------------------------------------------
// Eigendecomposition (QR Algorithm)
// ---------------------------------------------------------------------------

fn givens_rotation(x_val: &Cmplx, y_val: &Cmplx) -> (Cmplx, Cmplx) {
    if y_val.abs_sq().is_zero() {
        return (Cmplx::one(), Cmplx::zero());
    }
    if x_val.abs_sq().is_zero() {
        let mag = y_val.abs_sq().sqrt();
        let inv_mag = Cmplx::one().div(&Cmplx::new(mag, scalar_zero()));
        return (Cmplx::zero(), y_val.conj().mul(&inv_mag));
    }
    let r_val = (x_val.abs_sq() + y_val.abs_sq()).sqrt();
    let x_mag = x_val.abs_sq().sqrt();

    let inv_r = Cmplx::one().div(&Cmplx::new(r_val, scalar_zero()));
    let cos_val = Cmplx::new(x_mag.clone(), scalar_zero()).mul(&inv_r);

    let inv_x_mag = Cmplx::one().div(&Cmplx::new(x_mag, scalar_zero()));
    let sign_x = x_val.mul(&inv_x_mag);
    let sin_val = y_val.conj().mul(&sign_x).mul(&inv_r);

    (cos_val, sin_val)
}

fn hessenberg_reduce(a: &mut MatC, q: &mut MatC, buf_v: &mut Vec<Cmplx>) {
    let n = a.dim;
    if n <= 2 {
        return;
    }
    for k in 0..n - 2 {
        let mut norm_sq = scalar_zero();
        for i in k + 1..n {
            norm_sq = norm_sq + a.get(i, k).abs_sq();
        }
        let norm = norm_sq.sqrt();
        if norm.is_zero() {
            continue;
        }

        buf_v.clear();
        for i in k + 1..n {
            buf_v.push(a.get(i, k).clone());
        }

        let x0 = &buf_v[0];
        let x0_mag = x0.abs_sq().sqrt();
        let phase = if x0_mag.is_zero() {
            Cmplx::one()
        } else {
            let inv_mag = Cmplx::one().div(&Cmplx::new(x0_mag, scalar_zero()));
            x0.mul(&inv_mag)
        };
        let norm_c = Cmplx::new(norm, scalar_zero());
        let shifted = phase.mul(&norm_c);
        buf_v[0] = buf_v[0].add(&shifted);

        let mut v_norm_sq = Cmplx::zero();
        for vi in buf_v.iter() {
            v_norm_sq = v_norm_sq.add(&vi.conj().mul(vi));
        }
        if v_norm_sq.abs_sq().is_zero() {
            continue;
        }

        let two = Cmplx::new(scalar_one() + scalar_one(), scalar_zero());
        let two_over_v_norm_sq = two.div(&v_norm_sq);

        for j in k..n {
            let mut v_star_a = Cmplx::zero();
            for i in k + 1..n {
                v_star_a = v_star_a.add(&buf_v[i - k - 1].conj().mul(a.get(i, j)));
            }
            let scaled = v_star_a.mul(&two_over_v_norm_sq);
            for i in k + 1..n {
                let term = buf_v[i - k - 1].mul(&scaled);
                a.set(i, j, a.get(i, j).sub(&term));
            }
        }

        for i in k + 2..n {
            a.set(i, k, Cmplx::zero());
        }

        for i in 0..n {
            let mut a_v = Cmplx::zero();
            for j in k + 1..n {
                a_v = a_v.add(&a.get(i, j).mul(&buf_v[j - k - 1]));
            }
            let scaled = a_v.mul(&two_over_v_norm_sq);
            for j in k + 1..n {
                let term = scaled.mul(&buf_v[j - k - 1].conj());
                a.set(i, j, a.get(i, j).sub(&term));
            }
        }

        for i in 0..n {
            let mut q_v = Cmplx::zero();
            for j in k + 1..n {
                q_v = q_v.add(&q.get(i, j).mul(&buf_v[j - k - 1]));
            }
            let scaled = q_v.mul(&two_over_v_norm_sq);
            for j in k + 1..n {
                let term = scaled.mul(&buf_v[j - k - 1].conj());
                q.set(i, j, q.get(i, j).sub(&term));
            }
        }
    }
}

fn qr_step_hessenberg(
    a: &mut MatC,
    q_total: &mut MatC,
    active: usize,
    buf_givens: &mut Vec<(Cmplx, Cmplx)>,
) {
    let dim = a.dim;
    let mut shift = Cmplx::zero();
    if active >= 2 {
        let n1 = active - 1;
        let n2 = active - 2;
        let a_11 = a.get(n1, n1);
        let a_22 = a.get(n2, n2);
        let a_12 = a.get(n1, n2);
        let a_21 = a.get(n2, n1);

        let tr = a_22.add(a_11);
        let det = a_22.mul(a_11).sub(&a_21.mul(a_12));

        let four = Cmplx::new(
            scalar_one() + scalar_one() + scalar_one() + scalar_one(),
            scalar_zero(),
        );
        let disc = tr.mul(&tr).sub(&four.mul(&det));
        let sqrt_disc = disc.sqrt();

        let inv_two = Cmplx::one().div(&Cmplx::new(scalar_one() + scalar_one(), scalar_zero()));
        let l1 = tr.add(&sqrt_disc).mul(&inv_two);
        let l2 = tr.sub(&sqrt_disc).mul(&inv_two);

        if l1.sub(a_11).abs_sq().total_cmp(&l2.sub(a_11).abs_sq()) == core::cmp::Ordering::Less {
            shift = l1;
        } else {
            shift = l2;
        }
    }

    for i in 0..active {
        let val = a.get(i, i).sub(&shift);
        a.set(i, i, val);
    }

    buf_givens.clear();
    for i in 0..active - 1 {
        let x_val = a.get(i, i).clone();
        let y_val = a.get(i + 1, i).clone();
        let (cos_val, sin_val) = givens_rotation(&x_val, &y_val);
        buf_givens.push((cos_val.clone(), sin_val.clone()));

        for j in i..active {
            let a_ij = a.get(i, j).clone();
            let a_ip1j = a.get(i + 1, j).clone();
            a.set(i, j, cos_val.mul(&a_ij).add(&sin_val.mul(&a_ip1j)));
            a.set(
                i + 1,
                j,
                sin_val.conj().neg().mul(&a_ij).add(&cos_val.mul(&a_ip1j)),
            );
        }
    }

    for (i, (cos_val, sin_val)) in buf_givens.iter().cloned().enumerate() {
        for j in 0..=i + 1 {
            let a_ji = a.get(j, i).clone();
            let a_jip1 = a.get(j, i + 1).clone();
            a.set(j, i, a_ji.mul(&cos_val).add(&a_jip1.mul(&sin_val.conj())));
            a.set(
                j,
                i + 1,
                a_ji.mul(&sin_val.neg()).add(&a_jip1.mul(&cos_val)),
            );
        }
    }

    for i in 0..active {
        let val = a.get(i, i).add(&shift);
        a.set(i, i, val);
    }

    for (i, (cos_val, sin_val)) in buf_givens.iter().cloned().enumerate() {
        for j in 0..dim {
            let q_ji = q_total.get(j, i).clone();
            let q_jip1 = q_total.get(j, i + 1).clone();
            q_total.set(j, i, q_ji.mul(&cos_val).add(&q_jip1.mul(&sin_val.conj())));
            q_total.set(
                j,
                i + 1,
                q_ji.mul(&sin_val.neg()).add(&q_jip1.mul(&cos_val)),
            );
        }
    }
}

fn schur_decomposition(mat: &MatC) -> Option<(MatC, MatC)> {
    let mut a = mat.clone();
    let mut q = MatC::identity(mat.dim);

    // Pre-allocate buffers to avoid repeated heap allocations in the QR loop
    let mut buf_v: Vec<Cmplx> = Vec::with_capacity(mat.dim);
    let mut buf_givens: Vec<(Cmplx, Cmplx)> = Vec::with_capacity(mat.dim);

    let mut active = mat.dim;
    // Tolerance for deflation: 10 * epsilon^2
    let eps_val = super::super::scalar::Scalar::epsilon();
    let eps_sq = &eps_val * &eps_val;
    let ten_int = super::super::int_math::from_i64(10)?;
    let ten = super::super::scalar::Scalar::from_int(ten_int);
    let tol_sq = &eps_sq * &ten;

    hessenberg_reduce(&mut a, &mut q, &mut buf_v);

    for _ in 0..(mat.dim * 200) {
        if active <= 1 {
            break;
        }

        let mut deflated = false;
        let mut off_diag_sq_sum = scalar_zero();
        for i in 0..active - 1 {
            off_diag_sq_sum = off_diag_sq_sum + a.get(active - 1, i).abs_sq();
        }

        if off_diag_sq_sum.total_cmp(&tol_sq) == core::cmp::Ordering::Less {
            a.set(active - 1, active - 2, Cmplx::zero());
            active -= 1;
            deflated = true;
        }

        if !deflated {
            qr_step_hessenberg(&mut a, &mut q, active, &mut buf_givens);
        }
    }

    if active > 1 {
        return None;
    }

    Some((q, a))
}

// ---------------------------------------------------------------------------
// Eigendecomposition from Schur form (handles non-diagonalizable matrices)
// ---------------------------------------------------------------------------

/// Check if an upper triangular matrix is numerically diagonal.
fn is_triangular_diagonal(t: &MatC, tol_sq: &Scalar) -> bool {
    for i in 0..t.dim {
        for j in i + 1..t.dim {
            if t.get(i, j).abs_sq().total_cmp(tol_sq) == core::cmp::Ordering::Greater {
                return false;
            }
        }
    }
    true
}

/// Extract eigenvalues from the diagonal of an upper triangular matrix.
fn eigenvalues_from_triangular(t: &MatC) -> Vec<Cmplx> {
    let n = t.dim;
    let mut eigvals = Vec::with_capacity(n);
    for i in 0..n {
        eigvals.push(t.get(i, i).clone());
    }
    eigvals
}

/// Check whether all eigenvalues are numerically distinct.
fn eigenvalues_are_distinct(eigvals: &[Cmplx], tol_sq: &Scalar) -> bool {
    for i in 0..eigvals.len() {
        for j in i + 1..eigvals.len() {
            if eigvals[i].sub(&eigvals[j]).abs_sq().total_cmp(tol_sq) == core::cmp::Ordering::Less {
                return false;
            }
        }
    }
    true
}

/// Compute eigenvectors of an upper triangular matrix with **distinct** eigenvalues.
///
/// Returns `V` where each column is an eigenvector, normalized so that `V[i][i] = 1`.
/// `V` is upper triangular with unit diagonal.
fn eigenvectors_from_triangular_distinct(t: &MatC) -> Option<MatC> {
    let n = t.dim;
    let eigvals = eigenvalues_from_triangular(t);
    let eps_val = super::super::scalar::Scalar::epsilon();
    let eps_sq = &eps_val * &eps_val;
    let ten_int = super::super::int_math::from_i64(10)?;
    let ten = super::super::scalar::Scalar::from_int(ten_int);
    let distinct_tol_sq = &eps_sq * &ten;

    if !eigenvalues_are_distinct(&eigvals, &distinct_tol_sq) {
        return None;
    }

    let mut v = MatC::identity(n); // V[:, i] = eigenvector for λ_i

    for (i, eig_i) in eigvals.iter().enumerate() {
        // For eigenvector v_i (column i), components > i are 0, component i is 1.
        // Back-substitute for components j = i-1, i-2, ..., 0:
        //   v_i[j] = -sum_{k=j+1}^{i} T[j][k] * v_i[k] / (T[j][j] - λ_i)
        for j in (0..i).rev() {
            let denom = t.get(j, j).sub(eig_i);
            let mut sum = Cmplx::zero();
            for k in j + 1..=i {
                sum = sum.add(&t.get(j, k).mul(v.get(k, i)));
            }
            v.set(j, i, sum.neg().div(&denom));
        }
    }
    Some(v)
}

/// Compute the inverse of an upper triangular matrix with unit diagonal.
/// Result is also upper triangular with unit diagonal.
fn inverse_upper_triangular(v: &MatC) -> MatC {
    let n = v.dim;
    let mut vinv = MatC::identity(n);

    // For i < j: (V^{-1})[i][j] = -sum_{k=i+1}^{j} V[i][k] * (V^{-1})[k][j]
    for j in 1..n {
        for i in (0..j).rev() {
            let mut sum = Cmplx::zero();
            for k in i + 1..=j {
                sum = sum.add(&v.get(i, k).mul(vinv.get(k, j)));
            }
            vinv.set(i, j, sum.neg());
        }
    }
    vinv
}

/// Compute the rank of an upper triangular matrix by counting non-zero diagonal entries
/// above a numerical threshold.
fn rank_upper_triangular(mat: &MatC, tol_sq: &Scalar) -> usize {
    let mut r = 0;
    for i in 0..mat.dim {
        if mat.get(i, i).abs_sq().total_cmp(tol_sq) == core::cmp::Ordering::Greater {
            r += 1;
        }
    }
    r
}

/// Attempt a full eigendecomposition from a Schur factor `T`.
///
/// Returns `(V, V_inv)` where `T = V * D * V^{-1}`,
/// or `None` if the matrix is defective.
fn eigendecompose_from_schur(t: &MatC) -> Option<(MatC, MatC)> {
    let eps_val = super::super::scalar::Scalar::epsilon();
    let eps_sq = &eps_val * &eps_val;
    let ten_int = super::super::int_math::from_i64(10)?;
    let ten = super::super::scalar::Scalar::from_int(ten_int);
    let tol_sq = &eps_sq * &ten;

    // Fast path: T is already diagonal → V = I
    if is_triangular_diagonal(t, &tol_sq) {
        return Some((MatC::identity(t.dim), MatC::identity(t.dim)));
    }

    // Try to compute eigenvectors assuming distinct eigenvalues (most common case)
    if let Some(v) = eigenvectors_from_triangular_distinct(t) {
        let vinv = inverse_upper_triangular(&v);
        return Some((v, vinv));
    }

    // Repeated eigenvalues: check diagonalizability via rank condition.
    // For each distinct eigenvalue λ with multiplicity m:
    //   rank(T - λI) == n - m  ⇒  diagonalizable (geometric = algebraic)
    //   rank(T - λI) <  n - m  ⇒  defective (Jordan block)
    let n = t.dim;
    let eigvals = eigenvalues_from_triangular(t);

    // Group eigenvalues by proximity
    let mut processed = vec![false; n];
    for (i, eig_i) in eigvals.iter().enumerate() {
        if processed[i] {
            continue;
        }
        // Find all eigenvalues close to eig_i
        let mut multiplicity = 0_usize;
        for j in i..n {
            if eig_i.sub(&eigvals[j]).abs_sq().total_cmp(&tol_sq) == core::cmp::Ordering::Less {
                processed[j] = true;
                multiplicity += 1;
            }
        }

        // Compute rank(T - λ_i * I)
        let mut shifted = t.clone();
        for k in 0..n {
            let val = shifted.get(k, k).sub(eig_i);
            shifted.set(k, k, val);
        }
        let r = rank_upper_triangular(&shifted, &tol_sq);
        if r != n - multiplicity {
            return None; // defective
        }
    }

    // Matrix is diagonalizable with repeated eigenvalues.
    // For each distinct eigenvalue, find nullspace basis of (T - λI).
    let mut v = MatC::identity(n);
    processed = vec![false; n];
    for (i, eig_i) in eigvals.iter().enumerate() {
        if processed[i] {
            continue;
        }
        // Find cluster: all j where λ_j ≈ λ_i
        let mut cluster: Vec<usize> = Vec::new();
        for j in 0..n {
            if eig_i.sub(&eigvals[j]).abs_sq().total_cmp(&tol_sq) == core::cmp::Ordering::Less
                && !processed[j]
            {
                cluster.push(j);
                processed[j] = true;
            }
        }
        let m = cluster.len();
        if m == 1 {
            // Simple eigenvalue — already handled by identity initialization
            continue;
        }

        // Solve for nullspace basis vectors of (T - λI).
        // The nullspace has dimension m. For each cluster position idx,
        // set v[idx][idx] = 1 and solve for earlier components in the cluster.
        let lambda = eig_i;
        for &idx in &cluster {
            for &j in &cluster {
                if j < idx {
                    let mut sum = Cmplx::zero();
                    for &l in &cluster {
                        if l > j && l <= idx {
                            sum = sum.add(&t.get(j, l).mul(v.get(l, idx)));
                        }
                    }
                    let denom = t.get(j, j).sub(lambda);
                    if denom.abs_sq().total_cmp(&tol_sq) == core::cmp::Ordering::Greater {
                        v.set(j, idx, sum.neg().div(&denom));
                    }
                }
            }
        }
    }

    let vinv = inverse_upper_triangular(&v);
    Some((v, vinv))
}

// ---------------------------------------------------------------------------
// Spectral function application via Schur decomposition
// ---------------------------------------------------------------------------

/// Apply an analytic function `f: Cmplx → Cmplx` via Schur decomposition.
///
/// Returns `mv.clone()` if the algebra cannot be embedded (nilpotent gens) or
/// if the Schur decomposition fails to converge.
pub fn apply_via_spectral<F>(mv: &CliffordNumber, f: &F) -> CliffordNumber
where
    F: Fn(&Cmplx) -> Cmplx,
{
    let gens = mv.generator_set();
    let Some(mat) = to_matrix(mv) else {
        return nan_clifford(gens);
    };

    let Some((q, t)) = schur_decomposition(&mat) else {
        return nan_clifford(gens);
    };

    let dim = t.dim;

    // Attempt full eigendecomposition from the Schur form
    let Some((v, vinv)) = eigendecompose_from_schur(&t) else {
        return nan_clifford(gens);
    };

    // Build f(D) — diagonal matrix of f(eigenvalues)
    let eigvals = eigenvalues_from_triangular(&t);
    let mut fd_mat = MatC::zeros(dim);
    for (i, eig) in eigvals.iter().enumerate() {
        let f_eig = f(eig);
        if !f_eig.0.is_finite() || !f_eig.1.is_finite() {
            return nan_clifford(gens);
        }
        fd_mat.set(i, i, f_eig);
    }

    // result = Q * V * f(D) * V^{-1} * Q^H
    let qv = q.mul(&v);
    let fd_vinv = fd_mat.mul(&vinv);
    let qv_fd_vinv = qv.mul(&fd_vinv);
    let result_mat = qv_fd_vinv.mul(&q.dagger());

    from_matrix(&result_mat, gens)
}
