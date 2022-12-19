use cgmath::{prelude::*, Vector3};
use rand_distr::Distribution;
use std::time::Duration;

pub const PHYSICS_DELTA_TIME: Duration = Duration::from_millis(1);
pub const BODIES: u32 = 256;
const GRAVITY_CONSTANT: f32 = 80.0;
const GAP: f32 = 0.001;
const STIFFNESS: f32 = 1.0;
const DAMPING: f32 = 0.2; // In (0,1); less than 0.05 is wonky
const SYSTEM_RADIUS: f32 = 10.0;

#[derive(Copy, Clone)]
pub struct Body {
    pos: Vector3<f32>,
    vel: Vector3<f32>,
    radius: f32,
    color: u32,
}
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
    pub fn perform_step(bodies: &mut [Body]) {
        let mut vels_accels: Vec<[Vector3<f32>; 2]> = bodies
            .iter()
            .map(|body| body.compute_vel_accel(&bodies))
            .collect();
        let total_mass: f32 = bodies.iter().map(|b| b.radius.powi(3)).sum();
        let total_momentum: Vector3<f32> = bodies
            .iter()
            .zip(&vels_accels)
            .map(|(b, [v, _])| b.radius.powi(3) * v)
            .sum();
        vels_accels
            .iter_mut()
            .for_each(|[v, _]| *v -= total_momentum / total_mass);
        bodies
            .iter_mut()
            .zip(vels_accels)
            .for_each(|(b, va)| b.step_using_vel_accel(va));
    }
    pub fn pos(&self) -> Vector3<f32> {
        self.pos
    }
    pub fn radius(&self) -> f32 {
        self.radius
    }
    pub fn color(&self) -> u32 {
        self.color
    }
    fn compute_vel_accel(&self, others: &[Body]) -> [Vector3<f32>; 2] {
        let dt = PHYSICS_DELTA_TIME.as_secs_f32();
        let mut accel = Vector3::zero();
        for body in others {
            if body.pos == self.pos {
                continue; // Same body
            }
            let rel_pos = body.pos - self.pos;
            let distance = rel_pos.magnitude();
            let rel_pos_norm = rel_pos / distance;
            let rel_vel = (body.vel - self.vel).dot(rel_pos_norm);

            let overlap =
                self.radius + GAP + body.radius - distance - rel_vel * dt * (1.0 + DAMPING) / 2.0;
            if overlap > 0.0 {
                // Spring-based collision
                let force_towards_other = -STIFFNESS * overlap;
                accel += force_towards_other / self.radius.powi(3) * rel_pos_norm;
            }
            // Gravitational interaction
            accel += GRAVITY_CONSTANT * body.radius.powi(3) / distance.powi(2) * rel_pos_norm;
        }
        let mut vel = self.vel;
        if self.pos.magnitude2() > SYSTEM_RADIUS.powi(2) && vel.dot(self.pos) > 0.0 {
            vel *= -0.99;
        }
        [vel, accel]
    }
    fn step_using_vel_accel(&mut self, [vel, accel]: [Vector3<f32>; 2]) {
        let dt = PHYSICS_DELTA_TIME.as_secs_f32();
        self.pos = self.pos + vel * dt + accel * dt * dt / 2.0;
        self.vel = vel + accel * dt;
    }
}
