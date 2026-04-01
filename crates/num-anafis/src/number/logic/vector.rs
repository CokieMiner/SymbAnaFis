#![allow(
    clippy::same_name_method,
    reason = "Trait and inherent methods deliberately share the same name for ergonomics"
)]
use core::ops::{Add, Mul, Neg, Sub};

/// A 3-dimensional vector generic over its components.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct Vector<T> {
    /// The x, y, and z components of the vector.
    pub components: [T; 3],
}

impl<T> Vector<T> {
    /// Construct a 3D vector.
    #[must_use]
    pub const fn new(x: T, y: T, z: T) -> Self {
        Self {
            components: [x, y, z],
        }
    }
}

impl<T: Add<Output = T>> Add for Vector<T> {
    type Output = Self;

    fn add(self, other: Self) -> Self::Output {
        let [x1, y1, z1] = self.components;
        let [x2, y2, z2] = other.components;
        Self {
            components: [x1 + x2, y1 + y2, z1 + z2],
        }
    }
}

// Implementation for references (only applicable if `T` types allow reference addition `&T + &T`)
impl<'vector, 'rhs, T> Add<&'rhs Vector<T>> for &'vector Vector<T>
where
    &'vector T: Add<&'rhs T, Output = T>,
{
    type Output = Vector<T>;

    fn add(self, other: &'rhs Vector<T>) -> Self::Output {
        Vector {
            components: [
                &self.components[0] + &other.components[0],
                &self.components[1] + &other.components[1],
                &self.components[2] + &other.components[2],
            ],
        }
    }
}

impl<T> Vector<T> {
    /// Component-wise addition.
    #[must_use]
    pub fn add<'vector, 'rhs>(&'vector self, other: &'rhs Self) -> Self
    where
        &'vector T: Add<&'rhs T, Output = T>,
    {
        self + other
    }
}

impl<T: Sub<Output = T>> Sub for Vector<T> {
    type Output = Self;

    fn sub(self, other: Self) -> Self::Output {
        let [x1, y1, z1] = self.components;
        let [x2, y2, z2] = other.components;
        Self {
            components: [x1 - x2, y1 - y2, z1 - z2],
        }
    }
}

impl<'vector, 'rhs, T> Sub<&'rhs Vector<T>> for &'vector Vector<T>
where
    &'vector T: Sub<&'rhs T, Output = T>,
{
    type Output = Vector<T>;

    fn sub(self, other: &'rhs Vector<T>) -> Self::Output {
        Vector {
            components: [
                &self.components[0] - &other.components[0],
                &self.components[1] - &other.components[1],
                &self.components[2] - &other.components[2],
            ],
        }
    }
}

impl<T: Neg<Output = T>> Neg for Vector<T> {
    type Output = Self;

    fn neg(self) -> Self::Output {
        let [x, y, z] = self.components;
        Self {
            components: [-x, -y, -z],
        }
    }
}

impl<'vector, T> Neg for &'vector Vector<T>
where
    &'vector T: Neg<Output = T>,
{
    type Output = Vector<T>;

    fn neg(self) -> Self::Output {
        Vector {
            components: [
                -&self.components[0],
                -&self.components[1],
                -&self.components[2],
            ],
        }
    }
}

impl<T: Mul<Output = T> + Clone> Mul<T> for Vector<T> {
    type Output = Self;

    fn mul(self, scalar: T) -> Self::Output {
        let [x, y, z] = self.components;
        Self {
            components: [x * scalar.clone(), y * scalar.clone(), z * scalar],
        }
    }
}

impl<'vector, 'rhs, T> Mul<&'rhs T> for &'vector Vector<T>
where
    &'vector T: Mul<&'rhs T, Output = T>,
{
    type Output = Vector<T>;

    fn mul(self, scalar: &'rhs T) -> Self::Output {
        Vector {
            components: [
                &self.components[0] * scalar,
                &self.components[1] * scalar,
                &self.components[2] * scalar,
            ],
        }
    }
}
