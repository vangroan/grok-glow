use glutin::{
    dpi::LogicalSize,
    event::{Event, WindowEvent},
    event_loop::{ControlFlow, EventLoop},
    window::WindowBuilder,
    Api, ContextBuilder, GlProfile, GlRequest,
};
use grok_glow::{device::GraphicDevice, shader::Shader, sprite::Sprite, texture::Texture, utils};
use std::{
    error::Error,
    rc::Rc,
    time::{Duration, Instant},
};

fn main() -> Result<(), Box<dyn Error>> {
    // Create OpenGL context from window.
    let (graphics_device, event_loop, windowed_context) = {
        let el = glutin::event_loop::EventLoop::new();
        let wb = WindowBuilder::new()
            .with_title("Grok")
            .with_inner_size(LogicalSize::new(1024.0, 768.0));
        let windowed_context = ContextBuilder::new()
            .with_vsync(false)
            .with_gl(GlRequest::Specific(Api::OpenGl, (4, 6)))
            .with_gl_profile(GlProfile::Core)
            .build_windowed(wb, &el)?;
        let windowed_context = unsafe { windowed_context.make_current().unwrap() };
        let device = unsafe { GraphicDevice::from_windowed_context(&windowed_context) };
        (device, el, windowed_context)
    };

    println!("{}", graphics_device.opengl_info());

    // Shader
    // Shader is dropped after graphics device for some reason.
    let mut shader = Some(Shader::from_source(
        &graphics_device,
        include_str!("../src/sprite.vert"),
        include_str!("../src/sprite.frag"),
    ));

    // Sprite
    let sprite = {
        let img = image::open("src/test_pattern_2.png")?.to_rgba8();

        let mut texture = Texture::new(&graphics_device, img.width(), img.height())?;
        texture.update_data(&graphics_device, img.as_raw());

        let mut sprite = Sprite::with_size(&graphics_device, 64, 64);
        sprite.set_texture(Rc::new(texture));
        sprite
    };

    let mut sprites = vec![sprite];

    graphics_device.clear_screen([0.1, 0.2, 0.3, 1.0]);
    let mut last_time = Instant::now();
    let mut dt = Duration::from_millis(16); // Avoid divide by 0.
    let mut fps = utils::FpsCounter::new();

    event_loop.run(move |event, _, control_flow| {
        // *control_flow = ControlFlow::Wait;
        *control_flow = ControlFlow::Poll;
        match event {
            Event::LoopDestroyed => {
                sprites.clear();
                shader.take();
                return;
            }
            Event::MainEventsCleared => {
                windowed_context.window().request_redraw();
            }
            Event::RedrawRequested(_) => {
                let now = Instant::now();
                dt = now - last_time;
                last_time = now;
                fps.add(dt);

                // let dt_secs = dt.as_secs_f64();
                // let fps = 1.0 / dt.as_secs_f64();
                windowed_context
                    .window()
                    .set_title(&format!("Grok {:.0}fps", fps.fps()));

                graphics_device.maintain().unwrap();
                graphics_device.clear_screen([0.1, 0.2, 0.3, 1.0]);
                graphics_device.draw(&sprites, shader.as_ref().unwrap());

                // Important! Remember to swap the buffers else no drawing will show.
                windowed_context.swap_buffers().unwrap();
            }
            Event::WindowEvent { ref event, .. } => match event {
                WindowEvent::Resized(physical_size) => {
                    // Required on some platforms.
                    windowed_context.resize(*physical_size);

                    // Update viewport output.
                    graphics_device.set_viewport_size(*physical_size);
                }
                WindowEvent::CloseRequested => {
                    graphics_device.shutdown();
                    *control_flow = ControlFlow::Exit
                }
                _ => (),
            },
            _ => (),
        }
    });
}
