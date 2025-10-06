use thiserror::Error;

#[derive(Debug, Error)]
pub enum TexPackerError {
    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),
    #[error("Image error: {0}")]
    Image(#[from] image::ImageError),
    #[error("Invalid input: {0}")]
    InvalidInput(String),
    #[error("Texture too large or out of space for the atlas")]
    OutOfSpace,
    #[error("Nothing to pack")]
    Empty,
    #[error("Encoding error: {0}")]
    Encode(String),
}

pub type Result<T> = std::result::Result<T, TexPackerError>;
