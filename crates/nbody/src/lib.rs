mod body;
pub use crate::body::{Body, BODIES, PHYSICS_DELTA_TIME};

#[cfg(feature = "inner")]
mod inner;
#[cfg(feature = "inner")]
pub use inner::*;

#[cfg(feature = "outer")]
mod outer;
#[cfg(feature = "outer")]
pub use outer::*;

#[derive(Copy, Clone, Debug)]
#[repr(transparent)]
pub struct Accel(pub cgmath::Vector3<f32>);
unsafe impl bytemuck::Zeroable for Accel {}
unsafe impl bytemuck::Pod for Accel {}

#[cfg(any(feature = "inner", not(target_arch = "wasm32")))]
impl Accel {
    fn new(body: &Body, others: &[Body]) -> Self {
        use cgmath::{prelude::*, Vector3};

        const GRAVITY_CONSTANT: f32 = 80.0;
        const GAP: f32 = 0.001;
        const STIFFNESS: f32 = 1.0;
        const DAMPING: f32 = 0.2; // In (0,1); less than 0.05 is wonky

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
