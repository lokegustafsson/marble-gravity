use instant::Instant;
use std::time::Duration;

pub const PHYSICS_DELTA_TIME: Duration = Duration::from_millis(1);
pub const PHYSICS_MAX_BEHIND_TIME: Duration = Duration::from_secs(1);
pub const BODIES: usize = 256;

mod body;
pub use body::Body;

#[derive(Clone, Copy, Debug)]
pub struct Physics {
    bodies: [Body; BODIES],
    #[allow(unused)]
    timestamp: Instant,
}
unsafe impl bytemuck::Zeroable for Physics {}
unsafe impl bytemuck::Pod for Physics {}

#[derive(Clone, Copy, Debug)]
pub struct PhysicsResult {
    pub elapsed_real: Duration,
    pub elapsed_physics_ticks: u64,
}

impl Physics {
    pub fn initial() -> Box<Self> {
        Box::new(Self {
            bodies: (0..BODIES)
                .into_iter()
                .map(|_| Body::initial())
                .collect::<Vec<_>>()
                .try_into()
                .unwrap(),
            timestamp: Instant::now(),
        })
    }
    pub fn bodies(&self) -> &[Body; BODIES] {
        &self.bodies
    }
    #[cfg(any(feature = "rayon", not(target_arch = "wasm32")))]
    pub fn advance_to(&mut self, target: Instant) -> PhysicsResult {
        use cgmath::Vector3;
        use rayon::prelude::{IntoParallelRefIterator, ParallelIterator};

        let before = Instant::now();
        let mut elapsed_physics_ticks = 0;
        loop {
            let lag = target.checked_duration_since(self.timestamp);
            match lag {
                lag if lag < Some(PHYSICS_DELTA_TIME) => break,
                lag if lag > Some(PHYSICS_MAX_BEHIND_TIME) => {
                    let new_timestamp = target - PHYSICS_DELTA_TIME;
                    log::error!(
                        "Physics computation far behind, dropping {}ms",
                        (new_timestamp - self.timestamp).as_millis()
                    );
                    self.timestamp = new_timestamp;
                }
                _ => {}
            }
            let accels: Vec<Vector3<f32>> = self
                .bodies
                .par_iter()
                .map(|b| b.accel_from(&self.bodies))
                .collect();
            Body::perform_step(&mut self.bodies, accels);
            self.timestamp += PHYSICS_DELTA_TIME;
            elapsed_physics_ticks += 1;
        }
        PhysicsResult {
            elapsed_real: Instant::now() - before,
            elapsed_physics_ticks,
        }
    }
}
