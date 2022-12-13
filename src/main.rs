mod camera;
mod graphics;
mod physics;
mod run;
mod spheretree;

use crate::{graphics::Graphics, physics::Body};
use std::time::Duration;
use winit::{event_loop::EventLoop, window::WindowBuilder};

const MAX_BEHIND: Duration = Duration::from_secs(1);

pub fn main() {
    #[cfg(not(target_arch = "wasm32"))]
    {
        env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("info")).init();
        tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .unwrap()
            .block_on(setup_and_run())
    }
    #[cfg(target_arch = "wasm32")]
    {
        std::panic::set_hook(Box::new(console_error_panic_hook::hook));
        console_log::init_with_level(log::Level::Trace).expect("Couldn't initialize logger");
        wasm_bindgen_futures::spawn_local(setup_and_run());
    }
}

pub async fn setup_and_run() {
    let instance = wgpu::Instance::new(wgpu::Backends::all());
    let event_loop = EventLoop::new();
    let window = WindowBuilder::new()
        .with_title("Marble Gravity")
        .with_maximized(true)
        .build(&event_loop)
        .unwrap();

    #[cfg(target_arch = "wasm32")]
    {
        use winit::{dpi::PhysicalSize, platform::web::WindowExtWebSys};
        window.set_inner_size(PhysicalSize::new(450, 400));

        web_sys::window()
            .and_then(|win| win.document())
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

    let graphics = Graphics::initialize(surface, device_and_queue, size).await;
    log::info!("Starting event loop");
    run::run(event_loop, window, graphics).await;
}

async fn get_adapter(instance: &wgpu::Instance, surface: &wgpu::Surface) -> wgpu::Adapter {
    #[cfg(not(target_arch = "wasm32"))]
    {
        log::info!("Available adapters:");
        instance
            .enumerate_adapters(wgpu::Backends::all())
            .for_each(|adapter| log::info!("\t{:?}", adapter.get_info()));
    }
    let ret = instance
        .request_adapter(&wgpu::RequestAdapterOptionsBase {
            power_preference: wgpu::PowerPreference::HighPerformance,
            compatible_surface: Some(&surface),
            force_fallback_adapter: false,
        })
        .await
        .expect("Failed to acquire adapter");
    ret
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
