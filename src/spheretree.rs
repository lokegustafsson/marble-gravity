use crate::Body;
use cgmath::{prelude::*, Matrix4, Vector3};
use std::iter::repeat;

pub fn make_sphere_tree(bodies: &[Body], world_to_camera: Matrix4<f32>) -> Vec<Sphere> {
    let mut spheres: Vec<Option<Sphere>> = bodies
        .iter()
        .map(|body| Sphere::leaf(body, &world_to_camera))
        .map(Option::from)
        .collect();

    let tot_nodes = 2 * spheres.len() - 1;
    spheres.reserve_exact(spheres.len() - 1);
    let mut num_spheres = spheres.len();
    let mut tree: Vec<Sphere> = repeat(Sphere::placeholder()).take(tot_nodes).collect();
    let mut chain: Vec<usize> = Vec::new();
    while num_spheres > 1 {
        let current = loop {
            if chain.is_empty() {
                // Put arbitrary sphere on empty stack
                chain.push(
                    spheres
                        .iter()
                        .enumerate()
                        .rev()
                        .find(|(_, s)| Option::is_some(*s))
                        .unwrap()
                        .0,
                );
            }
            let current = chain[chain.len() - 1];
            if spheres[current].is_some() {
                break current;
            }
            chain.pop();
        };
        // Find closest neighbor
        let (_cost, nearest_neighbor) = spheres
            .iter()
            .enumerate()
            .filter(|(i, neighbor)| *i != current && neighbor.is_some())
            .map(|(i, neighbor)| (measure(&spheres[current].unwrap(), &neighbor.unwrap()), i))
            .min_by(|a, b| a.partial_cmp(b).unwrap())
            .unwrap();

        if chain.len() >= 2 && nearest_neighbor == chain[chain.len() - 2] {
            // Join a pair of mutually closest neighbors
            let last = chain[chain.len() - 2];
            spheres.push(Some(Sphere::branch(current, last, &spheres)));
            tree[current] = spheres[current].take().unwrap();
            tree[last] = spheres[last].take().unwrap();
            num_spheres -= 1;
            chain.pop();
            chain.pop();
        } else {
            // Found closer pair, pushing to stack
            chain.push(nearest_neighbor);
        }
    }
    tree[tot_nodes - 1] = spheres.last().unwrap().unwrap(); // Push root
    tree
}

// This is not strictly a measure, but it works as a cost in a nearest-neighbor chain algorithm
fn measure(a: &Sphere, b: &Sphere) -> f32 {
    let joined_radius = ((a.pos - b.pos).magnitude() + a.radius + b.radius) / 2.0;
    joined_radius.powi(3) - a.radius.powi(3) - b.radius.powi(3)
}

#[repr(C)]
#[derive(Copy, Clone, Debug)]
pub struct Sphere {
    pos: Vector3<f32>,
    radius: f32,
    left: i32,
    right: i32,
    color: u32,
    _padding: u32, // Bump to 32 bytes to satisfy multiple of 16 bytes criteria
}
impl Sphere {
    pub(self) fn leaf(body: &Body, world_to_camera: &Matrix4<f32>) -> Self {
        let hom_pos = world_to_camera * body.pos.extend(1.0);
        let w = hom_pos.w;
        Self {
            pos: hom_pos.truncate() / w,
            radius: body.radius,
            left: -1,
            right: -1,
            color: body.color,
            _padding: 0,
        }
    }
    pub(self) fn branch(a_index: usize, b_index: usize, spheres: &[Option<Sphere>]) -> Self {
        let a = spheres[a_index].unwrap();
        let b = spheres[b_index].unwrap();
        let rel_pos_norm = (b.pos - a.pos).normalize();
        let distance = (b.pos - a.pos).magnitude();
        let joined_midpoint =
            ((a.pos - rel_pos_norm * a.radius) + (b.pos + rel_pos_norm * b.radius)) / 2.0;
        let joined_radius = (distance + a.radius + b.radius) / 2.0;
        Self {
            pos: joined_midpoint,
            radius: joined_radius,
            left: a_index as i32,
            right: b_index as i32,
            color: 0,
            _padding: 0,
        }
    }
    pub(self) fn placeholder() -> Self {
        Self {
            pos: Vector3::zero(),
            radius: 0.0,
            left: 0,
            right: 0,
            color: 0,
            _padding: 0,
        }
    }
}
unsafe impl bytemuck::Pod for Sphere {}
unsafe impl bytemuck::Zeroable for Sphere {}
