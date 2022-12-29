use crate::{camera::Camera, graphics::Graphics, spheretree, PHYSICS_MAX_BEHIND_TIME};
use instant::Instant;
use nbody::{Body, NBodyResult, BODIES, PHYSICS_DELTA_TIME};
use std::time::Duration;
use winit::{
    dpi::{PhysicalPosition, PhysicalSize},
    event::{
        DeviceEvent, ElementState, Event, KeyboardInput, MouseButton, VirtualKeyCode, WindowEvent,
    },
    event_loop::{ControlFlow, EventLoop},
    window::{CursorGrabMode, Window},
};

struct Stats {
    frame_number: u64,
    tick_number: u64,
    instant_start: Instant,
    time_spent_in_physics: Duration,
    time_spent_in_graphics: Duration,
}

pub fn run(event_loop: EventLoop<NBodyResult>, window: Window, mut graphics: Graphics) {
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

    let mut stats = Stats {
        frame_number: 0,
        tick_number: 0,
        instant_start: Instant::now(),
        time_spent_in_physics: Duration::ZERO,
        time_spent_in_graphics: Duration::ZERO,
    };

    let proxy = event_loop.create_proxy();
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
                            virtual_keycode: Some(vk @ (VirtualKeyCode::Up | VirtualKeyCode::Down)),
                            state: ElementState::Pressed,
                            ..
                        },
                    ..
                } => graphics.change_ray_splits(match vk {
                    VirtualKeyCode::Up => 1,
                    VirtualKeyCode::Down => -1,
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
                    let new_physics_timestamp = now.checked_sub(PHYSICS_DELTA_TIME).unwrap();
                    log::error!(
                        "Physics computation far behind, dropping {}ms",
                        (new_physics_timestamp - physics_timestamp).as_millis()
                    );
                    physics_timestamp = new_physics_timestamp;
                }
                NBodyResult::spawn_compute_accels(&bodies, proxy.clone());
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
                let instant_pre_graphics = Instant::now();
                graphics.render(
                    spheretree::make_sphere_tree(&bodies, camera.world_to_camera()),
                    camera.rotation(),
                );
                stats.time_spent_in_graphics += Instant::now().duration_since(instant_pre_graphics);
                stats.frame_number += 1;
                if stats.frame_number.is_power_of_two() || stats.frame_number % 1024 == 0 {
                    log::info!(
                        "Elapsed {}s total, {}s physics ({} ticks), {}s graphics ({} frames)",
                        Instant::now().duration_since(stats.instant_start).as_secs(),
                        stats.time_spent_in_physics.as_secs(),
                        stats.tick_number,
                        stats.time_spent_in_graphics.as_secs(),
                        stats.frame_number,
                    );
                }
                control_flow
                    .set_wait_until(last_frame_processing_begun_instant + desired_frame_time);
            }
            Event::UserEvent(NBodyResult { accels, time_spent }) => {
                stats.time_spent_in_physics += time_spent;
                stats.tick_number += 1;
                Body::perform_step(&mut bodies, accels);
                physics_timestamp += PHYSICS_DELTA_TIME;

                if Instant::now().checked_duration_since(physics_timestamp)
                    > Some(PHYSICS_DELTA_TIME)
                {
                    NBodyResult::spawn_compute_accels(&bodies, proxy.clone());
                }
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
