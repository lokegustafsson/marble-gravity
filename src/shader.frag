#version 450

// Note that the rust side includes 64 bit padding at the end which is implicit here
// Buffer items need their size to be a multiple of 128 bits. This struct is 256 bits.
struct body {
    vec3 pos;
    float radius;
    int left;
    int right;
};
struct hit_report {
    vec3 pos;
    int hit_idx;
};
const int STACK_SIZE = 20;
const int STACK_OVERFLOW = -2;
const int NO_HIT = -1;
const vec4 BLACK = vec4(0,0,0,1);
const vec4 RED = vec4(1,0,0,1);
const vec4 WHITE = vec4(1);

// IO
in vec4 gl_FragCoord;
layout(location=0) out vec4 f_color;

// Buffers & Uniforms
layout(set=0, binding=0) readonly buffer Bodies {
    body bodies[];
};
layout(set=0, binding=1) uniform Uniforms {
    vec2 window_size;
};

// Forward function declarations
float hit_time(vec3, vec3, vec3, float);
hit_report cast_ray(vec3, vec3);

void main() {
    vec2 frag_pos = gl_FragCoord.xy / window_size.y;
    vec2 mid_frag_pos = vec2(0.5 * window_size.x / window_size.y, 0.5);
    vec3 camera_ray = vec3(frag_pos - mid_frag_pos, 1);

    hit_report res = cast_ray(vec3(0), camera_ray);
    switch (res.hit_idx) {
        case STACK_OVERFLOW:
            f_color = RED;
            break;
        case NO_HIT:
            f_color = BLACK;
            break;
        default:
            f_color = WHITE;
    }
}

hit_report cast_ray(vec3 from, vec3 ray) {
    // Indices of spheres the ray eventually collides with, with first collision on top
    int stack[STACK_SIZE];
    int stack_ptr = -1;

    int root = bodies.length() - 1;
    if (hit_time(from, ray, bodies[root].pos, bodies[root].radius) > 0) {
        stack[++stack_ptr] = root;
    }
    while (stack_ptr >= 0) {
        int hit = stack[stack_ptr--];
        if (bodies[hit].left == -1) {
            vec3 hit_pos = from + ray * hit_time(from, ray, bodies[hit].pos, bodies[hit].radius);
            return hit_report(hit_pos, hit);
        } else {
            int left = bodies[hit].left;
            int right = bodies[hit].right;
            float l_hit = hit_time(from, ray, bodies[left].pos, bodies[left].radius);
            float r_hit = hit_time(from, ray, bodies[right].pos, bodies[right].radius);
            if (r_hit < l_hit) {
                float tmpf = l_hit;
                l_hit = r_hit;
                r_hit = tmpf;

                int tmp = left;
                left = right;
                right = tmp;
            }
            if (r_hit > 0) {
                if (stack_ptr + 1 == STACK_SIZE) {
                    return hit_report(vec3(0), STACK_OVERFLOW);
                }
                stack[++stack_ptr] = right;
            }
            if (l_hit > 0) {
                if (stack_ptr + 1 == STACK_SIZE) {
                    return hit_report(vec3(0), STACK_OVERFLOW);
                }
                stack[++stack_ptr] = left;
            }
        }
    }
    return hit_report(vec3(0), NO_HIT);
}

float hit_time(vec3 from, vec3 ray, vec3 body_pos, float r) {
    /* Solve system for t:
         (xyz - body_pos)^2 == r^2
         xyz = from + ray * t
    i.e. find the intersections of the body and the camera ray.
    This is a quadratic equation At^2 - 2Bt + C == 0
    */
    const vec3 rel_pos = body_pos - from;
    const float A = dot(ray, ray);
    const float B = dot(ray, rel_pos);
    const float C = dot(rel_pos, rel_pos) - r*r;

    const float det = B*B - A*C;
    if (det < 0) {
        return -1;
    }
    const float sqrtd = sqrt(det);
    const float t1 = (B + sqrtd)/A;
    const float t2 = (B - sqrtd)/A;
    if (t1 > 0 && t2 > 0) {
        return min(t1, t2);
    } else if (t1 > 0) {
        return t1;
    } else {
        return t2;
    }
}
