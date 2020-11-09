mod camera;
mod graphics;
mod physics;

use anyhow::*;
use async_std::task::block_on;
use camera::Camera;
use graphics::Graphics;
use physics::{Body, BODIES};
use rayon::prelude::*;
use std::time::Instant;
use winit::{
    dpi::PhysicalPosition,
    event::{Event, ModifiersState, WindowEvent},
    event_loop::{ControlFlow, EventLoop},
    window::{Window, WindowBuilder},
};

fn main() -> Result<(), anyhow::Error> {
    env_logger::init();
    let event_loop = EventLoop::new();
    let window = WindowBuilder::new()
        .with_title("Marble Gravity")
        .with_maximized(true)
        .build(&event_loop)
        .unwrap();

    let mut graphics = block_on(Graphics::initialize(&window))?;
    let mut camera = Camera::new();
    let mut bodies: Vec<Body> = (0..BODIES).into_iter().map(|_| Body::initial()).collect();
    let mut last_update = Instant::now();
    let mut capture_mouse = false;

    event_loop.run(move |event, _, control_flow| {
        *control_flow = ControlFlow::Poll;
        match event {
            Event::WindowEvent {
                window_id: _id,
                event: w_event,
            } => match w_event {
                WindowEvent::CloseRequested => *control_flow = ControlFlow::Exit,
                WindowEvent::Resized(new_size) => graphics.resize(new_size),
                WindowEvent::ModifiersChanged(mods) => {
                    if mods | ModifiersState::SHIFT == ModifiersState::SHIFT {
                        capture_mouse = begin_capture_mouse(&window).is_ok();
                    } else {
                        stop_capture_mouse(&window);
                        capture_mouse = false;
                    }
                }
                WindowEvent::KeyboardInput {
                    device_id: _,
                    input: key,
                    is_synthetic: _,
                } => camera.key_input(key),
                WindowEvent::CursorMoved {
                    device_id: _,
                    position: pos,
                    modifiers: _,
                } => {
                    if capture_mouse && continue_capture_mouse(&window) {
                        let size = window.inner_size();
                        camera.mouse_input(pos.x, pos.y, size.width, size.height);
                    }
                }
                _ => {}
            },
            Event::MainEventsCleared => {
                let now = Instant::now();
                let dt = now.duration_since(last_update).as_secs_f32();
                last_update = now;

                bodies = bodies
                    .par_iter()
                    .map(|body| body.update(&bodies, dt))
                    .collect();
                camera.update(dt);
                window.request_redraw();
            }
            Event::RedrawRequested(_window_id) => {
                graphics.render(&bodies, camera.world_to_camera())
            }
            _ => {}
        }
    });
}

fn begin_capture_mouse(window: &Window) -> Result<()> {
    window.set_cursor_grab(true)?;
    let size = window.inner_size();
    window.set_cursor_position(PhysicalPosition::new(size.width / 2, size.height / 2))?;
    window.set_cursor_visible(false);
    Ok(())
}
fn continue_capture_mouse(window: &Window) -> bool {
    let size = window.inner_size();
    window
        .set_cursor_position(PhysicalPosition::new(size.width / 2, size.height / 2))
        .is_ok()
}
fn stop_capture_mouse(window: &Window) {
    window.set_cursor_grab(false).unwrap();
    window.set_cursor_visible(true);
}
