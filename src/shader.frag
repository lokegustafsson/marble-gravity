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
    vec3 normal;
    int id;
};

// Constants ===
const int STACK_SIZE = 20;
const vec4 RED = vec4(1,0,0,1);
const int NO_HIT = -1;
const float EPSILON = 0.01;

const vec3 AMBIENT = vec3(0.01, 0.01, 0.01);
const vec3 SUN_COLOR = vec3(1,1,0.8);
const float SUN_SIZE = 1e-3;
const float SUN_CORONA = 1e-4;

const float SPECULAR_EXP = 16;
const vec3 MARBLE_COLOR = vec3(0,0.3,1);
const float REFRACTIVE_INDEX = 1.1;

// Global variables ===
bool stack_overflow = false;

// IO ===
in vec4 gl_FragCoord;
layout(location=0) out vec4 f_color;

// Buffers & Uniforms ===
layout(set=0, binding=0) readonly buffer Bodies {
    body bodies[];
};
// Padding is apparently necessary
layout(set=0, binding=1) uniform Uniforms {
    vec3 sun_direction;
    float _padding;
    vec2 window_size;
};

// Forward function declarations ===
float hit_time(const vec3, const vec3, const uint);
hit_report cast_ray(const vec3, const vec3);
vec3 simple_ray(const vec3, const vec3);
vec3 double_ray(const vec3, const vec3);
vec3 background_light(const vec3);
vec3 primary_illumination(const vec3, const vec3, const vec3);

void main() {
    const vec2 frag_pos = gl_FragCoord.xy / window_size.y;
    const vec2 mid_frag_pos = vec2(0.5 * window_size.x / window_size.y, 0.5);
    const vec3 camera_ray = normalize(vec3(frag_pos - mid_frag_pos, 1));
    f_color = vec4(double_ray(vec3(0), camera_ray), 1);
    if (stack_overflow) {
        f_color = RED;
    }
}

// Casts a ray with a single reflection and refraction
vec3 double_ray(const vec3 from, const vec3 ray) {
    hit_report hit = cast_ray(vec3(0), ray);
    if (hit.id == NO_HIT) {
        return background_light(ray);
    }
    const vec3 normal = hit.normal;
    const vec3 hit_centre = bodies[hit.id].pos;
    const vec3 hit_from_centre = normal * bodies[hit.id].radius;
    const vec3 hit_point = hit_centre + (1 + EPSILON) * hit_from_centre;

    vec3 light = primary_illumination(hit_point, normal, ray);
    // Reflected
    light += simple_ray(hit_point, reflect(ray, normal));

    { // Refracted
        const vec3 out_of_plane = cross(ray, normal);
        const vec3 internal_ray = refract(ray, normal, 1/REFRACTIVE_INDEX);
        const vec3 internal_normal = cross(out_of_plane, internal_ray);
        const vec3 exit_pos = hit_centre + (1 + EPSILON) * reflect(-hit_from_centre, internal_normal);
        const vec3 exit_ray = reflect(ray, internal_normal);
        light += simple_ray(exit_pos, exit_ray);
    }
    return light;
}

// Casts a ray considering the sun, but ignores reflection and refraction
vec3 simple_ray(const vec3 from, const vec3 ray) {
    hit_report hit = cast_ray(from, ray);
    if (hit.id == NO_HIT) {
        return background_light(ray);
    }
    const vec3 normal = hit.normal;
    const vec3 hit_centre = bodies[hit.id].pos;
    const vec3 hit_from_centre = normal * bodies[hit.id].radius;
    const vec3 hit_point = hit_centre + (1 + EPSILON) * hit_from_centre;

    return primary_illumination(hit_point, normal, ray);
}

// What color is the background in the [ray] direction?
vec3 background_light(const vec3 ray) {
    const float alignment = max(0, dot(ray, sun_direction));
    return SUN_COLOR * pow(min(1, SUN_SIZE + alignment), 1/SUN_CORONA);
}

// When a ray travelling along [ray] hits a surface with normal [normal] in
// [location], what color given by the global illumination according to
// Blinn-Phong?
vec3 primary_illumination(const vec3 location, const vec3 normal, const vec3 ray) {
    // Ambient
    vec3 light = AMBIENT * MARBLE_COLOR;
    if (cast_ray(location, sun_direction).id == NO_HIT) {
        const float alignment = dot(normal, normalize(sun_direction - ray));
        light += MARBLE_COLOR * SUN_COLOR * alignment;
        light += SUN_COLOR * pow(alignment, SPECULAR_EXP);
    }
    return light;
}

// Cast a ray by traversing the body tree. Will set [stack_overflow] on overflow
hit_report cast_ray(const vec3 from, const vec3 ray) {
    int stack[STACK_SIZE];
    int stack_ptr = -1;

    const int root = bodies.length() - 1;
    if (hit_time(from, ray, root) > 0) {
        stack[++stack_ptr] = root;
    }
    float first_hit_time = 1e9;
    int first_hit_target = NO_HIT;
    while (stack_ptr >= 0) {
        const int hit = stack[stack_ptr--];
        if (bodies[hit].left == -1) {
            const float time = hit_time(from, ray, hit);
            if (time < first_hit_time) {
                first_hit_time = time;
                first_hit_target = hit;
            }
        } else {
            int left = bodies[hit].left;
            int right = bodies[hit].right;
            float l_hit = hit_time(from, ray, left);
            float r_hit = hit_time(from, ray, right);
            if (r_hit < l_hit) {
                float tmpf = l_hit;
                l_hit = r_hit;
                r_hit = tmpf;

                int tmpi = left;
                left = right;
                right = tmpi;
            }
            if (r_hit > 0) {
                if (stack_ptr + 1 == STACK_SIZE) {
                    stack_overflow = true;
                    return hit_report(vec3(0), NO_HIT);
                }
                stack[++stack_ptr] = right;
            }
            if (l_hit > 0) {
                if (stack_ptr + 1 == STACK_SIZE) {
                    stack_overflow = true;
                    return hit_report(vec3(0), NO_HIT);
                }
                stack[++stack_ptr] = left;
            }
        }
    }
    const vec3 hit_pos = from + ray * first_hit_time;
    return hit_report(normalize(hit_pos - bodies[first_hit_target].pos), first_hit_target);
}

// When will the ray from [from] along [ray] intersect body [body]?
float hit_time(const vec3 from, const vec3 ray, const uint body) {
    /* Solve system for t:
         (xyz - body_pos)^2 == r^2
         xyz = from + ray * t
    i.e. find the intersections of the body and the camera ray.
    This is a quadratic equation At^2 - 2Bt + C == 0
    */
    const vec3 rel_pos = bodies[body].pos - from;
    const float r = bodies[body].radius;

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
