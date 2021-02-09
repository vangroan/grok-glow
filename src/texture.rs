use crate::{
    device::{Destroy, GraphicDevice},
    errors::{self, debug_assert_gl, gl_error, gl_result},
    marker::Invariant,
    rect::Rect,
};
use glow::HasContext;
use std::{cell::RefCell, rc::Rc, sync::mpsc::Sender};

/// Handle to a texture located in video memory.
#[derive(Clone)]
pub struct Texture {
    /// Handle to texture allocated in video memory.
    /// We keep a copy of the handle inlined in the struct
    /// to save on a pointer jump during drawing, but the
    /// handle is really owned by the `Rc`.
    texture: glow::Texture,
    /// Total size in texels of the whole texture in video memory.
    /// We need to keep this around for UVs coordinates calculations.
    orig_size: [u32; 2],
    /// Sub-rectangle representing the view of this texture into
    /// the complete texture.
    ///
    /// Must be equal or smaller than `orig_size`.
    rect: Rect<u32>,
    /// Handle to texture allocated in video memory, behind
    /// a reference counted pointed. The `Rc` manages ownership
    /// and triggers a deallocate in video memory when all
    /// references are released.
    handle: Rc<RefCell<TextureHandle>>,
}

impl Texture {
    pub fn new(device: &GraphicDevice, width: u32, height: u32) -> errors::Result<Self> {
        // Upfront validations.
        Self::validate_size(width, height)?;

        // When non-power-of-two textures are not available, several
        // bad things can happen from degraded performance to OpenGL
        // errors.
        if !Self::is_npot_available(device) {
            if !Self::is_power_of_two(width) || !Self::is_power_of_two(height) {
                return Err(crate::errors::Error::InvalidTextureSize(width, height));
            }
        }

        // Important: Non power of two textures may not have mipmaps

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

            // Match the allocated texture.
            let rect = Rect {
                pos: [0, 0],
                size: [width, height],
            };

            Ok(Self {
                texture: handle,
                orig_size: [width, height],
                rect,
                handle: Rc::new(RefCell::new(TextureHandle {
                    handle,
                    size: [width, height],
                    destroy: device.destroy_sender(),
                    _invariant: Default::default(),
                })),
            })
        }
    }

    /// Create a sub texture from the given texture view.
    ///
    /// Does not allocate new texture space in video memory.
    /// Instead creates a view into the same memory backed
    /// by `source`.
    ///
    /// # Errors
    ///
    /// Returns `InvalidSubTexture` if the given position and
    /// size do not fit inside the source texture.
    ///
    /// Returns `InvalidTextureSize` if any given dimension is 0
    /// or invalid for the current graphic device.
    pub fn new_sub(&self, pos: [u32; 2], size: [u32; 2]) -> errors::Result<Self> {
        let target_rect = Rect { pos, size };

        if !self.rect.can_fit(&target_rect) {
            return Err(errors::Error::InvalidSubTexture {
                source: self.rect,
                target: target_rect,
            });
        }

        Self::validate_size(size[0], size[1])?;

        // We can probably get away without checking power-of-two since we're not
        // allocating video memory.

        Ok(Self {
            texture: self.texture,
            orig_size: self.orig_size,
            rect: target_rect,
            handle: self.handle.clone(),
        })
    }

    fn validate_size(width: u32, height: u32) -> errors::Result<()> {
        if width == 0 || height == 0 {
            return Err(crate::errors::Error::InvalidTextureSize(width, height));
        }

        Ok(())
    }

    fn is_power_of_two(n: u32) -> bool {
        // This bitwise test does not work on the number zero.
        n != 0 && ((n & n - 1) == 0)
    }

    /// Queries the device support for non-power-of-two-textures.
    pub fn is_npot_available(device: &GraphicDevice) -> bool {
        device.has_extension("GL_ARB_texture_non_power_of_two")
    }

    pub fn raw_handle(&self) -> glow::Texture {
        self.handle.borrow().handle
    }

    pub fn update_data(
        &mut self,
        device: &GraphicDevice,
        data: &[u8],
    ) -> crate::errors::Result<()> {
        let size = self.handle.borrow().size;
        self.update_sub_data(device, [0, 0], size, data)
    }

    /// Uploads image data to the texture's storage on the GPU device.
    pub fn update_sub_data(
        &mut self,
        device: &GraphicDevice,
        pos: [u32; 2],
        size: [u32; 2],
        data: &[u8],
    ) -> crate::errors::Result<()> {
        // TODO: Unbind GL_PIXEL_UNPACK_BUFFER
        //       https://www.khronos.org/opengl/wiki/GLAPI/glTexSubImage2D
        //       If a non-zero named buffer object is bound to the
        //       GL_PIXEL_UNPACK_BUFFER target (see glBindBuffer)
        //       while a texture image is specified, data is
        //       treated as a byte offset into the buffer object's
        //       data store.

        // TODO: Validate given pos and size against target texture rectangle. Must fit.

        // Upfront validation
        let expected_len = size[0] as usize * size[1] as usize * 4;
        if data.len() != expected_len {
            return Err(crate::errors::Error::InvalidImageData {
                expected: expected_len,
                actual: data.len(),
            });
        }

        // Borrow mut to enforce runtime borrow rules.
        let handle = self.handle.borrow_mut();

        unsafe {
            let _save = TextureSave::new(&device);

            device
                .gl
                .bind_texture(glow::TEXTURE_2D, Some(handle.handle));
            device.gl.tex_sub_image_2d(
                glow::TEXTURE_2D,
                0,                   // level
                pos[0] as i32,       // x_offset
                pos[1] as i32,       // y_offset
                size[0] as i32,      // width
                size[1] as i32,      // height
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
        let size = self.handle.borrow().size;
        // Each pixel is 4 bytes, RGBA
        size[0] as usize * size[1] as usize * 4
    }
}

impl Drop for Texture {
    fn drop(&mut self) {
        // self.destroy.send(Destroy::Texture(self.handle)).unwrap();
    }
}

/// Wrapper for a handle to a texture in video memory.
///
/// This wrapper is considered the owner of the video memory, and
/// is responsible for triggering a deallocate on drop.
struct TextureHandle {
    handle: glow::Texture,
    size: [u32; 2],
    destroy: Sender<Destroy>,
    _invariant: Invariant,
}

impl Drop for TextureHandle {
    fn drop(&mut self) {
        self.destroy.send(Destroy::Texture(self.handle)).expect("TextureHandle dropped, but channel closed. OpenGL context was possibly terminated with dangling resources.");
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
