use glfw::{Action, Key};

use ash::vk;

use nalgebra_glm as glm;

use trekanten::material;
use trekanten::mesh;
use trekanten::window::Window;
use trekanten::Handle;
use trekanten::ResourceManager;

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

fn vertices() -> Vec<Vertex> {
    vec![
        Vertex {
            pos: glm::vec2(-0.5, -0.5),
            col: glm::vec3(1.0, 0.0, 0.0),
        },
        Vertex {
            pos: glm::vec2(0.5, -0.5),
            col: glm::vec3(0.0, 1.0, 0.0),
        },
        Vertex {
            pos: glm::vec2(0.5, 0.5),
            col: glm::vec3(0.0, 0.0, 1.0),
        },
        Vertex {
            pos: glm::vec2(-0.5, 0.5),
            col: glm::vec3(1.0, 1.0, 1.0),
        },
    ]
}

fn indices() -> Vec<u32> {
    vec![0, 1, 2, 2, 3, 0]
}

fn main() -> Result<(), trekanten::RenderError> {
    env_logger::init();

    let vertices = vertices();
    let indices = indices();

    let mut window = trekanten::window::GlfwWindow::new();
    let mut renderer = trekanten::Renderer::new(&window)?;

    let vertex_buffer_descriptor = mesh::VertexBufferDescriptor::from_slice(&vertices);
    let vertex_buffer_handle: Handle<mesh::VertexBuffer> = renderer
        .create_resource(vertex_buffer_descriptor)
        .expect("Failed to create vertex buffer");

    let index_buffer_descriptor = mesh::IndexBufferDescriptor::from_slice(&indices);
    let index_buffer_handle = renderer
        .create_resource(index_buffer_descriptor)
        .expect("Failed to create index buffer");

    let material_info = material::MaterialDescriptor::builder()
        .vertex_shader("vert.spv")
        .fragment_shader("frag.spv")
        .vertex_type::<Vertex>()
        .build()
        .expect("Failed to create material desc");

    let material_handle = renderer
        .create_resource(material_info)
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
            .get_resource(&material_handle)
            .expect("Missing material");
        let index_buffer = renderer
            .get_resource(&index_buffer_handle)
            .expect("Missing index buffer");
        let vertex_buffer = renderer
            .get_resource(&vertex_buffer_handle)
            .expect("Missing vertex buffer");
        let cmd_buf = frame
            .new_command_buffer()?
            .begin_single_submit()?
            .begin_render_pass(render_pass, framebuffer, extent)
            .bind_material(&material)
            .bind_index_buffer(&index_buffer)
            .bind_vertex_buffer(&vertex_buffer)
            .draw_indexed(indices.len() as u32)
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
