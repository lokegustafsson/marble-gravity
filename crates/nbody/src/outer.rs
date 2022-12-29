use crate::{body::Body, Accel};
use instant::Instant;
use std::time::Duration;
use winit::event_loop::EventLoopProxy;

#[derive(Debug)]
pub struct NBodyResult {
    pub accels: Vec<Accel>,
    pub time_spent: Duration,
}

#[cfg(target_arch = "wasm32")]
use js_sys::Promise;

#[cfg(target_arch = "wasm32")]
#[wasm_bindgen::prelude::wasm_bindgen(raw_module = "./compute.js")]
extern "C" {
    /// Promise<Box<[u8]>>
    /// Called from main wasm on main thread
    /// Main thread cannot block
    #[wasm_bindgen::prelude::wasm_bindgen(js_name = "computeAccelsOuter")]
    pub fn compute_accels_outer(bodies_byte_buffer: &[u8]) -> Promise;
}

impl NBodyResult {
    pub fn spawn_compute_accels(bodies: &[Body], proxy: EventLoopProxy<NBodyResult>) {
        let before = Instant::now();
        #[cfg(target_arch = "wasm32")]
        {
            use js_sys::Uint8Array;
            use std::mem;
            use wasm_bindgen_futures::JsFuture;

            let promise = compute_accels_outer(bytemuck::cast_slice(bodies));
            let bytes_len = bodies.len() * mem::size_of::<Accel>();
            wasm_bindgen_futures::spawn_local(async move {
                let accels_bytes: Vec<u8> =
                    Uint8Array::from(JsFuture::from(promise).await.unwrap()).to_vec();
                assert_eq!(accels_bytes.len(), bytes_len);
                let accels: Vec<Accel> = bytemuck::cast_vec(accels_bytes);
                proxy
                    .send_event(NBodyResult {
                        accels,
                        time_spent: Instant::now().duration_since(before),
                    })
                    .unwrap();
            });
        }
        #[cfg(not(target_arch = "wasm32"))]
        {
            use rayon::prelude::{IntoParallelRefIterator, ParallelIterator};
            let ret: Vec<_> = bodies.par_iter().map(|b| Accel::new(b, bodies)).collect();
            proxy
                .send_event(NBodyResult {
                    accels: ret,
                    time_spent: Instant::now().duration_since(before),
                })
                .unwrap();
        };
    }
}
