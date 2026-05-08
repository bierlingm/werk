use thiserror::Error;

#[derive(Debug, Error)]
pub enum SigilError {
    #[error("construction error: {message} at {line}:{col}")]
    Construction {
        message: String,
        line: usize,
        col: usize,
    },
    #[error("unsupported feature: {feature}")]
    Unsupported { feature: String },
    #[error("unknown channel: {name}")]
    UnknownChannel { name: String },
    #[error("ir incompatible: stage={stage} expected={expected:?} actual={actual:?}")]
    IrIncompatible {
        stage: String,
        expected: crate::IrKind,
        actual: crate::IrKind,
    },
    #[error("render error: {message}")]
    Render { message: String },
    #[error("internal error: {message}")]
    Internal { message: String },
    #[error("io error: {message}")]
    Io { message: String },
}

impl SigilError {
    pub fn construction(message: impl Into<String>, line: usize, col: usize) -> Self {
        Self::Construction {
            message: message.into(),
            line,
            col,
        }
    }

    pub fn unsupported(feature: impl Into<String>) -> Self {
        Self::Unsupported {
            feature: feature.into(),
        }
    }

    pub fn render(message: impl Into<String>) -> Self {
        Self::Render {
            message: message.into(),
        }
    }

    pub fn internal(message: impl Into<String>) -> Self {
        Self::Internal {
            message: message.into(),
        }
    }

    pub fn io(message: impl Into<String>) -> Self {
        Self::Io {
            message: message.into(),
        }
    }
}
