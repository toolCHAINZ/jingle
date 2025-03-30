#[cfg(feature = "pyo3")]
use pyo3::exceptions::PyRuntimeError;
#[cfg(feature = "pyo3")]
use pyo3::PyErr;
use thiserror::Error;

/// An error (usually from across the FFI boundary) in something involving sleigh
#[derive(Debug, Error)]
pub enum JingleSleighError {
    /// The sleigh compiler was run against a language definition that had some missing files.
    /// Probably indicates that the path to the language specification was wrong
    #[error("Unable to parse sleigh language!")]
    LanguageSpecRead,
    /// A language specification existed, but was unable to be parsed
    #[error("failed to parse sleigh language definition")]
    LanguageSpecParse(#[from] serde_xml_rs::Error),
    /// The user provided a sleigh language ID that has not been loaded
    #[error("that's not a valid language id")]
    InvalidLanguageId,
    /// Attempted to initialize sleigh but something went wrong
    #[error("Something went wrong putting bytes into sleigh: {0}")]
    SleighInitError(String),
    /// Unable to load the provided binary image for sleigh
    #[error("Something went wrong putting bytes into sleigh")]
    ImageLoadError,
    /// Attempted to initialize sleigh with an empty image
    #[error("You didn't provide any bytes to sleigh")]
    NoImageProvided,
    /// Sleigh encountered an error attempting to disassemble an instruction.
    /// This most likely just indicates an invalid opcode.
    #[error("Sleigh unable to decode an instruction")]
    InstructionDecode,
    /// A [`VarNode`](crate::VarNode) was constructed referencing a non-existent space
    #[error("A varnode was constructed referencing a non-existent space")]
    InvalidSpaceName,
    /// Attempted to construct an [Instruction](crate::Instruction) from an empty slice of instructions
    #[error("Attempted to construct an instruction from an empty slice of instructions")]
    EmptyInstruction,
    #[error("Failure to acquire mutex to sleigh FFI function")]
    SleighCompilerMutexError,
}

impl From<JingleSleighError> for std::fmt::Error {
    fn from(_value: JingleSleighError) -> Self {
        std::fmt::Error
    }
}

#[cfg(feature = "pyo3")]
impl From<JingleSleighError> for PyErr {
    fn from(value: JingleSleighError) -> Self {
        PyRuntimeError::new_err(value.to_string())
    }
}
