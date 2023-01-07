use crate::PHYSICS_DELTA_TIME;
use cgmath::{prelude::*, Vector3};
use rand_distr::Distribution;

const SYSTEM_RADIUS: f32 = 5.0;
const GRAVITY_CONSTANT: f32 = 40.0;
const GAP: f32 = 0.001;
const STIFFNESS: f32 = 1.0;
const DAMPING: f32 = 0.2; // In (0,1); less than 0.05 is wonky

#[derive(Debug, Copy, Clone)]
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
            radius: 0.03 * (0.8 * r().abs() + 0.2),
            color: rand::random(),
        }
    }
    pub fn perform_step(bodies: &mut [Body], accels: Vec<Vector3<f32>>) {
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
            .for_each(|((b, v), a)| b.step_using_vel_accel([v, a]));
    }
    pub fn accel_from(&self, bodies: &[Body]) -> Vector3<f32> {
        let dt = PHYSICS_DELTA_TIME.as_secs_f32();
        let mut accel = Vector3::zero();
        for other in bodies {
            if other.pos == self.pos {
                continue; // Same body
            }
            let rel_pos = other.pos - self.pos;
            let distance = rel_pos.magnitude();
            let rel_pos_norm = rel_pos / distance;
            let rel_vel = (other.vel - self.vel).dot(rel_pos_norm);

            let overlap =
                self.radius + GAP + other.radius - distance - rel_vel * dt * (1.0 + DAMPING) / 2.0;
            if overlap > 0.0 {
                // Spring-based collision
                let force_towards_other = -STIFFNESS * overlap;
                accel += force_towards_other / self.radius.powi(3) * rel_pos_norm;
            }
            // Gravitational interaction
            accel += GRAVITY_CONSTANT * other.radius.powi(3) / distance.powi(2) * rel_pos_norm;
        }
        accel
    }
    fn new_vel(&self) -> Vector3<f32> {
        if self.pos.magnitude2() > SYSTEM_RADIUS.powi(2) && self.vel.dot(self.pos) > 0.0 {
            self.vel * 0.99
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
