pub mod types;
pub mod lexer;
pub mod parser;
pub mod evaluator;
pub mod format;
pub mod config;

pub use evaluator::{EvalContext, EvalError, evaluate_document};
pub use types::Value;
pub use config::Settings;
