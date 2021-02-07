use glow::HasContext;
use glutin::dpi::PhysicalSize;
use glutin::{
    dpi::LogicalSize,
    event::{Event, WindowEvent},
    event_loop::{ControlFlow, EventLoop},
    window::WindowBuilder,
    Api, ContextBuilder, GlProfile, GlRequest,
};
use grok_glow::{
    device::GraphicDevice,
    errors::{assert_gl, debug_assert_gl},
};
use image::GenericImageView;
use std::{error::Error, mem, slice};

unsafe fn as_u8<T>(buf: &[T]) -> &[u8] {
    let ptr = buf.as_ptr() as *const u8;
    let size = buf.len() * mem::size_of::<T>();
    slice::from_raw_parts(ptr, size)
}

struct Sprite {
    vertex_array: Option<glow::VertexArray>,
    texture: Option<glow::Texture>,
}

impl Sprite {
    fn new(gl: &glow::Context, effect: &Effect) -> Self {
        unsafe {
            // Vertex Array
            let vertex_array = gl.create_vertex_array().unwrap();
            gl.bind_vertex_array(Some(vertex_array));

            // Positions
            // let positions: &[[f32; 2]] = &[[-0.5, -0.5], [0.5, -0.5], [0.5, 0.5], [-0.5, 0.5]];
            let positions: &[[f32; 2]] = &[[100., 100.], [200., 100.], [200., 200.], [100., 200.]];
            let position_bytes = as_u8(positions);
            let position_buf = gl.create_buffer().unwrap();
            gl.bind_buffer(glow::ARRAY_BUFFER, Some(position_buf));
            gl.buffer_data_u8_slice(glow::ARRAY_BUFFER, position_bytes, glow::STATIC_DRAW);
            gl.enable_vertex_attrib_array(effect.pos_attr);
            gl.vertex_attrib_pointer_f32(
                effect.pos_attr, // Attribute location in shader program.
                2,               // Size. Components per iteration.
                glow::FLOAT,     // Type to get from buffer.
                false,           // Normalize.
                0,               // Stride. Bytes to advance each iteration.
                0,               // Offset. Bytes from start of buffer.
            );

            // UVs
            let uvs: &[[f32; 2]] = &[[0.0, 0.0], [1.0, 0.0], [1.0, 1.0], [0.0, 1.0]];
            let uv_bytes = as_u8(uvs);
            let uv_buf = gl.create_buffer().unwrap();
            gl.bind_buffer(glow::ARRAY_BUFFER, Some(uv_buf));
            gl.buffer_data_u8_slice(glow::ARRAY_BUFFER, uv_bytes, glow::STATIC_DRAW);
            gl.enable_vertex_attrib_array(effect.uv_attr);
            gl.vertex_attrib_pointer_f32(
                effect.uv_attr, // Attribute location in shader program.
                2,              // Size. Components per iteration.
                glow::FLOAT,    // Type to get from buffer.
                false,          // Normalize.
                0,              // Stride. Bytes to advance each iteration.
                0,              // Offset. Bytes from start of buffer.
            );

            // Indices.
            let indices: &[u8] = &[0, 1, 2, 0, 2, 3]; // Counter-clockwise
            let index_buf = gl.create_buffer().unwrap();
            gl.bind_buffer(glow::ELEMENT_ARRAY_BUFFER, Some(index_buf));
            gl.buffer_data_u8_slice(glow::ELEMENT_ARRAY_BUFFER, indices, glow::STATIC_DRAW);

            // Done with vertex array.
            gl.bind_vertex_array(None);

            // Texture
            let img = Self::load_image();
            println!("image ({} x {})", img.width(), img.height());
            println!("image length {}", img.as_raw().len());
            println!("image data {:?}", &img.as_raw());
            let clr: Vec<[u8; 4]> = vec![[128, 128, 255, 128]; 1024];
            let mut buf = vec![];
            for px in clr {
                buf.extend_from_slice(&px);
            }

            let texture = gl.create_texture().unwrap();
            gl.bind_texture(glow::TEXTURE_2D, Some(texture));
            gl.tex_image_2d(
                glow::TEXTURE_2D,
                0,                   // Mip level
                glow::RGBA8 as i32,  // Internal colour format
                img.width() as i32,  // Width in pixels
                img.height() as i32, // Height in pixels
                0,                   // border
                glow::RGBA,          // format
                glow::UNSIGNED_BYTE, // type
                // Some(&buf),          // data
                Some(img.as_raw()),
            );
            assert_gl(&gl);

            gl.tex_parameter_i32(
                glow::TEXTURE_2D,
                glow::TEXTURE_MIN_FILTER,
                glow::NEAREST as i32,
            );
            gl.tex_parameter_i32(
                glow::TEXTURE_2D,
                glow::TEXTURE_MAG_FILTER,
                glow::NEAREST as i32,
            );
            gl.tex_parameter_i32(
                glow::TEXTURE_2D,
                glow::TEXTURE_WRAP_S,
                glow::CLAMP_TO_EDGE as i32,
            );
            gl.tex_parameter_i32(
                glow::TEXTURE_2D,
                glow::TEXTURE_WRAP_T,
                glow::CLAMP_TO_EDGE as i32,
            );
            gl.bind_texture(glow::TEXTURE_2D, None);
            assert_gl(&gl);
            // let texture = 0;

            Self {
                vertex_array: Some(vertex_array),
                texture: Some(texture),
            }
        }
    }

