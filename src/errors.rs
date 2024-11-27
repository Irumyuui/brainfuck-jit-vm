use std::fmt::Display;

#[derive(Debug, thiserror::Error)]
pub enum CompileErrorKind {
    #[error("Unclosed left bracket")]
    UnclosedLeftBracket,

    #[error("Unexpected right bracket")]
    UnexpectedRightBracket,
}

#[derive(Debug)]
pub struct CompileError {
    pub(crate) line: u32,
    pub(crate) col: u32,
    pub(crate) kind: CompileErrorKind,
}

impl Display for CompileError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "Error at line {}, column {}: {}",
            self.line, self.col, self.kind,
        )
    }
}

impl std::error::Error for CompileError {}

#[derive(Debug, thiserror::Error)]
pub enum RuntimeError {
    #[error("IO: {0}")]
    IO(#[from] std::io::Error),

    #[error("Pointer overflow")]
    PointerOverflow,
}

#[derive(Debug, thiserror::Error)]
pub enum VMError {
    #[error("IO: {0}")]
    IO(#[from] std::io::Error),

    #[error("Compile: {0}")]
    Compile(#[from] CompileError),

    #[error("Runtime: {0}")]
    Runtime(#[from] RuntimeError),
}
