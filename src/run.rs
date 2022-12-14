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
    event::{DeviceEvent, ElementState, Event, KeyboardInput, VirtualKeyCode, WindowEvent},
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
    } / 10;
    let mut initialized = false;
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
                WindowEvent::KeyboardInput {
                    input:
                        KeyboardInput {
                            virtual_keycode:
                                Some(vk @ (VirtualKeyCode::Plus | VirtualKeyCode::Minus)),
                            state: ElementState::Pressed,
                            ..
                        },
                    ..
                } => graphics.change_ray_splits(match vk {
                    VirtualKeyCode::Plus => 1,
                    VirtualKeyCode::Minus => -1,
                    _ => unreachable!(),
                }),
                WindowEvent::KeyboardInput { input: key, .. } => camera.key_input(key, slow_mode),
                WindowEvent::Focused(true) => capture_mouse = begin_capture_mouse(&window).is_ok(),
                WindowEvent::Focused(false) => {
                    stop_capture_mouse(&window);
                    capture_mouse = false;
                }
                _ => {}
            },
            Event::DeviceEvent {
                device_id: _,
                event: DeviceEvent::MouseMotion { delta: (dx, dy) },
            } => {
                if capture_mouse {
                    continue_capture_mouse(&window);
                    camera.mouse_input(dx, dy);
                }
            }
            Event::MainEventsCleared => {
                let now = Instant::now();
                if !initialized {
                    camera_timestamp = now;
                    physics_timestamp = now;
                    initialized = true;
                }
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
        .or_else(|_| window.set_cursor_grab(CursorGrabMode::Locked))
        .map_err(|_| ())?;
    window.set_cursor_visible(false);
    Ok(())
}
fn continue_capture_mouse(window: &Window) {
    let size = window.inner_size();
    let _ = window.set_cursor_position(PhysicalPosition::new(size.width / 2, size.height / 2));
}
fn stop_capture_mouse(window: &Window) {
    window.set_cursor_grab(CursorGrabMode::None).unwrap();
    window.set_cursor_visible(true);
}
