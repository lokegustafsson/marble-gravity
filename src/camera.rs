use cgmath::{prelude::*, Matrix4, Quaternion, Rad, Vector3};
use winit::event::{ElementState, KeyboardInput, VirtualKeyCode};

const SPEED: f32 = 2.0;
const ROLL_RATE: f32 = 2.0;
const SENSITIVITY: f32 = 0.001;

pub struct Camera {
    position: Vector3<f32>,
    rotation: Quaternion<f32>,
    forwards: bool,
    backwards: bool,
    right: bool,
    left: bool,
    down: bool,
    up: bool,
    roll_right: bool,
    roll_left: bool,
    pitch_up: f32,
    yaw_right: f32,
}

impl Camera {
    pub fn new() -> Self {
        Self {
            position: Vector3::new(0.0, 0.0, -2.0),
            rotation: Quaternion::one(),
            forwards: false,
            backwards: false,
            right: false,
            left: false,
            down: false,
            up: false,
            roll_right: false,
            roll_left: false,
            pitch_up: 0.0,
            yaw_right: 0.0,
        }
    }
    pub fn update(&mut self, dt: f32) {
        let mut velocity = Vector3::zero();
        if self.forwards {
            velocity += Vector3::unit_z();
        }
        if self.backwards {
            velocity -= Vector3::unit_z();
        }
        if self.right {
            velocity += Vector3::unit_x();
        }
        if self.left {
            velocity -= Vector3::unit_x();
        }
        if self.down {
            velocity += Vector3::unit_y();
        }
        if self.up {
            velocity -= Vector3::unit_y();
        }
        let roll_factor =
            if self.roll_right { 1.0 } else { 0.0 } + if self.roll_left { -1.0 } else { 0.0 };

        self.position += self.rotation.rotate_vector(velocity * SPEED * dt);
        self.rotation = self.rotation
            * Quaternion::from_axis_angle(Vector3::unit_z(), Rad(ROLL_RATE * roll_factor * dt))
            * Quaternion::from_axis_angle(Vector3::unit_x(), Rad(self.pitch_up))
            * Quaternion::from_axis_angle(Vector3::unit_y(), Rad(self.yaw_right));
        self.pitch_up = 0.0;
        self.yaw_right = 0.0;
    }
    pub fn key_input(&mut self, key: KeyboardInput) {
        use VirtualKeyCode::*;
        if key.virtual_keycode.is_none() {
            return;
        }
        let active = key.state == ElementState::Pressed;
        match key.virtual_keycode.unwrap() {
            W => self.forwards = active,
            S => self.backwards = active,
            D => self.right = active,
            A => self.left = active,
            LShift => self.down = active,
            Space => self.up = active,
            E => self.roll_right = active,
            Q => self.roll_left = active,
            _ => {}
        }
    }
    pub fn mouse_input(&mut self, x: f64, y: f64, w: u32, h: u32) {
        let (mx, my) = (w as f32 / 2.0, h as f32 / 2.0);
        self.pitch_up -= SENSITIVITY * (y as f32 - my);
        self.yaw_right += SENSITIVITY * (x as f32 - mx);
    }
    pub fn world_to_camera(&mut self) -> Matrix4<f32> {
        let trans = Matrix4::from_translation(-self.position);
        let rot = Matrix4::from(self.rotation.conjugate());
        rot * trans
    }
}
