use crate::{
    camera::Camera,
    graphics::Graphics,
    physics::{Body, BODIES, PHYSICS_DELTA_TIME},
    spheretree, PHYSICS_MAX_BEHIND_TIME,
};
use instant::Instant;
use std::time::Duration;
use winit::{
    dpi::PhysicalPosition,
    event::{Event, WindowEvent},
    event_loop::{ControlFlow, EventLoop},
    window::{CursorGrabMode, Window},
};

pub fn run(event_loop: EventLoop<()>, window: Window, mut graphics: Graphics) {
    let mut camera = Camera::new();
    let mut bodies: Vec<Body> = (0..BODIES).into_iter().map(|_| Body::initial()).collect();
    let mut capture_mouse = false;
    let mut slow_mode = false;

    let desired_frame_time = match window
        .current_monitor()
        .and_then(|mon| mon.refresh_rate_millihertz())
    {
        Some(rate) => Duration::from_secs(1000) / rate,
        None => Duration::from_secs(1) / 60,
    };
    let mut last_frame_processing_begun_instant = Instant::now();
    let mut physics_timestamp = last_frame_processing_begun_instant;
    let mut camera_timestamp = last_frame_processing_begun_instant;

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
                let now = Instant::now();
                camera_timestamp += camera.update_return_stepped(now - camera_timestamp);
                if now < last_frame_processing_begun_instant + desired_frame_time {
                    control_flow
                        .set_wait_until(last_frame_processing_begun_instant + desired_frame_time);
                    return;
                }
                last_frame_processing_begun_instant = now;

                if now.checked_duration_since(physics_timestamp) > Some(PHYSICS_MAX_BEHIND_TIME) {
                    log::error!(
                        "Physics computation too far behind ({}ms). Exiting..",
                        PHYSICS_MAX_BEHIND_TIME.as_millis()
                    );
                    control_flow.set_exit();
                }
                while now.checked_duration_since(physics_timestamp) > Some(PHYSICS_DELTA_TIME) {
                    bodies = bodies.iter().map(|body| body.update(&bodies)).collect();
                    physics_timestamp += PHYSICS_DELTA_TIME;
                }
                window.request_redraw();
            }
            Event::RedrawRequested(_window_id) => {
                graphics.render(
                    spheretree::make_sphere_tree(&bodies, camera.world_to_camera()),
                    camera.rotation(),
                );
                graphics.report_frame_time_multiple(
                    Instant::now()
                        .duration_since(last_frame_processing_begun_instant)
                        .as_secs_f64()
                        / desired_frame_time.as_secs_f64(),
                );
                control_flow
                    .set_wait_until(last_frame_processing_begun_instant + desired_frame_time);
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