    fn draw(&self, gl: &glow::Context) {
        if let (Some(vertex_array), Some(texture)) = (self.vertex_array, self.texture) {
            unsafe {
                gl.bind_vertex_array(Some(vertex_array));

                gl.active_texture(glow::TEXTURE0);
                gl.bind_texture(glow::TEXTURE_2D, Some(texture));

                // TODO: Model matrix
                // gl.uniform_1_i32(Some(&do_const_attr), 1);
                // gl.uniform_2_f32(Some(&1), 1024.0, 768.0);

                // gl.draw_arrays(glow::TRIANGLES, 0, 6);
                gl.draw_elements(glow::TRIANGLES, 6, glow::UNSIGNED_BYTE, 0);
                assert_gl(&gl);

                gl.bind_texture(glow::TEXTURE_2D, None);
                gl.bind_vertex_array(None);
            }
        }
    }

    fn load_image() -> image::RgbaImage {
        let img = image::open("src/test_pattern_2.png").unwrap();
        println!("{:?}", img.color());
        img.into_rgba8()
    }

    fn destroy(&mut self, gl: &glow::Context) {
        unsafe {
            if let Some(vertex_array) = self.vertex_array.take() {
                gl.delete_vertex_array(vertex_array);
            }

            if let Some(texture) = self.texture {
                gl.delete_texture(texture);
            }
        }
    }
}

#[must_use]
struct Effect {
    program: glow::Program,
    pos_attr: glow::UniformLocation,
    uv_attr: glow::UniformLocation,
    res_unif: glow::UniformLocation,
}

