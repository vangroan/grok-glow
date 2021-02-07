use glow::HasContext;
use std::fmt;

#[derive(Debug)]
pub enum Error {
    InvalidTextureSize(u32, u32),
    InvalidImageData { expected: usize, actual: usize },
    OpenGl(u32),
    OpenGlMessage(String),
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            Error::InvalidTextureSize(width, height) => write!(
                f,
                "Invalid texture size ({}, {}). Ensure that neither dimension is zero.",
                width, height
            ),
            Error::InvalidImageData { expected, actual } => write!(f, "Image data does not match texture storage size. Expected {} bytes. Actual {} bytes.", expected, actual),
            Error::OpenGl(error_code) => write!(f, "OpenGL Error: 0x{:x}", error_code),
            Error::OpenGlMessage(error_msg) => write!(f, "OpenGL Error: {}", error_msg),
        }
    }
}

impl std::error::Error for Error {}

pub type Result<T> = std::result::Result<T, Error>;

pub unsafe fn assert_gl(gl: &glow::Context) {
    let gl_err = gl.get_error();
    if gl_err != glow::NO_ERROR {
        panic!("OpenGL Error: 0x{:x}", gl_err);
    }
}

#[inline(always)]
pub unsafe fn debug_assert_gl<T>(gl: &glow::Context, value: T) -> T {
    #[cfg(debug_assertions)]
    {
        let gl_err = gl.get_error();
        if gl_err != glow::NO_ERROR {
            panic!("OpenGL Error: 0x{:x}", gl_err);
        }
    }

    value
}

#[inline(always)]
pub unsafe fn gl_result<T>(
    gl: &glow::Context,
    result: std::result::Result<T, String>,
) -> crate::errors::Result<T> {
    let gl_err = gl.get_error();
    if gl_err != glow::NO_ERROR {
        Err(crate::errors::Error::OpenGl(gl_err))
    } else {
        match result {
            Ok(value) => Ok(value),
            Err(message) => Err(crate::errors::Error::OpenGlMessage(message)),
        }
    }
}

#[inline(always)]
pub unsafe fn gl_error<T>(gl: &glow::Context, value: T) -> crate::errors::Result<T> {
    let gl_err = gl.get_error();
    if gl_err != glow::NO_ERROR {
        Err(crate::errors::Error::OpenGl(gl_err))
    } else {
        Ok(value)
    }
}
