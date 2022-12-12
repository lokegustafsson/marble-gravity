mod camera;
mod graphics;
mod physics;
mod spheretree;

use camera::Camera;
use graphics::Graphics;
use physics::{Body, BODIES, PHYSICS_DELTA_TIME};
use rayon::prelude::*;
use spheretree::make_sphere_tree;
use std::time::{Duration, Instant};
use winit::{
    dpi::PhysicalPosition,
    event::{Event, WindowEvent},
    event_loop::{ControlFlow, EventLoop},
    window::{CursorGrabMode, Window, WindowBuilder},
};

const MAX_BEHIND: Duration = Duration::from_secs(1);

#[tokio::main(flavor = "current_thread")]
async fn main() {
    env_logger::init();
    let event_loop = EventLoop::new();
    let window = WindowBuilder::new()
        .with_title("Marble Gravity")
        .with_maximized(true)
        .build(&event_loop)
        .unwrap();

    let mut graphics = Graphics::initialize(&window).await;
    let mut camera = Camera::new();
    let mut bodies: Vec<Body> = (0..BODIES).into_iter().map(|_| Body::initial()).collect();
    let mut simulation_timestamp = Instant::now();
    let mut capture_mouse = false;
    let mut slow_mode = false;
    let mut last_redraw_request = Instant::now();

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
                    if mods.alt() || mods.logo() {
                        stop_capture_mouse(&window);
                        capture_mouse = false;
                    } else {
                        capture_mouse = begin_capture_mouse(&window).is_ok();
                    }
                    slow_mode = mods.ctrl();
                }
                WindowEvent::KeyboardInput { input: key, .. } => camera.key_input(key, slow_mode),
                WindowEvent::CursorMoved { position: pos, .. } => {
                    if capture_mouse && continue_capture_mouse(&window) {
                        let size = window.inner_size();
                        camera.mouse_input(pos.x, pos.y, size.width, size.height);
                    }
                }
                WindowEvent::Focused(true) => capture_mouse = begin_capture_mouse(&window).is_ok(),
                WindowEvent::Focused(false) => {
                    stop_capture_mouse(&window);
                    capture_mouse = false;
                }
                _ => {}
            },
            Event::MainEventsCleared => {
                let time_enter = Instant::now();
                let mut behind = time_enter.checked_duration_since(simulation_timestamp);

                while behind > Some(PHYSICS_DELTA_TIME) {
                    bodies = bodies.par_iter().map(|body| body.update(&bodies)).collect();
                    camera.update(PHYSICS_DELTA_TIME.as_secs_f32());

                    simulation_timestamp += PHYSICS_DELTA_TIME;
                    let new_behind = Instant::now().checked_duration_since(simulation_timestamp);
                    if new_behind > Some(MAX_BEHIND) && new_behind > behind {
                        println!("Physics computation too slow. Exiting...");
                        *control_flow = ControlFlow::Exit;
                        return;
                    }
                    behind = new_behind;
                }
                window.request_redraw();
            }
            Event::RedrawRequested(_window_id) => {
                if let Some(rate) = window
                    .current_monitor()
                    .and_then(|mon| mon.refresh_rate_millihertz())
                {
                    let span = Instant::now().duration_since(last_redraw_request);
                    let natural_span = Duration::from_secs(1000) / rate;
                    if span < natural_span {
                        std::thread::sleep(natural_span - span);
                    }
                }
                last_redraw_request = Instant::now();
                graphics.render(
                    make_sphere_tree(&bodies, camera.world_to_camera()),
                    camera.rotation(),
                )
            }
            _ => {}
        }
    });
}

fn begin_capture_mouse(window: &Window) -> Result<(), ()> {
    window
        .set_cursor_grab(CursorGrabMode::Confined)
        .map_err(|_| ())?;
    let size = window.inner_size();
    window
        .set_cursor_position(PhysicalPosition::new(size.width / 2, size.height / 2))
        .unwrap();

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
    window.set_cursor_grab(CursorGrabMode::None).unwrap();
    window.set_cursor_visible(true);
}
