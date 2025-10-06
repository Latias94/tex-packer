use thiserror::Error;

#[derive(Debug, Error)]
pub enum TexPackerError {
    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),

    #[error("Image error: {0}")]
    Image(#[from] image::ImageError),

    #[error("Invalid input: {0}")]
    InvalidInput(String),

    #[error("Invalid configuration: {0}")]
    InvalidConfig(String),

    #[error("Texture '{key}' ({width}x{height}) exceeds maximum atlas dimensions ({max_width}x{max_height})")]
    TextureTooLarge {
        key: String,
        width: u32,
        height: u32,
        max_width: u32,
        max_height: u32,
    },

    #[error("Out of space: unable to fit texture '{key}' ({width}x{height}) into atlas (tried {pages_attempted} page(s))")]
    OutOfSpace {
        key: String,
        width: u32,
        height: u32,
        pages_attempted: usize,
    },

    #[error("Out of space: unable to fit remaining textures into atlas (placed {placed}/{total} textures)")]
    OutOfSpaceGeneric { placed: usize, total: usize },

    #[error("Nothing to pack: input list is empty")]
    Empty,

    #[error("Encoding error: {0}")]
    Encode(String),

    #[error("Invalid dimensions: width and height must be greater than 0 (got {width}x{height})")]
    InvalidDimensions { width: u32, height: u32 },

    #[error("Invalid padding configuration: border_padding ({border}) + texture_padding ({texture}) + texture_extrusion ({extrusion}) exceeds available space")]
    InvalidPadding {
        border: u32,
        texture: u32,
        extrusion: u32,
    },
}

pub type Result<T> = std::result::Result<T, TexPackerError>;
