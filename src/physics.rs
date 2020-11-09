use cgmath::{prelude::*, Vector3};

pub const BODIES: u32 = 100;
const GRAVITY_CONSTANT: f32 = 1.0;

pub struct Body {
    pos: Vector3<f32>,
    vel: Vector3<f32>,
    radius: f32,
}
impl Body {
    pub fn initial() -> Body {
        fn pos() -> f32 {
            1.0 * (rand::random::<f32>() - 0.5)
        }
        Body {
            pos: [pos(), pos(), pos()].into(),
            vel: Vector3::zero(),
            radius: 0.03 * rand::random::<f32>(),
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
        for body in others {
            if self.pos.distance(body.pos) < self.radius + body.radius {
                // Intersecting
                continue;
            } else {
                // Not intersecting
                let volume = 4.0 / 3.0 * std::f32::consts::PI * body.radius.powi(3);
                let position = body.pos - self.pos;
                accel += GRAVITY_CONSTANT * volume / position.magnitude().powi(3) * position;
            }
        }
        Body {
            pos: self.pos + self.vel * dt + accel * dt * dt / 2.0,
            vel: self.vel + accel * dt,
            radius: self.radius,
        }
    }
}
