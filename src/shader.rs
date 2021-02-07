use crate::device::{Destroy, GraphicDevice};
use glow::HasContext;
use std::sync::mpsc::Sender;

pub struct Shader {
    pub(crate) program: u32,
    destroy: Sender<Destroy>,
}

impl Shader {
    pub fn from_source(device: &GraphicDevice, vertex: &str, fragment: &str) -> Self {
        // Create Shader program.
        let program = unsafe { device.gl.create_program().unwrap() };

        // Link shaders.
        let shader_sources = [
            (glow::VERTEX_SHADER, vertex),
            (glow::FRAGMENT_SHADER, fragment),
        ];

        let mut shaders = Vec::with_capacity(shader_sources.len());

        for (shader_type, shader_source) in shader_sources.iter() {
            unsafe {
                let shader = device.gl.create_shader(*shader_type).unwrap();
                device.gl.shader_source(shader, shader_source);
                device.gl.compile_shader(shader);
                if !device.gl.get_shader_compile_status(shader) {
                    panic!(device.gl.get_shader_info_log(shader));
                }
                device.gl.attach_shader(program, shader);
                shaders.push(shader);
            }
        }

        unsafe {
            device.gl.link_program(program);
            if !device.gl.get_program_link_status(program) {
                panic!(device.gl.get_program_info_log(program));
            }
        }

        // Once the shaders are linked to a program, it's safe to detach and delete them.
        for shader in shaders {
            unsafe {
                device.gl.detach_shader(program, shader);
                device.gl.delete_shader(shader);
            }
        }

        Self {
            program,
            destroy: device.destroy_sender(),
        }
    }
}

impl Drop for Shader {
    fn drop(&mut self) {
        self.destroy.send(Destroy::Shader(self.program)).unwrap();
    }
}
