#![cfg(target_arch = "wasm32")]

#[cfg(not(target_arch = "wasm32"))]
compile_error!("This crate's only purpose is supporting multithreading through web workers");

use instant::Instant;
use physics::{Physics, PhysicsResult};

#[cfg(feature = "inner")]
pub mod inner {
    use super::*;

    pub use wasm_bindgen_rayon::init_thread_pool;

    #[wasm_bindgen::prelude::wasm_bindgen(start)]
    pub fn init_logging() {
        std::panic::set_hook(Box::new(console_error_panic_hook::hook));
        let (Ok(_) | Err(_)) = console_log::init_with_level(log::Level::Info);
    }

    /// Called by javascript glue on worker thread
    /// Allowed to block
    #[wasm_bindgen::prelude::wasm_bindgen(js_name = "workerInner")]
    pub fn worker_inner(input: &[u64]) -> Box<[u64]> {
        let WorkerInput {
            physics,
            target_instant,
        } = if let &[data] = bytemuck::cast_slice(input) {
            data
        } else {
            unreachable!();
        };
        let mut physics = physics.clone();
        let result = physics.advance_to(target_instant);
        let output: Vec<WorkerOutput> = vec![WorkerOutput { physics, result }];
        bytemuck::cast_vec(output).into_boxed_slice()
    }
}

#[cfg(feature = "outer")]
pub mod outer {
    use super::*;
    use winit::event_loop::EventLoopProxy;

    pub struct Worker;
    impl Worker {
        pub fn advance_physics_to(
            physics: &Physics,
            target: Instant,
            proxy: EventLoopProxy<(Box<Physics>, PhysicsResult)>,
        ) -> Result<(), ()> {
            use js_sys::BigUint64Array;
            use wasm_bindgen_futures::JsFuture;

            if !poll_ready() {
                return Err(());
            }

            let input: WorkerInput = WorkerInput {
                physics: physics.clone(),
                target_instant: target,
            };
            let promise = worker_outer(bytemuck::cast_slice(&[input]));
            wasm_bindgen_futures::spawn_local(async move {
                let output_data: Vec<u64> =
                    BigUint64Array::from(JsFuture::from(promise).await.unwrap()).to_vec();
                if let &[WorkerOutput { physics, result }] = bytemuck::cast_slice(&*output_data) {
                    proxy.send_event((Box::new(physics), result)).unwrap();
                } else {
                    unreachable!();
                }
            });
            Ok(())
        }
    }

    #[wasm_bindgen::prelude::wasm_bindgen(raw_module = "./compute.js")]
    extern "C" {
        /// Promise<Box<[u64]>>
        /// Called from main wasm on main thread
        /// Main thread cannot block
        #[wasm_bindgen::prelude::wasm_bindgen(js_name = "workerOuter")]
        pub fn worker_outer(input: &[u64]) -> js_sys::Promise;

        #[wasm_bindgen::prelude::wasm_bindgen(js_name = "pollReady")]
        pub fn poll_ready() -> bool;
    }
}

#[derive(Clone, Copy)]
struct WorkerInput {
    #[allow(dead_code)]
    physics: Physics,
    #[allow(dead_code)]
    target_instant: Instant,
}
unsafe impl bytemuck::Zeroable for WorkerInput {}
unsafe impl bytemuck::Pod for WorkerInput {}

#[derive(Clone, Copy)]
struct WorkerOutput {
    #[allow(dead_code)]
    physics: Physics,
    #[allow(dead_code)]
    result: PhysicsResult,
}
unsafe impl bytemuck::Zeroable for WorkerOutput {}
unsafe impl bytemuck::Pod for WorkerOutput {}
