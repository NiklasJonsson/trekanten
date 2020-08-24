use glfw::{Action, Key};

use ash::vk;

use nalgebra_glm as glm;

use trekanten::mesh;
use trekanten::pipeline;
use trekanten::texture;
use trekanten::uniform;
use trekanten::window::Window;
use trekanten::Handle;
use trekanten::ResourceManager;
use trekanten::vertex::Vertex;

#[derive(Vertex)]
#[repr(C, packed)]
struct VertexTy {
    pos: glm::Vec2,
    col: glm::Vec3,
    tex_coord: glm::Vec2,
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
            vk::VertexInputAttributeDescription {
                binding: 0,
                location: 2,
                format: vk::Format::R32G32_SFLOAT,
                offset: memoffset::offset_of!(Vertex, tex_coord) as u32,
            },
        ]
    }
}

#[repr(C)]
struct UniformBufferObject {
    model: glm::Mat4,
    view: glm::Mat4,
    proj: glm::Mat4,
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
            tex_coord: glm::vec2(0.0, 0.0),
        },
        Vertex {
            pos: glm::vec2(0.5, -0.5),
            col: glm::vec3(0.0, 1.0, 0.0),
            tex_coord: glm::vec2(1.0, 0.0),
        },
        Vertex {
            pos: glm::vec2(0.5, 0.5),
            col: glm::vec3(0.0, 0.0, 1.0),
            tex_coord: glm::vec2(1.0, 1.0),
        },
        Vertex {
            pos: glm::vec2(-0.5, 0.5),
            col: glm::vec3(1.0, 1.0, 1.0),
            tex_coord: glm::vec2(0.0, 1.0),
        },
    ]
}

fn indices() -> Vec<u32> {
    vec![0, 1, 2, 2, 3, 0]
}

fn get_next_mvp(start: &std::time::Instant, aspect_ratio: f32) -> UniformBufferObject {
    let time = std::time::Instant::now() - *start;
    let time = time.as_secs_f32();

    let mut ubo = UniformBufferObject {
        model: glm::rotate(
            &glm::identity(),
            time * std::f32::consts::FRAC_PI_2,
            &glm::vec3(0.0, 0.0, 1.0),
        ),
        view: glm::look_at(
            &glm::vec3(2.0, 2.0, 2.0),
            &glm::vec3(0.0, 0.0, 0.0),
            &glm::vec3(0.0, 0.0, 1.0),
        ),
        proj: glm::perspective(std::f32::consts::FRAC_PI_4, aspect_ratio, 0.1, 10.0),
    };

    ubo.proj[(1, 1)] *= -1.0;

    ubo
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

    let pipeline_descriptor = pipeline::GraphicsPipelineDescriptor::builder()
        .vertex_shader("vert.spv")
        .fragment_shader("frag.spv")
        .vertex_type::<Vertex>()
        .build()
        .expect("Failed to create graphics pipeline desc");

    let gfx_pipeline_handle = renderer
        .create_resource(pipeline_descriptor)
        .expect("Failed to create graphics pipeline");

    let uniform_buffer_desc =
        uniform::UniformBufferDescriptor::uninitialized::<UniformBufferObject>(1);

    let uniform_buffer_handle = renderer
        .create_resource(uniform_buffer_desc)
        .expect("Failed to create uniform buffer");

    let texture_handle = renderer
        .create_resource(texture::TextureDescriptor::new(
            "textures/statue-1275469_640.jpg".into(),
        ))
        .expect("Failed to create texture");

    let desc_set_handle = renderer
        .create_descriptor_set(
            &gfx_pipeline_handle,
            &uniform_buffer_handle,
            &texture_handle,
        )
        .expect("Failed to create descriptor set");

    let start = std::time::Instant::now();
    while !window.window.should_close() {
        window.glfw.poll_events();
        for (_, event) in glfw::flush_messages(&window.events) {
            handle_window_event(&mut window.window, event);
        }

        let mut frame = match renderer.next_frame() {
            Err(trekanten::RenderError::NeedsResize(reason)) => {
                log::info!("Resize reason: {:?}", reason);
                renderer.resize(window.extents())?;
                renderer.next_frame()
            }
            x => x,
        }?;

        let next_mvp = get_next_mvp(&start, window.aspect_ratio());
        renderer
            .update_uniform(&uniform_buffer_handle, &next_mvp)
            .expect("Failed to update uniform buffer!");

        let render_pass = renderer.render_pass();
        let extent = renderer.swapchain_extent();
        let framebuffer = renderer.framebuffer(&frame);

        let gfx_pipeline = renderer
            .get_resource(&gfx_pipeline_handle)
            .expect("Missing graphics pipeline");
        let index_buffer = renderer
            .get_resource(&index_buffer_handle)
            .expect("Missing index buffer");
        let vertex_buffer = renderer
            .get_resource(&vertex_buffer_handle)
            .expect("Missing vertex buffer");
        let desc_set = renderer
            .get_descriptor_set(&desc_set_handle)
            .expect("Missing descriptor set");

        let cmd_buf = frame
            .new_command_buffer()?
            .begin_single_submit()?
            .begin_render_pass(render_pass, framebuffer, extent)
            .bind_graphics_pipeline(&gfx_pipeline)
            .bind_descriptor_set(&desc_set, &gfx_pipeline)
            .bind_index_buffer(&index_buffer)
            .bind_vertex_buffer(&vertex_buffer)
            .draw_indexed(indices.len() as u32)
            .end_render_pass()
            .end()?;

        frame.add_command_buffer(cmd_buf);

        renderer.submit(frame).or_else(|e| {
            if let trekanten::RenderError::NeedsResize(reason) = e {
                log::info!("Resize reason: {:?}", reason);
                renderer.resize(window.extents())
            } else {
                Err(e)
            }
        })?;
    }

    Ok(())
}
