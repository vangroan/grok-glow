use crate::{
    device::GraphicDevice,
    texture::Texture,
    vertex::{Vertex, VertexBuffer},
};
use std::rc::Rc;

/// Basically a drawable rectangle and texture.
pub struct Sprite {
    pub(crate) pos: [i32; 2],
    pub(crate) size: [u32; 2],
    pub(crate) vertex_buffer: VertexBuffer,
    pub(crate) texture: Option<Texture>,
}

impl Sprite {
    pub fn with_size(device: &GraphicDevice, x: i32, y: i32, width: u32, height: u32) -> Self {
        const WHITE: [f32; 4] = [1.0; 4];

        let [x, y] = [x as f32, y as f32];
        let [w, h] = [width as f32, height as f32];

        // FIXME: This is counter-clockwise winding.
        //        Since the shader is flipping the y-axis, and in the future
        //        a camera matrix may as well, we are actually mirroring
        //        the vertices and viewing the back.
        //        Even though we don't do backface culling, this may or may not be
        //        be ideal.
        let vertices = [
            Vertex {
                position: [x, y],
                uv: [0.0, 0.0],
                color: WHITE,
            },
            Vertex {
                position: [x + w, y],
                uv: [1.0, 0.0],
                color: WHITE,
            },
            Vertex {
                position: [x + w, y + h],
                uv: [1.0, 1.0],
                color: WHITE,
            },
            Vertex {
                position: [x, y + h],
                uv: [0.0, 1.0],
                color: WHITE,
            },
        ];

        // Counter-clockwise
        let indices = &[0, 1, 2, 0, 2, 3];

        Self {
            pos: [0, 0],
            size: [width, height],
            vertex_buffer: VertexBuffer::new_static(device, &vertices, indices),
            texture: None,
        }
    }

    pub fn set_texture(&mut self, texture: Texture) {
        self.texture = Some(texture);
    }

    pub(crate) unsafe fn texture_handle(&self) -> Option<u32> {
        self.texture.as_ref().map(|texture| texture.raw_handle())
    }
}
