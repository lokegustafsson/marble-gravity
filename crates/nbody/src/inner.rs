use crate::{Accel, Body};

pub use wasm_bindgen_rayon::init_thread_pool;

#[wasm_bindgen::prelude::wasm_bindgen(start)]
pub fn init_logging() {
    std::panic::set_hook(Box::new(console_error_panic_hook::hook));
    console_log::init_with_level(log::Level::Info);
    log::info!("NBODY START");
}

/// Called by javascript glue on worker thread
/// Allowed to block
#[wasm_bindgen::prelude::wasm_bindgen(js_name = "computeAccelsInner")]
pub fn compute_accels_inner(bodies_bytes: &[u32]) -> Box<[u32]> {
    use rayon::prelude::{IntoParallelRefIterator, ParallelIterator};
    let bodies: &[Body] = bytemuck::cast_slice(bodies_bytes);
    let ret_vec: Vec<Accel> = bodies.par_iter().map(|b| Accel::new(b, bodies)).collect();
    let ret: Box<[Accel]> = ret_vec.into();
    bytemuck::cast_slice_box(ret)
}
