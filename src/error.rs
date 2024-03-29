/// Errors thrown by this library
#[derive(Debug)]
pub enum Error {
    /// Error when using either oneshot or mpsc channels
    TokioSend(String),
    /// Error when receiving a confirmation of operation from a channel
    RecvError(std::sync::mpsc::RecvError),
    /// IO errors (for example, tcp errors)
    IO(std::io::Error),
    /// In theory, impossible error for Any to Input and Any to Output conversion
    WrongType,
    /// The replicas parameter supplied needs to be at least 1
    WrongReplicas,
    /// Indicates that there is no such demon with this location 
    InvalidLocation,
    /// Indicates that the address that is trying to be occupied is already taken
    OccupiedAddress,
    /// Indicates that communication with the demon could not be stablished (probably a broken channel)
    DemonCommunication
}

impl std::fmt::Display for Error {
    fn fmt(&self, formatter: &mut std::fmt::Formatter) -> Result<(), std::fmt::Error> {
        let content = match self {
            Error::TokioSend(detail) => format!("{}", detail),
            Error::RecvError(detail) => format!("{}", detail),
            Error::IO(e) => format!("{}", e),
            Error::WrongType => format!("a correct `Any` to `Input` or `Any` to `Output` downcast failed... contact this library's developer"),
            Error::WrongReplicas => format!("the minimum number of replicas needs to be 1"),
            Error::InvalidLocation => format!("the location is no longer valid"),
            Error::OccupiedAddress => format!("the location for this demon is already taken"),
            Error::DemonCommunication => format!("message to the demon could not be delivered")
        };
        write!(formatter, "{}", content)
    }
}

impl std::error::Error for Error {}