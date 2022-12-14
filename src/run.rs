use crate::{
    camera::Camera,
    graphics::Graphics,
    physics::{Body, BODIES, PHYSICS_DELTA_TIME},
    spheretree, PHYSICS_MAX_BEHIND_TIME,
};
use instant::Instant;
use std::time::Duration;
use winit::{
    dpi::{PhysicalPosition, PhysicalSize},
    event::{
        DeviceEvent, ElementState, Event, KeyboardInput, MouseButton, VirtualKeyCode, WindowEvent,
    },
    event_loop::{ControlFlow, EventLoop},
    window::{CursorGrabMode, Window},
};

pub fn run(event_loop: EventLoop<()>, window: Window, mut graphics: Graphics) {
    let mut camera = Camera::new();
    let mut bodies: Vec<Body> = (0..BODIES).into_iter().map(|_| Body::initial()).collect();
    let mut capture_mouse = false;
    let mut slow_mode = false;

    const DESIRED_FRAME_MULTIPLE: u32 = if cfg!(target_arch = "wasm32") { 2 } else { 1 };
    let desired_frame_time = match window
        .current_monitor()
        .and_then(|mon| mon.refresh_rate_millihertz())
    {
        Some(rate) => Duration::from_secs(1000) / rate,
        None => Duration::from_secs(1) / 60,
    } / DESIRED_FRAME_MULTIPLE;
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
                WindowEvent::Resized(PhysicalSize { width, height })
                | WindowEvent::ScaleFactorChanged {
                    scale_factor: _,
                    new_inner_size: &mut PhysicalSize { width, height },
                } => graphics.resize((width, height)),
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
                            virtual_keycode: Some(VirtualKeyCode::Escape),
                            state,
                            ..
                        },
                    ..
                } => {
                    if state == ElementState::Pressed {
                        stop_capture_mouse(&window);
                        capture_mouse = false;
                    }
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
                WindowEvent::MouseInput {
                    button: MouseButton::Left,
                    state,
                    ..
                } => {
                    if state == ElementState::Pressed {
                        capture_mouse = begin_capture_mouse(&window).is_ok();
                    }
                }
                WindowEvent::KeyboardInput { input: key, .. } => {
                    capture_mouse = begin_capture_mouse(&window).is_ok();
                    camera.key_input(key, slow_mode);
                }
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
                if capture_mouse && continue_capture_mouse(&window) {
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
                #[cfg(target_arch = "wasm32")]
                {
                    let js_window = web_sys::window().unwrap();
                    let size = (
                        js_window.inner_width().unwrap().as_f64().unwrap() as u32,
                        js_window.inner_height().unwrap().as_f64().unwrap() as u32,
                    );
                    if size != graphics.window_size() {
                        window.set_inner_size(PhysicalSize::new(size.0, size.1));
                        graphics.resize(size);
                    }
                }
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
fn continue_capture_mouse(window: &Window) -> bool {
    #[cfg(target_arch = "wasm32")]
    if web_sys::window()
        .unwrap()
        .document()
        .unwrap()
        .pointer_lock_element()
        .is_none()
    {
        return false;
    }
    let size = window.inner_size();
    let _ = window.set_cursor_position(PhysicalPosition::new(size.width / 2, size.height / 2));
    true
}
fn stop_capture_mouse(window: &Window) {
    window.set_cursor_grab(CursorGrabMode::None).unwrap();
    window.set_cursor_visible(true);
}
