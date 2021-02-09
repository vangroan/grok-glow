use glutin::{
    dpi::LogicalSize,
    event::{Event, WindowEvent},
    event_loop::{ControlFlow, EventLoop},
    window::WindowBuilder,
    Api, ContextBuilder, GlProfile, GlRequest,
};
use grok_glow::sprite_batch::SpriteBatch;
use grok_glow::{
    device::GraphicDevice, shader::Shader, sprite::Sprite, texture::Texture,
    texture_pack::TexturePack, utils,
};
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
    // let mut sprites = vec![];

    // {
    //     let img = image::open("src/test_pattern_2.png")?.to_rgba8();
    //
    //     let mut texture = Texture::new(&graphics_device, img.width(), img.height())?;
    //     texture.update_data(&graphics_device, img.as_raw());
    //     let tex_rc = Rc::new(texture);
    //
    //     for y in 0..12 {
    //         for x in 0..16 {
    //             let mut sprite = Sprite::with_size(&graphics_device, x * 64, y * 64, 64, 64);
    //             sprite.set_texture(tex_rc.clone());
    //             sprites.push(sprite);
    //         }
    //     }
    // }

    // Sprite Batch
    let mut sprites = vec![];
    let mut sprite_batch = SpriteBatch::new(&graphics_device);

    {
        let img = image::open("src/test_pattern_2.png")?.to_rgba8();

        let mut texture = Texture::new(&graphics_device, img.width(), img.height())?;
        texture.update_data(&graphics_device, img.as_raw());

        for y in 0..12 {
            for x in 0..16 {
                let mut sprite = grok_glow::sprite_batch::Sprite::with([x * 64, y * 64], [64, 64]);
                sprite.set_texture(texture.clone());
                // sprites.push(sprite);
            }
        }
    }

    {
        let mut tex_pack = TexturePack::new(&graphics_device)?;
        let filenames = [
            "./examples/01.png",
            "./examples/03.png",
            "./examples/02.png",
        ];

        for (idx, filename) in filenames.iter().enumerate() {
            let img = image::open(filename)?.to_rgba8();
            let texture = tex_pack
                .add_image_data(&graphics_device, img.width(), img.height(), img.as_raw())
                .unwrap();
            let mut sprite =
                grok_glow::sprite_batch::Sprite::with([idx as i32 * 64, 64], [1024, 1024]);
            sprite.set_texture(texture);
            sprites.push(sprite);
        }
    }

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

                // Sprite must be added to the batch each draw call.
                for sprite in &sprites {
                    sprite_batch.add(sprite);
                }

                graphics_device.maintain().unwrap();
                graphics_device.clear_screen([0.1, 0.2, 0.3, 1.0]);
                // graphics_device.draw(&sprites, shader.as_ref().unwrap());
                sprite_batch.draw(&graphics_device, shader.as_ref().unwrap());

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
