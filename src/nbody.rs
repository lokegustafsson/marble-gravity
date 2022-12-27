use crate::physics::{Body, PHYSICS_DELTA_TIME};
use cgmath::{prelude::*, Vector3};
use instant::Instant;
use std::time::Duration;
use winit::event_loop::EventLoopProxy;

const GRAVITY_CONSTANT: f32 = 80.0;
const GAP: f32 = 0.001;
const STIFFNESS: f32 = 1.0;
const DAMPING: f32 = 0.2; // In (0,1); less than 0.05 is wonky

#[derive(Copy, Clone, Debug)]
#[repr(transparent)]
pub struct Accel(pub Vector3<f32>);
unsafe impl bytemuck::Zeroable for Accel {}
unsafe impl bytemuck::Pod for Accel {}

#[derive(Debug)]
pub struct NBodyResult {
    pub accels: Vec<Accel>,
    pub time_spent: Duration,
}

#[cfg(target_arch = "wasm32")]
mod bindings {
    use super::*;
    use js_sys::Promise;
    pub use wasm_bindgen_rayon::init_thread_pool;

    #[wasm_bindgen::prelude::wasm_bindgen]
    extern "C" {
        /// Promise<Box<[u8]>>
        /// Called from main wasm on main thread
        /// Main thread cannot block
        pub fn compute_accels_outer(bodies_byte_buffer: &[u8]) -> Promise;
    }
    /// Called by javascript glue on worker thread
    /// Allowed to block
    #[wasm_bindgen::prelude::wasm_bindgen]
    pub fn compute_accels_inner(bodies_bytes: &[u8]) -> Box<[u8]> {
        use rayon::prelude::{IntoParallelRefIterator, ParallelIterator};
        let bodies: &[Body] = bytemuck::cast_slice(bodies_bytes);
        let ret_vec: Vec<Accel> = bodies.par_iter().map(|b| Accel::new(b, bodies)).collect();
        let ret: Box<[Accel]> = ret_vec.into();
        bytemuck::cast_slice_box(ret)
    }
}

impl NBodyResult {
    pub fn spawn_compute_accels(bodies: &[Body], proxy: EventLoopProxy<NBodyResult>) {
        let before = Instant::now();
        #[cfg(target_arch = "wasm32")]
        {
            use js_sys::Uint8Array;
            use std::mem;
            use wasm_bindgen_futures::JsFuture;

            let promise = bindings::compute_accels_outer(bytemuck::cast_slice(bodies));
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

impl Accel {
    fn new(body: &Body, others: &[Body]) -> Self {
        let dt = PHYSICS_DELTA_TIME.as_secs_f32();
        let mut accel = Vector3::zero();
        for other in others {
            if other.pos == body.pos {
                continue; // Same body
            }
            let rel_pos = other.pos - body.pos;
            let distance = rel_pos.magnitude();
            let rel_pos_norm = rel_pos / distance;
            let rel_vel = (other.vel - body.vel).dot(rel_pos_norm);

            let overlap =
                body.radius + GAP + other.radius - distance - rel_vel * dt * (1.0 + DAMPING) / 2.0;
            if overlap > 0.0 {
                // Spring-based collision
                let force_towards_other = -STIFFNESS * overlap;
                accel += force_towards_other / body.radius.powi(3) * rel_pos_norm;
            }
            // Gravitational interaction
            accel += GRAVITY_CONSTANT * other.radius.powi(3) / distance.powi(2) * rel_pos_norm;
        }
        Self(accel)
    }
}
