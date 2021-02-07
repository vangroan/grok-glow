use crate::{
    device::GraphicDevice,
    errors::debug_assert_gl,
    shader::Shader,
    texture::Texture,
    utils,
    vertex::{Vertex, VertexBuffer},
};
use glow::HasContext;
use glutin::dpi::PhysicalSize;
use std::rc::Rc;

pub struct SpriteBatch {
    items: Vec<BatchItem>,
    vertices: Vec<Vertex>,
    indices: Vec<u16>,
    vertex_buffer: VertexBuffer,
}

impl SpriteBatch {
    // Vertex buffer size is unbounded, but uniform buffers have a size limit.
    // If we ever send matrices via uniform buffers, we would need to limit
    // the batch size accordingly.
    // https://www.khronos.org/opengl/wiki/Uniform_Buffer_Object#Limitations
    pub const BATCH_SIZE: usize = 2048;
    // pub const BATCH_SIZE: usize = 512;

    pub fn new(device: &GraphicDevice) -> Self {
        // 4 vertices per sprite
        let vertices = (0..Self::BATCH_SIZE * 4)
            .map(|_| Vertex {
                position: [0.0, 0.0],
                uv: [0.0, 0.0],
                color: [1.0, 1.0, 1.0, 1.0],
            })
            .collect::<Vec<_>>();

        // 2 triangles, 6 indices per sprite
        let mut indices: Vec<u16> = vec![];
        for i in 0..Self::BATCH_SIZE as u16 {
            indices.push(i);
            indices.push(i + 1);
            indices.push(i + 2);

            indices.push(i);
            indices.push(i + 2);
            indices.push(i + 3);
        }

        Self {
            items: Vec::with_capacity(Self::BATCH_SIZE),
            vertices: Vec::with_capacity(Self::BATCH_SIZE * 4),
            indices: Vec::with_capacity(Self::BATCH_SIZE * 6),
            vertex_buffer: VertexBuffer::new_static(device, &vertices, &indices),
        }
    }

    pub fn add(&mut self, sprite: &Sprite) {
        // Copies stuff needed for drawing to the internal batch item buffer.
        // Sprites without textures are not drawn anyway.
        if let Some(texture) = sprite.texture.as_ref() {
            let [x, y] = [sprite.pos[0] as f32, sprite.pos[1] as f32];
            let [w, h] = [sprite.size[0] as f32, sprite.size[1] as f32];

            self.items.push(BatchItem {
                pos: [x, y],
                size: [w, h],
                texture: texture.clone(),
            });
        }
    }