impl Effect {
    fn new(gl: &glow::Context) -> Self {
        // Create Shader program.
        let program = unsafe { gl.create_program().unwrap() };

        // Link shaders.
        let shader_sources = [
            (glow::VERTEX_SHADER, include_str!("../src/sprite.vert")),
            (glow::FRAGMENT_SHADER, include_str!("../src/sprite.frag")),
        ];

        let mut shaders = Vec::with_capacity(shader_sources.len());

        for (shader_type, shader_source) in shader_sources.iter() {
            unsafe {
                let shader = gl.create_shader(*shader_type).unwrap();
                gl.shader_source(shader, shader_source);
                gl.compile_shader(shader);
                if !gl.get_shader_compile_status(shader) {
                    panic!(gl.get_shader_info_log(shader));
                }
                gl.attach_shader(program, shader);
                shaders.push(shader);
            }
        }

        unsafe {
            gl.link_program(program);
            if !gl.get_program_link_status(program) {
                panic!(gl.get_program_info_log(program));
            }
        }

        // Once the shaders are linked to a program, it's safe to detach and delete them.
        for shader in shaders {
            unsafe {
                gl.detach_shader(program, shader);
                gl.delete_shader(shader);
            }
        }

        // Lookup attribute positions.
        let pos_attr = unsafe { gl.get_attrib_location(program, "a_Pos").unwrap() };

        // Lookup attribute texture coordinates.
        let uv_attr = unsafe { gl.get_attrib_location(program, "a_UV").unwrap() };
        // let uv_attr = 0;

        // Lookup uniform canvas dimensions.
        let res_unif = unsafe { gl.get_uniform_location(program, "u_Resolution").unwrap() };
        println!("res_unif {}", res_unif);
        Effect {
            program,
            pos_attr,
            uv_attr,
            res_unif,
        }
    }

    fn apply(&self, gl: &glow::Context, canvas_size: PhysicalSize<f32>) {
        unsafe {
            gl.use_program(Some(self.program));
            gl.uniform_2_f32(Some(&self.res_unif), canvas_size.width, canvas_size.height);
        }
    }

    fn destroy(&mut self, gl: &glow::Context) {
        unsafe {
            gl.delete_program(self.program);
        }
    }
}

