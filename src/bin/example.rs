use glfw::{Action, Key};

use ash::vk;

use nalgebra_glm as glm;

use trekanten::material;
use trekanten::window::Window;

#[repr(C, packed)]
struct Vertex {
    pos: glm::Vec2,
    col: glm::Vec3,
}

impl trekanten::vertex::VertexDefinition for Vertex {
    fn binding_description() -> Vec<vk::VertexInputBindingDescription> {
        vec![vk::VertexInputBindingDescription {
            binding: 0,
            stride: std::mem::size_of::<Vertex>() as u32,
            input_rate: vk::VertexInputRate::VERTEX,
        }]
    }

    fn attribute_description() -> Vec<vk::VertexInputAttributeDescription> {
        vec![
            vk::VertexInputAttributeDescription {
                binding: 0,
                location: 0,
                format: vk::Format::R32G32_SFLOAT,
                offset: memoffset::offset_of!(Vertex, pos) as u32,
            },
            vk::VertexInputAttributeDescription {
                binding: 0,
                location: 1,
                format: vk::Format::R32G32B32_SFLOAT,
                offset: memoffset::offset_of!(Vertex, col) as u32,
            },
        ]
    }
}

// TODO:
// * Handle window requested resize
// * Wait while minimized
fn handle_window_event(window: &mut glfw::Window, event: glfw::WindowEvent) {
    match event {
        glfw::WindowEvent::Key(Key::Escape, _, Action::Press, _) => window.set_should_close(true),
        _ => {}
    }
}

fn vertex_buffer() -> Vec<Vertex> {
    vec![
        Vertex {
            pos: glm::vec2(0.0, -0.5),
            col: glm::vec3(1.0, 0.0, 0.0),
        },
        Vertex {
            pos: glm::vec2(0.5, 0.5),
            col: glm::vec3(0.0, 1.0, 0.0),
        },
        Vertex {
            pos: glm::vec2(-0.5, 0.5),
            col: glm::vec3(0.0, 0.0, 1.0),
        },
    ]
}

fn main() -> Result<(), trekanten::RenderError> {
    env_logger::init();

    let vertices = vertex_buffer();

    let mut window = trekanten::window::GlfwWindow::new();
    let mut renderer = trekanten::Renderer::new(&window)?;

    let vertex_buffer = renderer
        .vertex_buffer_from_slice(&vertices)
        .expect("Failed to create vertex buffer");

    let material_info = material::MaterialDescriptor::builder()
        .vertex_shader("vert.spv")
        .fragment_shader("frag.spv")
        .vertex_type::<Vertex>()
        .build()
        .expect("Failed to create material desc");

    let material_handle = renderer
        .create_material(material_info)
        .expect("Failed to create material");

    while !window.window.should_close() {
        window.glfw.poll_events();
        for (_, event) in glfw::flush_messages(&window.events) {
            handle_window_event(&mut window.window, event);
        }

        let mut frame = match renderer.next_frame() {
            Err(trekanten::RenderError::NeedsResize) => {
                renderer.resize(window.extents())?;
                renderer.next_frame()
            }
            x => x,
        }?;

        let render_pass = renderer.render_pass();

        let extent = renderer.swapchain_extent();
        let framebuffer = renderer.framebuffer(&frame);

        let material = renderer
            .get_material(&material_handle)
            .expect("Missing material");
        let cmd_buf = frame
            .new_command_buffer()?
            .begin_single_submit()?
            .begin_render_pass(render_pass, framebuffer, extent)
            .bind_material(&material)
            .bind_vertex_buffer(&vertex_buffer)
            .draw(vertices.len() as u32, 1, 0, 0)
            .end_render_pass()
            .end()?;

        frame.add_command_buffer(cmd_buf);

        renderer.submit(frame).or_else(|e| {
            if let trekanten::RenderError::NeedsResize = e {
                renderer.resize(window.extents())
            } else {
                Err(e)
            }
        })?;
    }

    Ok(())
}
