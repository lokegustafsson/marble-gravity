use cgmath::{prelude::*, Vector3, Vector4};
use std::time::Duration;

pub const PHYSICS_DELTA_TIME: Duration = Duration::from_millis(1);
pub const BODIES: u32 = 100;
const GRAVITY_CONSTANT: f32 = 5.0;
const GAP: f32 = 0.0001;
const STIFFNESS: f32 = 10.0;
const DAMPING: f32 = 0.5; // In (0,1); less than 0.05 is wonky

pub struct Body {
    pos: Vector3<f32>,
    vel: Vector3<f32>,
    radius: f32,
    color: Vector4<f32>,
}
impl Body {
    pub fn initial() -> Body {
        fn pos() -> f32 {
            2.0 * (rand::random::<f32>() - 0.5)
        }
        fn c() -> f32 {
            rand::random::<f32>()
        }
        Body {
            pos: [pos(), pos(), pos()].into(),
            vel: Vector3::zero(),
            radius: 0.03 * (0.8 * rand::random::<f32>() + 0.2),
            color: [c(), c(), c(), c()].into(),
        }
    }
    pub fn pos(&self) -> Vector3<f32> {
        self.pos
    }
    pub fn radius(&self) -> f32 {
        self.radius
    }
    pub fn color(&self) -> Vector4<f32> {
        self.color
    }
    pub fn update(&self, others: &[Body]) -> Body {
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
        Body {
            pos: self.pos + self.vel * dt + accel * dt * dt / 2.0,
            vel: self.vel + accel * dt,
            radius: self.radius,
            color: self.color,
        }
    }
}
