use crate::{Accel, Body};

pub use wasm_bindgen_rayon::init_thread_pool;

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
