use std::{
    error::Error,
    fmt::{Debug, Display},
};

/// Error while binding a service to a provider
#[non_exhaustive]
#[derive(Debug)]
pub enum BindError {
    /// The service has already been bound to another provider
    ServiceBound(&'static str),
}

impl Error for BindError {}

impl Display for BindError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::ServiceBound(service) => {
                write!(f, "service `{service}` is already bound to a provider")
            }
        }
    }
}

#[non_exhaustive]
#[derive(Debug)]
pub enum UnbindError {
    /// The service is not bound to a provider
    ServiceUnbound(&'static str),
}

impl Error for UnbindError {}

impl Display for UnbindError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            UnbindError::ServiceUnbound(service) => {
                write!(f, "service `{service}` is not bound to a provider")
            }
        }
    }
}
