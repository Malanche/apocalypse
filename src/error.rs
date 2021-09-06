/// Errors thrown by this library
#[derive(Debug)]
pub enum Error {
    /// Error when using either oneshot or mpsc channels
    TokioSend(String),
    /// IO errors (for example, tcp errors)
    IO(std::io::Error),
    /// In theory, impossible error for Any to Input and Any to Output conversion
    WrongType,
    /// Indicates that there is no such demon with this location 
    InvalidLocation,
    /// Needs to be removed
    Unimplemented
}

impl std::fmt::Display for Error {
    fn fmt(&self, formatter: &mut std::fmt::Formatter) -> Result<(), std::fmt::Error> {
        let content = match self {
            Error::TokioSend(detail) => format!("{}", detail),
            Error::IO(e) => format!("{}", e),
            Error::WrongType => format!("A correct `Any` to `Input` or `Any` to `Output` downcast failed... contact this library's developer"),
            Error::InvalidLocation => format!("The location is no longer valid"),
            Error::Unimplemented => format!("Unimplemented feature!")
        };
        write!(formatter, "{}", content)
    }
}

impl std::error::Error for Error {}