    pub fn draw(&mut self, device: &GraphicDevice, shader: &Shader) {
        // Nothing to draw.
        if self.items.is_empty() {
            return;
        }

        unsafe {
            let canvas_size = device.get_viewport_size();

            let physical_size_i32 = canvas_size.cast::<i32>();
            device
                .gl
                .viewport(0, 0, physical_size_i32.width, physical_size_i32.height);

            device.gl.use_program(Some(shader.program));

            // FIXME: Specific to the sprite shader.
            device.gl.uniform_2_f32(
                Some(&0),
                canvas_size.width as f32,
                canvas_size.height as f32,
            );
        }

        unsafe {
            device.gl.bind_vertex_array(Some(self.vertex_buffer.vbo));
        }

        let SpriteBatch {
            items,
            vertices,
            indices,
            vertex_buffer,
        } = self;

        let mut batch_count = 0;
        let mut last_texture = None;

        for item in items.drain(..) {
            // println!("### BATCH {} ###", batch_count);

            if batch_count >= Self::BATCH_SIZE {
                Self::flush(device, vertex_buffer, &vertices, &indices);
                vertices.clear();
                indices.clear();
                batch_count = 0;
            }

            // The buffer is flushed each time we encounter a new texture.
            if last_texture != Some(item.texture.handle) {
                Self::flush(device, vertex_buffer, &vertices, &indices);
                vertices.clear();
                indices.clear();
                batch_count = 0;
                last_texture = Some(item.texture.handle);
                unsafe {
                    // Texture slot determined by sprite shader.
                    device.gl.active_texture(glow::TEXTURE0);
                    device
                        .gl
                        .bind_texture(glow::TEXTURE_2D, Some(item.texture.handle));
                }
            }

            let BatchItem {
                pos: [x, y],
                size: [w, h],
                ..
            } = item;
            // println!("{:?} {:?}", [x, y], [w, h]);

            // Build vertices from sprite parameters.
            vertices.push(Vertex {
                position: [x, y],
                uv: [0.0, 0.0],
                color: [1.0, 1.0, 1.0, 1.0],
            });
            vertices.push(Vertex {
                position: [x + w, y],
                uv: [1.0, 0.0],
                color: [1.0, 1.0, 1.0, 1.0],
            });
            vertices.push(Vertex {
                position: [x + w, y + h],
                uv: [1.0, 1.0],
                color: [1.0, 1.0, 1.0, 1.0],
            });
            vertices.push(Vertex {
                position: [x, y + h],
                uv: [0.0, 1.0],
                color: [1.0, 1.0, 1.0, 1.0],
            });
            // println!("{:?}", &vertices[vertices.len() - 4..vertices.len()]);

            let i = batch_count as u16 * 4;
            indices.push(i);
            indices.push(i + 1);
            indices.push(i + 2);
            indices.push(i + 0);
            indices.push(i + 2);
            indices.push(i + 3);
            // println!("{:?}", &indices[indices.len() - 6..indices.len()]);

            batch_count += 1;
        }

        // Flush the last sprites that didn't reach the threshold.
        if batch_count > 0 {
            Self::flush(device, vertex_buffer, &vertices, &indices);
            vertices.clear();
            indices.clear();
            batch_count = 0;
        }

        unsafe {
            device.gl.bind_texture(glow::TEXTURE_2D, None);
            device.gl.bind_vertex_array(None);
            device.gl.use_program(None);
        }
    }

    /// this is where the actual drawing will happen.
    fn flush(
        device: &GraphicDevice,
        vertex_buf: &VertexBuffer,
        vertices: &[Vertex],
        indices: &[u16],
    ) {
        if vertices.is_empty() {
            // Nothing to draw
            return;
        }

        debug_assert!(vertices.len() / 4 == indices.len() / 6);

        unsafe {
            // Upload new data.
            device
                .gl
                .bind_buffer(glow::ARRAY_BUFFER, Some(vertex_buf.vertex_buffer));
            device
                .gl
                .buffer_sub_data_u8_slice(glow::ARRAY_BUFFER, 0, &utils::as_u8(vertices));
            debug_assert_gl(&device.gl, ());

            device
                .gl
                .bind_buffer(glow::ELEMENT_ARRAY_BUFFER, Some(vertex_buf.index_buffer));
            device.gl.buffer_sub_data_u8_slice(
                glow::ELEMENT_ARRAY_BUFFER,
                0,
                &utils::as_u8(indices),
            );
            debug_assert_gl(&device.gl, ());

            // FIXME: Unsigned short is a detail of the vertex buffer, so drawing should probably happen there.
            device.gl.draw_elements(
                glow::TRIANGLES,
                indices.len() as i32,
                glow::UNSIGNED_SHORT,
                0,
            );
            debug_assert_gl(&device.gl, ());
        }
    }
}

/// Batch specific sprite. Could replace current implementation.
pub struct Sprite {
    pub(crate) pos: [i32; 2],
    pub(crate) size: [u32; 2],
    pub(crate) texture: Option<Rc<Texture>>,
}

impl Sprite {
    pub fn with(pos: [i32; 2], size: [u32; 2]) -> Self {
        Self {
            pos,
            size,
            texture: None,
        }
    }

    pub fn set_texture(&mut self, texture: Rc<Texture>) {
        self.texture = Some(texture);
    }
}

struct BatchItem {
    pos: [f32; 2],
    size: [f32; 2],
    texture: Rc<Texture>,
}