fn main() -> Result<(), Box<dyn Error>> {
    // Create OpenGL context from window.
    let (gl, event_loop, windowed_context) = {
        let el = glutin::event_loop::EventLoop::new();
        let wb = WindowBuilder::new()
            .with_title("Grok")
            .with_inner_size(LogicalSize::new(1024.0, 768.0));
        let windowed_context = ContextBuilder::new()
            .with_vsync(true)
            .with_gl(GlRequest::Specific(Api::OpenGl, (4, 6)))
            .with_gl_profile(GlProfile::Core)
            .build_windowed(wb, &el)?;
        let windowed_context = unsafe { windowed_context.make_current().unwrap() };
        let gl_context = unsafe {
            glow::Context::from_loader_function(|s| {
                windowed_context.get_proc_address(s) as *const _
            })
        };
        (gl_context, el, windowed_context)
    };

    // Configure OpenGL
    unsafe {
        // Counter-clockwise winding
        gl.front_face(glow::CCW);
        // For troubleshooting
        // gl.enable(glow::CULL_FACE);
        // gl.cull_face(glow::BACK);
    }

    // Create Shader program.
    let shader_program = unsafe { gl.create_program()? };

    // Link shaders.
    let shader_sources = [
        (glow::VERTEX_SHADER, include_str!("../src/basic.vert")),
        (glow::FRAGMENT_SHADER, include_str!("../src/basic.frag")),
    ];

    let mut shaders = Vec::with_capacity(shader_sources.len());

    for (shader_type, shader_source) in shader_sources.iter() {
        unsafe {
            let shader = gl.create_shader(*shader_type)?;
            gl.shader_source(shader, shader_source);
            gl.compile_shader(shader);
            if !gl.get_shader_compile_status(shader) {
                panic!(gl.get_shader_info_log(shader));
            }
            gl.attach_shader(shader_program, shader);
            shaders.push(shader);
        }
    }

    unsafe {
        gl.link_program(shader_program);
        if !gl.get_program_link_status(shader_program) {
            panic!(gl.get_program_info_log(shader_program));
        }
    }

    // Once the shaders are linked to a program, it's safe to detach and delete them.
    for shader in shaders {
        unsafe {
            gl.detach_shader(shader_program, shader);
            gl.delete_shader(shader);
        }
    }

    // Lookup attribute positions.
    let position_attr = unsafe {
        gl.get_attrib_location(shader_program, "a_position")
            .unwrap()
    };

    // Lookup uniform positions.
    let do_const_attr = unsafe {
        gl.get_uniform_location(shader_program, "u_do_const")
            .unwrap()
    };

    let mut sprite_effect = Effect::new(&gl);
    let mut sprite = Sprite::new(&gl, &sprite_effect);

    // Create vertex array and data.
    let vertex_array = unsafe {
        let vertex_array = gl.create_vertex_array().unwrap();
        gl.bind_vertex_array(Some(vertex_array));

        // Position buffer.
        let positions: &[[f32; 2]] = &[[1.5, 1.0], [0.0, 0.0], [1.0, 0.0]];
        let positions = as_u8(positions);
        let position_buf = gl.create_buffer().unwrap();
        gl.bind_buffer(glow::ARRAY_BUFFER, Some(position_buf));
        gl.buffer_data_u8_slice(glow::ARRAY_BUFFER, positions, glow::STATIC_DRAW);
        // Turn on the buffer.
        gl.enable_vertex_attrib_array(position_attr);
        gl.vertex_attrib_pointer_f32(
            position_attr, // Attribute location in shader program.
            2,             // Size. Components per iteration.
            glow::FLOAT,   // Type to get from buffer.
            false,         // Normalize.
            0,             // Stride. Bytes to advance each iteration.
            0,             // Offset. Bytes from start of buffer.
        );
        assert_gl(&gl);

        // Indices
        let indices = &[0, 1, 2]; // Counter-clockwise
        let index_buf = gl.create_buffer().unwrap();
        gl.bind_buffer(glow::ELEMENT_ARRAY_BUFFER, Some(index_buf));
        gl.buffer_data_u8_slice(glow::ELEMENT_ARRAY_BUFFER, indices, glow::STATIC_DRAW);
        assert_gl(&gl);

        // Not needed for this example, but if we were to do vertex array operations
        // later we would be affecting this vertex array.
        gl.bind_vertex_array(None);

        vertex_array
    };

    unsafe {
        gl.clear_color(0.1, 0.2, 0.3, 1.0);
    }

    let mut quitting = false;
    event_loop.run(move |event, _, control_flow| {
        *control_flow = ControlFlow::Wait;
        match event {
            Event::LoopDestroyed => {
                return;
            }
            Event::MainEventsCleared => {
                windowed_context.window().request_redraw();
            }
            Event::RedrawRequested(_) => {
                if quitting {
                    // Prevent drawing after resources are delted.
                    return;
                }
                unsafe {
                    gl.clear(glow::COLOR_BUFFER_BIT);

                    gl.use_program(Some(shader_program));

                    gl.bind_vertex_array(Some(vertex_array));

                    // Select between constant vertex and vertex array.
                    // - 0: Use vertices sent via attribute buffer.
                    // - 1: Use vertices in shader.
                    gl.uniform_1_i32(Some(&do_const_attr), 1);

                    // gl.draw_arrays(glow::TRIANGLES, 0, 3);
                    gl.draw_elements(glow::TRIANGLES, 3, glow::UNSIGNED_BYTE, 0);

                    sprite_effect.apply(&gl, windowed_context.window().inner_size().cast());
                    sprite.draw(&gl);

                    windowed_context.swap_buffers().unwrap();
                }
            }
            Event::WindowEvent { ref event, .. } => match event {
                WindowEvent::Resized(physical_size) => {
                    windowed_context.resize(*physical_size);

                    let physical_size_i32 = physical_size.cast::<i32>();
                    unsafe {
                        gl.viewport(0, 0, physical_size_i32.width, physical_size_i32.height);
                    }
                }
                WindowEvent::CloseRequested => {
                    quitting = true;
                    unsafe {
                        gl.delete_program(shader_program);
                        gl.delete_vertex_array(vertex_array);
                        sprite_effect.destroy(&gl);
                        sprite.destroy(&gl);
                    }
                    *control_flow = ControlFlow::Exit
                }
                _ => (),
            },
            _ => (),
        }
    });
}
