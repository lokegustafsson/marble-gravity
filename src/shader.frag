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
const float EPSILON = 0.01;

const vec4 BLACK = vec4(vec3(0),1);
const vec4 RED = vec4(1,0,0,1);
const vec4 WHITE = vec4(1);

const vec3 AMBIENT = vec3(0.01, 0.01, 0.01);
const vec3 SOURCE = vec3(1);

const float SPECULAR_EXP = 16;
const vec3 MARBLE_COLOR = vec3(0,0,1);

// IO
in vec4 gl_FragCoord;
layout(location=0) out vec4 f_color;

// Buffers & Uniforms
layout(set=0, binding=0) readonly buffer Bodies {
    body bodies[];
};
layout(set=0, binding=1) uniform Uniforms {
    vec3 source_direction;
    float _padding;
    vec2 window_size;
};

// Forward function declarations
float hit_time(const vec3, const vec3, const vec3, const float);
hit_report cast_ray(const vec3, const vec3);

void main() {
    const vec2 frag_pos = gl_FragCoord.xy / window_size.y;
    const vec2 mid_frag_pos = vec2(0.5 * window_size.x / window_size.y, 0.5);
    const vec3 camera_ray = normalize(vec3(frag_pos - mid_frag_pos, 1));

    hit_report primary = cast_ray(vec3(0), camera_ray);
    if (primary.hit_idx == STACK_OVERFLOW) {
        f_color = RED;
        return;
    } else if (primary.hit_idx == NO_HIT) {
        f_color = BLACK;
        return;
    }
    const vec3 primary_normal = primary.pos - bodies[primary.hit_idx].pos;
    const hit_report source = cast_ray(primary.pos + EPSILON * primary_normal, source_direction);
    if (source.hit_idx == STACK_OVERFLOW) {
        f_color = RED;
        return;
    }
    vec3 light = AMBIENT * MARBLE_COLOR;
    if (source.hit_idx == NO_HIT) {
        float alignment = dot(normalize(primary_normal), normalize(source_direction - camera_ray));
        light += MARBLE_COLOR * SOURCE * alignment;
        light += SOURCE * pow(alignment, SPECULAR_EXP);
    }
    f_color = vec4(light, 1);
}

hit_report cast_ray(const vec3 from, const vec3 ray) {
    // Indices of spheres the ray eventually collides with, with first collision on top
    int stack[STACK_SIZE];
    int stack_ptr = -1;

    const int root = bodies.length() - 1;
    if (hit_time(from, ray, bodies[root].pos, bodies[root].radius) > 0) {
        stack[++stack_ptr] = root;
    }
    float first_hit_time = 1e9;
    int first_hit_target = NO_HIT;
    while (stack_ptr >= 0) {
        const int hit = stack[stack_ptr--];
        if (bodies[hit].left == -1) {
            const float time = hit_time(from, ray, bodies[hit].pos, bodies[hit].radius);
            if (time < first_hit_time) {
                first_hit_time = time;
                first_hit_target = hit;
            }
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
    return hit_report(from + ray * first_hit_time, first_hit_target);
}

float hit_time(const vec3 from, const vec3 ray, const vec3 body_pos, const float r) {
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
