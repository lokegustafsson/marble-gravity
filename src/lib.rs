mod camera;
mod graphics;
mod nbody;
mod physics;
mod run;
mod spheretree;

use crate::{
    graphics::{Graphics, Parameters},
    physics::Body,
};
use std::time::Duration;
use winit::{event_loop::EventLoopBuilder, window::WindowBuilder};

const PHYSICS_MAX_BEHIND_TIME: Duration = Duration::from_secs(1);

#[cfg_attr(target_arch = "wasm32", wasm_bindgen::prelude::wasm_bindgen)]
pub fn start() {
    #[cfg(not(target_arch = "wasm32"))]
    {
        env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("info")).init();
        pollster::block_on(setup_and_run());
    }
    #[cfg(target_arch = "wasm32")]
    {
        std::panic::set_hook(Box::new(console_error_panic_hook::hook));
        console_log::init_with_level(log::Level::Info).unwrap();
        wasm_bindgen_futures::spawn_local(setup_and_run());
    }
}

async fn setup_and_run() {
    log::info!("Setting up");
    let instance = wgpu::Instance::new(wgpu::Backends::all());
    let event_loop = EventLoopBuilder::with_user_event().build();
    let window = WindowBuilder::new()
        .with_title("Marble Gravity")
        .with_maximized(true)
        .build(&event_loop)
        .unwrap();

    #[cfg(target_arch = "wasm32")]
    {
        use winit::{dpi::PhysicalSize, platform::web::WindowExtWebSys};
        let js_window = web_sys::window().unwrap();
        window.set_inner_size(PhysicalSize::new(
            js_window.inner_width().unwrap().as_f64().unwrap() as u32,
            js_window.inner_height().unwrap().as_f64().unwrap() as u32,
        ));

        js_window
            .document()
            .and_then(|doc| {
                let dst = doc.get_element_by_id("wasm")?;
                let canvas = web_sys::Element::from(window.canvas());
                dst.append_child(&canvas).ok()?;
                Some(())
            })
            .expect("Couldn't append canvas to document body.");
    }

    let surface = unsafe { instance.create_surface(&window) };
    let adapter = get_adapter(&instance, &surface).await;
    let size: (u32, u32) = window.inner_size().into();

    let device_and_queue = get_device_and_queue(&adapter).await;
    let parameters = Parameters {
        texture_format: *surface.get_supported_formats(&adapter).first().unwrap(),
        present_mode: {
            let supported = surface.get_supported_present_modes(&adapter);
            if supported.contains(&wgpu::PresentMode::Mailbox) {
                wgpu::PresentMode::Mailbox
            } else {
                *supported.first().unwrap()
            }
        },
    };

    let graphics = Graphics::initialize(parameters, surface, device_and_queue, size).await;

    log::info!("Starting event loop");
    run::run(event_loop, window, graphics);
}

async fn get_adapter(instance: &wgpu::Instance, surface: &wgpu::Surface) -> wgpu::Adapter {
    #[cfg(not(target_arch = "wasm32"))]
    {
        log::info!("Available adapters:");
        instance
            .enumerate_adapters(wgpu::Backends::all())
            .for_each(|adapter| log::info!("\t{:?}", adapter.get_info()));
    }
    instance
        .request_adapter(&wgpu::RequestAdapterOptionsBase {
            power_preference: wgpu::PowerPreference::HighPerformance,
            compatible_surface: Some(surface),
            force_fallback_adapter: false,
        })
        .await
        .expect("Failed to acquire adapter")
}

async fn get_device_and_queue(adapter: &wgpu::Adapter) -> (wgpu::Device, wgpu::Queue) {
    adapter
        .request_device(
            &wgpu::DeviceDescriptor {
                label: Some("device"),
                features: wgpu::Features::empty(),
                limits: if cfg!(target_arch = "wasm32") {
                    wgpu::Limits::downlevel_webgl2_defaults()
                } else {
                    wgpu::Limits::default()
                },
            },
            None, // Trace path
        )
        .await
        .expect("Failed to acquire device")
}
