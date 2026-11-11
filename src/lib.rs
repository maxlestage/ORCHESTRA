//! ORCHESTRA : le compilateur Rust‑Compose ([`drc`]), le rendu HTML de ses résultats
//! ([`render`]) et, compilé en WebAssembly, le compilateur du navigateur.

pub mod drc;
pub mod render;

#[cfg(target_arch = "wasm32")]
mod wasm;
