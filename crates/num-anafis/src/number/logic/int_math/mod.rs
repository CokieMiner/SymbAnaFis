#[cfg(any(feature = "backend_big_astro", feature = "backend_big_rug"))]
mod bigint_math;
#[cfg(feature = "backend32")]
mod i32_math;
#[cfg(all(
    not(feature = "backend32"),
    not(feature = "backend_big_astro"),
    not(feature = "backend_big_rug")
))]
mod i64_math;

mod api;

pub use api::*;
