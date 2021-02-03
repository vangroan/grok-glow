use glutin::{
    dpi::LogicalSize,
    event_loop::{EventLoop, ControlFlow},
    window::WindowBuilder,
    event::{Event, WindowEvent},
    ContextBuilder,
    GlProfile,
    GlRequest,
    Api,
};
use std::{error::Error, mem, slice};
use glow::HasContext;

unsafe fn as_u8<T>(buf: &[T]) -> &[u8] {
    let ptr = buf.as_ptr() as *const u8;
    let size = buf.len() * mem::size_of::<T>();
    slice::from_raw_parts(ptr, size)
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

    // Create Shader program.
    let shader_program = unsafe {
        gl.create_program()?
    };

    // Link shaders.
    let shader_sources = [
        (glow::VERTEX_SHADER, include_str!("basic.vertex")),
        (glow::FRAGMENT_SHADER, include_str!("basic.fragment")),
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
        gl.get_attrib_location(shader_program, "a_position").unwrap()
    };

    // Lookup uniform positions.
    let do_const_attr = unsafe {
        gl.get_uniform_location(shader_program, "u_do_const").unwrap()
    };

    // Create vertex array and data.
    let vertex_array = unsafe {
        let vertex_array = gl.create_vertex_array().unwrap();
        gl.bind_vertex_array(Some(vertex_array));

        // Position buffer.
        let positions: &[[f32; 2]] = &[
            [1.5, 1.0],
            [0.0, 0.0],
            [1.0, 0.0],
        ];
        let positions = as_u8(positions);
        let position_buf = gl.create_buffer().unwrap();
        gl.bind_buffer(glow::ARRAY_BUFFER, Some(position_buf));
        gl.buffer_data_u8_slice(glow::ARRAY_BUFFER, positions, glow::STATIC_DRAW);
        // Turn on the buffer.
        gl.enable_vertex_attrib_array(position_attr);
        gl.vertex_attrib_pointer_f32(
            position_attr, // Attribute location in shader program.
            2, // Size. Components per iteration.
            glow::FLOAT, // Type to get from buffer.
            false, // Normalize.
            0, // Stride. Bytes to advance each iteration.
            0, // Offset. Bytes from start of buffer.
        );

        // Not needed for this example, but if we were to do vertex array operations
        // later we would be affecting this vertex array.
        gl.bind_vertex_array(Some(vertex_array));

        vertex_array
    };

    unsafe {
        gl.clear_color(0.1, 0.2, 0.3, 1.0);
    }

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
                unsafe {
                    gl.clear(glow::COLOR_BUFFER_BIT);

                    gl.use_program(Some(shader_program));

                    gl.bind_vertex_array(Some(vertex_array));

                    // Select between constant vertex and vertex array.
                    gl.uniform_1_i32(Some(&do_const_attr), 1);

                    gl.draw_arrays(glow::TRIANGLES, 0, 3);

                    windowed_context.swap_buffers().unwrap();
                }
            }
            Event::WindowEvent { ref event, .. } => match event {
                WindowEvent::Resized(physical_size) => {
                    windowed_context.resize(*physical_size);
                }
                WindowEvent::CloseRequested => {
                    unsafe {
                        gl.delete_program(shader_program);
                        gl.delete_vertex_array(vertex_array);
                    }
                    *control_flow = ControlFlow::Exit
                }
                _ => (),
            },
            _ => ()
        }
    });
}
