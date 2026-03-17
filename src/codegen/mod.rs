pub mod wasm;

/// Trait for emitting an IR to a text representation.
pub trait Emit {
    fn emit(&self) -> String;
}
