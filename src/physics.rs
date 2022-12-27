use crate::nbody::Accel;
use cgmath::{prelude::*, Vector3};
use rand_distr::Distribution;
use std::time::Duration;

pub const PHYSICS_DELTA_TIME: Duration = Duration::from_millis(1);
pub const BODIES: u32 = 256;
const SYSTEM_RADIUS: f32 = 10.0;

#[derive(Copy, Clone)]
pub struct Body {
    pub pos: Vector3<f32>,
    pub vel: Vector3<f32>,
    pub radius: f32,
    pub color: u32,
}
unsafe impl bytemuck::Zeroable for Body {}
unsafe impl bytemuck::Pod for Body {}
impl Body {
    pub fn initial() -> Body {
        let mut normal = rand_distr::Normal::new(0.0f32, 1.0)
            .unwrap()
            .sample_iter(rand::thread_rng());
        let mut r = move || normal.next().unwrap();
        let pos = [r(), r(), r()].into();
        let rand = [r(), r(), r()].into();
        Body {
            pos,
            vel: 0.1 * pos.cross(rand),
            radius: 0.03 * (0.8 * rand::random::<f32>() + 0.2),
            color: rand::random(),
        }
    }
    pub fn perform_step(bodies: &mut [Body], accels: Vec<Accel>) {
        let mut vels: Vec<_> = bodies.iter().map(Body::new_vel).collect();
        let total_mass: f32 = bodies.iter().map(|b| b.radius.powi(3)).sum();
        let total_momentum: Vector3<f32> = bodies
            .iter()
            .zip(&vels)
            .map(|(b, v)| b.radius.powi(3) * v)
            .sum();
        vels.iter_mut()
            .for_each(|v| *v -= total_momentum / total_mass);
        bodies
            .iter_mut()
            .zip(vels)
            .zip(accels)
            .for_each(|((b, v), Accel(a))| b.step_using_vel_accel([v, a]));
    }
    fn new_vel(&self) -> Vector3<f32> {
        if self.pos.magnitude2() > SYSTEM_RADIUS.powi(2) && self.vel.dot(self.pos) > 0.0 {
            self.vel * -0.99
        } else {
            self.vel
        }
    }
    fn step_using_vel_accel(&mut self, [vel, accel]: [Vector3<f32>; 2]) {
        let dt = PHYSICS_DELTA_TIME.as_secs_f32();
        self.pos = self.pos + vel * dt + accel * dt * dt / 2.0;
        self.vel = vel + accel * dt;
    }
}
