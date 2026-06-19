pub mod analyzer;
pub mod builtin;
pub mod context;
pub mod decl;
pub mod error;
pub mod expr;
pub mod hir;
pub mod symbols;
pub mod test_utils;
pub mod types;

pub use analyzer::SemanticAnalyzer;

#[cfg(test)]
mod tests;
