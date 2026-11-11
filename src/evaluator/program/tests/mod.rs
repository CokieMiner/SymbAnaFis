//! Scalar numerical execution contracts.

mod contracts;
mod differential;
mod disassembly;
mod functions;
mod multi;
mod native;
mod optimization;
mod pairs;
mod properties;
mod support;
mod validation;
mod wasm;
mod workspaces;

pub use wasm::WasmInspector;
