//! Graphics device context.
use crate::{errors::debug_assert_gl, marker::Invariant};
use glow::HasContext;
use glutin::{dpi::PhysicalSize, PossiblyCurrent};
use std::collections::HashSet;
use std::{cell::Cell, fmt, marker::PhantomData, sync::mpsc};

pub struct GraphicDevice {
    pub(crate) gl: glow::Context,
    extensions: HashSet<String>,
    tx: mpsc::Sender<Destroy>,
    rx: mpsc::Receiver<Destroy>,
    size: Cell<PhysicalSize<u32>>,
    shutting_down: Cell<bool>,
    /// Inner OpenGL context has inner mutability, and is not thread safe.
    _invariant: Invariant,
}

impl GraphicDevice {
    pub fn new(gl: glow::Context) -> Self {
        let mut extensions = HashSet::new();

        // This implementation is taken from glow::Context::from_loader_function.
        let num_extensions = unsafe { gl.get_parameter_i32(glow::NUM_EXTENSIONS) };
        for i in 0..num_extensions {
            let extension_name =
                unsafe { gl.get_parameter_indexed_string(glow::EXTENSIONS, i as u32) };
            extensions.insert(extension_name);
        }

        println!("Extensions:");
        for ext in extensions.iter() {
            println!("  {}", ext);
        }

        // Ensure our preferred settings.
        unsafe {
            gl.front_face(glow::CCW); // Counter-clockwise winding.
                                      // gl.enable(glow::CULL_FACE);
                                      // gl.cull_face(glow::BACK);
        }

        // Dropped resources need to be deallocated via the OpenGL context.
        let (tx, rx) = mpsc::channel();

        Self {
            gl,
            extensions,
            tx,
            rx,
            size: Cell::new(PhysicalSize::new(640, 480)),
            shutting_down: Cell::new(false),
            _invariant: PhantomData,
        }
    }

    pub fn has_extension(&self, extension: &str) -> bool {
        self.extensions.contains(extension)
    }

    pub unsafe fn from_windowed_context(
        windowed_context: &glutin::WindowedContext<PossiblyCurrent>,
    ) -> Self {
        let gl = glow::Context::from_loader_function(|s| {
            windowed_context.get_proc_address(s) as *const _
        });

        let device = Self::new(gl);
        device.set_viewport_size(windowed_context.window().inner_size());

        device
    }

    pub fn opengl_info(&self) -> OpenGlInfo {
        unsafe {
            let version = self.gl.get_parameter_string(glow::VERSION);
            let vendor = self.gl.get_parameter_string(glow::VENDOR);
            let renderer = self.gl.get_parameter_string(glow::RENDERER);
            debug_assert_gl(&self.gl, ());

            OpenGlInfo {
                version,
                vendor,
                renderer,
            }
        }
    }

    pub(crate) fn destroy_sender(&self) -> mpsc::Sender<Destroy> {
        self.tx.clone()
    }

    pub fn set_viewport_size(&self, size: PhysicalSize<u32>) {
        self.size.set(size);
    }

    pub fn get_viewport_size(&self) -> PhysicalSize<u32> {
        self.size.get()
    }

    pub fn shutdown(&self) {
        self.shutting_down.set(true);
        self.maintain();
    }

    pub fn draw(&self, sprites: &[crate::sprite::Sprite], shader: &crate::shader::Shader) {
        // TODO: This drawing code may have to live in the render target.

        // Destroying resources before a draw will cause memory access errors.
        // FIXME: Test whether the drop and maintain prevents this.
        if self.shutting_down.get() {
            println!("Shutting down");
            return;
        }

        let canvas_size = self.size.get();

        unsafe {
            let physical_size_i32 = self.size.get().cast::<i32>();
            self.gl
                .viewport(0, 0, physical_size_i32.width, physical_size_i32.height);

            self.gl.use_program(Some(shader.program));

            // FIXME: Specific to the sprite shader.
            self.gl.uniform_2_f32(
                Some(&0),
                canvas_size.width as f32,
                canvas_size.height as f32,
            );
        }

        for sprite in sprites {
            unsafe {
                // Only sprites with textures are drawn.
                if let Some(texture_handle) = sprite.texture_handle() {
                    self.gl.bind_vertex_array(Some(sprite.vertex_buffer.vbo));

                    self.gl.active_texture(glow::TEXTURE0);
                    self.gl.bind_texture(glow::TEXTURE_2D, Some(texture_handle));

                    // FIXME: Unsigned short is a detail of the vertex buffer, so drawing should probably happen there.
                    self.gl
                        .draw_elements(glow::TRIANGLES, 6, glow::UNSIGNED_SHORT, 0);
                    debug_assert_gl(&self.gl, ());
                }
            }
        }

        // Cleanup
        unsafe {
            self.gl.bind_vertex_array(None);
            self.gl.use_program(None);
        }
    }

    pub fn clear_screen(&self, color: [f32; 4]) {
        unsafe {
            let physical_size_i32 = self.size.get().cast::<i32>();
            self.gl
                .viewport(0, 0, physical_size_i32.width, physical_size_i32.height);

            self.gl.clear_color(color[0], color[1], color[2], color[3]);
            self.gl.clear(glow::COLOR_BUFFER_BIT);
            debug_assert_gl(&self.gl, ());
        }
    }

    pub fn maintain(&self) -> crate::errors::Result<()> {
        while let Ok(resource) = self.rx.try_recv() {
            match resource {
                Destroy::Texture(handle) => unsafe {
                    println!("destroying texture");
                    self.gl.delete_texture(handle);
                },
                Destroy::Shader(program) => unsafe {
                    println!("destroying texture");
                    self.gl.delete_program(program);
                },
                Destroy::VertexArray(handle) => unsafe {
                    println!("destroying texture");
                    self.gl.delete_vertex_array(handle);
                },
            }
        }

        Ok(())
    }
}

pub(crate) enum Destroy {
    Texture(u32),
    Shader(u32),
    VertexArray(u32),
}

pub struct OpenGlInfo {
    pub version: String,
    pub vendor: String,
    pub renderer: String,
}

impl fmt::Display for OpenGlInfo {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        writeln!(f, "OpenGL Info:")?;
        writeln!(f, "    Version: {}", self.version)?;
        writeln!(f, "    Vendor: {}", self.vendor)?;
        writeln!(f, "    Renderer: {}", self.renderer)?;

        Ok(())
    }
}
