#version 450

// Note that the rust side includes 64 bit padding at the end which is implicit here
// Buffer items need their size to be a multiple of 128 bits. This struct is 256 bits.
struct body {
    vec3 pos;
    float radius;
    int left;
    int right;
};
const int STACK_SIZE = 20;
const vec4 BLACK = vec4(0,0,0,1);
const vec4 RED = vec4(1,0,0,1);
const vec4 WHITE = vec4(1);

// Fragment specific
in vec4 gl_FragCoord;
layout(location=0) out vec4 f_color;

// Global
layout(set=0, binding=0) readonly buffer Bodies {
    body bodies[];
};
layout(set=0, binding=1) uniform Uniforms {
    vec2 window_size;
};

// Fragment global variables
vec3 camera_ray;

// Functions
float cast_ray(vec3, float);

void main() {
    vec2 frag_pos = gl_FragCoord.xy / window_size.y;
    vec2 mid_frag_pos = vec2(0.5 * window_size.x / window_size.y, 0.5);
    camera_ray = vec3(frag_pos - mid_frag_pos, 1);

    // Indices of spheres the ray eventually collides with, with first collision on top
    int stack[STACK_SIZE];
    int stack_ptr = -1;

    int root = bodies.length() - 1;
    if (cast_ray(bodies[root].pos, bodies[root].radius) > 0) {
        stack[++stack_ptr] = root;
    }
    while (stack_ptr >= 0) {
        int hit = stack[stack_ptr--];
        if (bodies[hit].left == -1) {
            f_color = WHITE;
            return;
        } else {
            int left = bodies[hit].left;
            int right = bodies[hit].right;
            float l_hit = cast_ray(bodies[left].pos, bodies[left].radius);
            float r_hit = cast_ray(bodies[right].pos, bodies[right].radius);
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
                    f_color = RED;
                    return;
                }
                stack[++stack_ptr] = right;
            }
            if (l_hit > 0) {
                if (stack_ptr + 1 == STACK_SIZE) {
                    f_color = RED;
                    return;
                }
                stack[++stack_ptr] = left;
            }
        }
    }
    f_color = BLACK;
}

float cast_ray(vec3 body_pos, float r) {
    /* Solve system for t:
         (xyz - body_pos)^2 == r^2
         xyz = camera_ray * t
    i.e. find the intersections of the body and the camera ray.
    This is a quadratic equation At^2 - 2Bt + C == 0
    */
    float A = dot(camera_ray, camera_ray);
    float B = dot(camera_ray, body_pos);
    float C = dot(body_pos, body_pos) - r*r;

    float det = B*B - A*C;
    if (det < 0) {
        return -1;
    }
    float t1 = (B + sqrt(det))/A;
    float t2 = (B - sqrt(det))/A;
    if (t1 > 0 && t2 > 0) {
        return min(t1, t2);
    } else if (t1 > 0) {
        return t1;
    } else {
        return t2;
    }
}
