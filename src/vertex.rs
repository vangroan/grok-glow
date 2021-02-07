use crate::{
    device::{Destroy, GraphicDevice},
    errors::assert_gl,
    utils,
};
use glow::HasContext;
use std::{mem, sync::mpsc::Sender};

#[derive(Debug, Clone)]
pub struct Vertex {
    pub position: [f32; 2],
    pub uv: [f32; 2],
    pub color: [f32; 4],
}

/// Handle to a vertex buffer object located in video memory.
pub struct VertexBuffer {
    pub(crate) handle: u32,
    destroy: Sender<Destroy>,
}

impl VertexBuffer {
    // FIXME: Locations determined by sprite shader.
    const POSITION_LOC: u32 = 0;
    const UV_LOC: u32 = 1;
    const COLOR_LOC: u32 = 2;

    pub fn new_static(device: &GraphicDevice, vertices: &[Vertex], indices: &[u16]) -> Self {
        unsafe {
            // Vertex Buffer Object
            let vertex_array = device.gl.create_vertex_array().unwrap();
            device.gl.bind_vertex_array(Some(vertex_array));

            // Attached buffer space
            let buf = device.gl.create_buffer().unwrap();
            device.gl.bind_buffer(glow::ARRAY_BUFFER, Some(buf));
            device.gl.buffer_data_u8_slice(
                glow::ARRAY_BUFFER,
                utils::as_u8(vertices),
                glow::STATIC_DRAW,
            );
            assert_gl(&device.gl);

            // Vertex data is interleaved.
            // Attribute layout positions are determined by shader.
            // Positions
            device.gl.enable_vertex_attrib_array(Self::POSITION_LOC);
            device.gl.vertex_attrib_pointer_f32(
                Self::POSITION_LOC,              // Attribute location in shader program.
                2,                               // Size. Components per iteration.
                glow::FLOAT,                     // Type to get from buffer.
                false,                           // Normalize.
                mem::size_of::<Vertex>() as i32, // Stride. Bytes to advance each iteration.
                memoffset::offset_of!(Vertex, position) as i32, // Offset. Bytes from start of buffer.
            );
            assert_gl(&device.gl);

            // UVs
            device.gl.enable_vertex_attrib_array(Self::UV_LOC);
            device.gl.vertex_attrib_pointer_f32(
                Self::UV_LOC,                             // Attribute location in shader program.
                2,                                        // Size. Components per iteration.
                glow::FLOAT,                              // Type to get from buffer.
                false,                                    // Normalize.
                mem::size_of::<Vertex>() as i32, // Stride. Bytes to advance each iteration.
                memoffset::offset_of!(Vertex, uv) as i32, // Offset. Bytes from start of buffer.
            );
            assert_gl(&device.gl);

            // Colors
            device.gl.enable_vertex_attrib_array(Self::COLOR_LOC);
            device.gl.vertex_attrib_pointer_f32(
                Self::COLOR_LOC,                             // Attribute location in shader program.
                4,                                           // Size. Components per iteration.
                glow::FLOAT,                                 // Type to get from buffer.
                false,                                       // Normalize.
                mem::size_of::<Vertex>() as i32, // Stride. Bytes to advance each iteration.
                memoffset::offset_of!(Vertex, color) as i32, // Offset. Bytes from start of buffer.
            );
            assert_gl(&device.gl);

            // Indices
            let index_buf = device.gl.create_buffer().unwrap();
            device
                .gl
                .bind_buffer(glow::ELEMENT_ARRAY_BUFFER, Some(index_buf));
            device.gl.buffer_data_u8_slice(
                glow::ELEMENT_ARRAY_BUFFER,
                utils::as_u8(indices),
                glow::STATIC_DRAW,
            );

            device.gl.bind_buffer(glow::ARRAY_BUFFER, None);
            device.gl.bind_vertex_array(None);

            Self {
                handle: vertex_array,
                destroy: device.destroy_sender(),
            }
        }
    }
}

impl Drop for VertexBuffer {
    fn drop(&mut self) {
        self.destroy
            .send(Destroy::VertexArray(self.handle))
            .unwrap();
    }
}
