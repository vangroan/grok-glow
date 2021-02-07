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
    pub(crate) texture: Option<Rc<Texture>>,
}

impl Sprite {
    pub fn with_size(device: &GraphicDevice, width: u32, height: u32) -> Self {
        const WHITE: [f32; 4] = [1.0; 4];

        let [w, h] = [width as f32, height as f32];

        let vertices = [
            Vertex {
                position: [0.0, 0.0],
                uv: [0.0, 0.0],
                color: WHITE,
            },
            Vertex {
                position: [w, 0.0],
                uv: [1.0, 0.0],
                color: WHITE,
            },
            Vertex {
                position: [w, h],
                uv: [1.0, 1.0],
                color: WHITE,
            },
            Vertex {
                position: [0.0, h],
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

    pub fn set_texture(&mut self, texture: Rc<Texture>) {
        self.texture = Some(texture);
    }

    pub(crate) unsafe fn texture_handle(&self) -> Option<u32> {
        self.texture.as_ref().map(|rc| rc.handle)
    }
}
