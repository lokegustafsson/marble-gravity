use cgmath::{prelude::*, Vector3};

pub const BODIES: u32 = 100;
const GRAVITY_CONSTANT: f32 = 5.0;
const RESTITUTION: f32 = 0.5;

pub struct Body {
    pos: Vector3<f32>,
    vel: Vector3<f32>,
    radius: f32,
}
impl Body {
    pub fn initial() -> Body {
        fn pos() -> f32 {
            2.0 * (rand::random::<f32>() - 0.5)
        }
        Body {
            pos: [pos(), pos(), pos()].into(),
            vel: Vector3::zero(),
            radius: 0.03 * (0.8 * rand::random::<f32>() + 0.2),
        }
    }
    pub fn pos(&self) -> Vector3<f32> {
        self.pos
    }
    pub fn radius(&self) -> f32 {
        self.radius
    }
    pub fn update(&self, others: &[Body], dt: f32) -> Body {
        let mut accel = Vector3::zero();
        let mut post_collision_vel = self.vel;
        for body in others {
            let rel_pos_other = body.pos - self.pos;
            if body.pos == self.pos {
                // Same body
                continue;
            } else if rel_pos_other.magnitude2() < (self.radius + body.radius).powi(2) {
                // Almost-elastic collision
                let rel_pos_norm = rel_pos_other.normalize();
                let vel_towards_other = self.vel.dot(rel_pos_norm);
                let vel_towards_self = body.vel.dot(-rel_pos_norm);
                if vel_towards_other + vel_towards_self < 0.0 {
                    // Already separating from each other
                    continue;
                }
                let mass_self = self.radius.powi(3);
                let mass_other = body.radius.powi(3);
                let new_vel_from_other =
                    (RESTITUTION * mass_other * (vel_towards_self - vel_towards_other)
                        + mass_self * vel_towards_other
                        + mass_other * vel_towards_self)
                        / (mass_self + mass_other);
                post_collision_vel -= (new_vel_from_other + vel_towards_other) * rel_pos_norm;
            } else {
                // Gravitational interaction
                let distance_cubed = rel_pos_other.magnitude().powi(3);
                accel += GRAVITY_CONSTANT * body.radius.powi(3) / distance_cubed * rel_pos_other;
            }
        }
        Body {
            pos: self.pos + (self.vel + post_collision_vel) / 2.0 * dt + accel * dt * dt / 2.0,
            vel: post_collision_vel + accel * dt,
            radius: self.radius,
        }
    }
}
