use crate::{
    device::{Destroy, GraphicDevice},
    errors::{debug_assert_gl, gl_error, gl_result},
};
use glow::HasContext;
use std::sync::mpsc::Sender;

/// Handle to a texture located in video memory.
pub struct Texture {
    pub(crate) handle: u32,
    size: (u32, u32),
    destroy: Sender<Destroy>,
}

impl Texture {
    pub fn new(device: &GraphicDevice, width: u32, height: u32) -> crate::errors::Result<Self> {
        // Upfront validations.
        if width == 0 || height == 0 {
            return Err(crate::errors::Error::InvalidTextureSize(width, height));
        }

        unsafe {
            let handle = gl_result(&device.gl, device.gl.create_texture())?;
            device.gl.bind_texture(glow::TEXTURE_2D, Some(handle));

            // Allocate video memory for texture
            device.gl.tex_image_2d(
                glow::TEXTURE_2D,
                0,                   // Mip level
                glow::RGBA8 as i32,  // Internal colour format
                width as i32,        // Width in pixels
                height as i32,       // Height in pixels
                0,                   // Border
                glow::RGBA,          // Format
                glow::UNSIGNED_BYTE, // Color data type.
                None,                // Actual data can be uploaded later.
            );
            gl_error(&device.gl, ())?;

            device.gl.tex_parameter_i32(
                glow::TEXTURE_2D,
                glow::TEXTURE_MIN_FILTER,
                glow::NEAREST as i32,
            );
            device.gl.tex_parameter_i32(
                glow::TEXTURE_2D,
                glow::TEXTURE_MAG_FILTER,
                glow::NEAREST as i32,
            );
            device.gl.tex_parameter_i32(
                glow::TEXTURE_2D,
                glow::TEXTURE_WRAP_S,
                glow::CLAMP_TO_EDGE as i32,
            );
            device.gl.tex_parameter_i32(
                glow::TEXTURE_2D,
                glow::TEXTURE_WRAP_T,
                glow::CLAMP_TO_EDGE as i32,
            );
            device.gl.bind_texture(glow::TEXTURE_2D, None);

            Ok(Self {
                handle,
                size: (width, height),
                destroy: device.destroy_sender(),
            })
        }
    }

    /// Uploads image data to the texture's storage on the GPU device.
    pub fn update_data(
        &mut self,
        device: &GraphicDevice,
        data: &[u8],
    ) -> crate::errors::Result<()> {
        // TODO: Unbind GL_PIXEL_UNPACK_BUFFER
        //       https://www.khronos.org/opengl/wiki/GLAPI/glTexSubImage2D
        //       If a non-zero named buffer object is bound to the
        //       GL_PIXEL_UNPACK_BUFFER target (see glBindBuffer)
        //       while a texture image is specified, data is
        //       treated as a byte offset into the buffer object's
        //       data store.

        // Upfront validation
        if data.len() != self.data_len() {
            return Err(crate::errors::Error::InvalidImageData {
                expected: data.len(),
                actual: self.data_len(),
            });
        }

        unsafe {
            let _save = TextureSave::new(&device);

            device.gl.bind_texture(glow::TEXTURE_2D, Some(self.handle));
            device.gl.tex_sub_image_2d(
                glow::TEXTURE_2D,
                0,                   // level
                0,                   // x_offset
                0,                   // y_offset
                self.size.0 as i32,  // width
                self.size.1 as i32,  // height
                glow::RGBA,          // pixel format
                glow::UNSIGNED_BYTE, // color data type
                glow::PixelUnpackData::Slice(data),
            );
            gl_error(&device.gl, ())?;
        }

        Ok(())
    }

    /// Returns the number of bytes contained in the texture's storage.
    pub fn data_len(&self) -> usize {
        // Each pixel is 4 bytes, RGBA
        self.size.0 as usize * self.size.1 as usize * 4
    }
}

impl Drop for Texture {
    fn drop(&mut self) {
        self.destroy.send(Destroy::Texture(self.handle)).unwrap();
    }
}

/// Utility for saving the currently bound texture onto the call stack, and
/// restoring the binding on drop.
///
/// Used so that editing a texture does not disrupt a currently bound texture.
pub(crate) struct TextureSave<'a> {
    gl: &'a glow::Context,
    texture_handle: u32,
}

impl<'a> TextureSave<'a> {
    pub(crate) fn new(device: &'a GraphicDevice) -> Self {
        Self {
            gl: &device.gl,
            texture_handle: unsafe {
                // Get parameter failures are caused by incorrect parameter being passed in.
                debug_assert_gl(
                    &device.gl,
                    device.gl.get_parameter_i32(glow::TEXTURE_BINDING_2D) as u32,
                )
            },
        }
    }
}

impl<'a> Drop for TextureSave<'a> {
    fn drop(&mut self) {
        unsafe {
            self.gl
                .bind_texture(glow::TEXTURE_2D, Some(self.texture_handle));
        }
    }
}
