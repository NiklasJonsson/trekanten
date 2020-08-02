use glfw::{Action, Key};

use trekanten::window::Window;

fn handle_window_event(window: &mut glfw::Window, event: glfw::WindowEvent) {
    match event {
        glfw::WindowEvent::Key(Key::Escape, _, Action::Press, _) => window.set_should_close(true),
        _ => {}
    }
}

fn main() -> Result<(), trekanten::RenderError> {
    env_logger::init();

    let mut window = trekanten::window::GlfwWindow::new();
    let mut renderer = trekanten::Renderer::new(&window)?;

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
        let gfx_pipeline = renderer.gfx_pipeline();
        let extent = renderer.swapchain_extent();
        let framebuffer = renderer.framebuffer(&frame);

        let cmd_buf = frame
            .new_command_buffer()?
            .begin()?
            .begin_render_pass(render_pass, framebuffer, extent)
            .bind_gfx_pipeline(&gfx_pipeline)
            .draw(3, 1, 0, 0)
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
