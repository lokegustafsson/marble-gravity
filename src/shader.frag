#version 450

// Note that the rust side includes 64 bit padding at the end which is implicit here
// Buffer items need their size to be a multiple of 128 bits. This struct is 384 bits.
struct Body {
    vec3 pos;
    float radius;
    vec4 color;
    int left;
    int right;
};
// Internal structs
struct HitReport {
    vec3 normal;
    int id;
};
struct Rays {
    vec3 reflected_pos;
    vec3 reflected_ray;
    vec3 refracted_pos;
    vec3 refracted_ray;
};


// Constants ===
const int STACK_SIZE = 20;
const vec4 RED = vec4(1,0,0,1);
const int NO_HIT = -1;
const float EPSILON = 0.01;

const vec3 AMBIENT = vec3(0.08); // Uniform?
const vec3 SUN_COLOR = vec3(1); // Uniform?
const float SUN_SIZE = 1e-2;
const float SUN_CORONA = 1e-3;
const float REFRACTIVE_INDEX = 1.1;

// Global variables ===
bool stack_overflow = false;

// IO ===
in vec4 gl_FragCoord;
layout(location=0) out vec4 f_color;

// Buffers & Uniforms ===
layout(set=0, binding=0) readonly buffer Bodies {
    Body bodies[];
};
// Padding is apparently necessary
layout(set=0, binding=1) uniform Uniforms {
    vec3 sun_direction;
    float _padding;
    vec2 window_size;
};

// Forward function declarations ===
float softmax(float a, float b, float c);
float rings(float x);
vec3 background_light(const vec3 ray);
float hit_time(const vec3 from, const vec3 ray, const uint body);
HitReport cast_ray(const vec3 from, const vec3 ray);
vec3 refract3(vec3 incident, vec3 normal, float eta);
Rays ray_tracing_data(const vec3 normal, const vec3 ray, const uint hit_id);
vec3 simple_ray(const vec3 from, const vec3 ray);
vec3 double_ray(const vec3 from, const vec3 ray);
vec3 triple_ray(const vec3 from, const vec3 ray);
vec3 quad_ray(const vec3 from, const vec3 ray);

void fs_main() {
    const vec2 frag_pos = gl_FragCoord.xy / window_size.y;
    const vec2 mid_frag_pos = vec2(0.5 * window_size.x / window_size.y, 0.5);
    const vec3 camera_ray = normalize(vec3(frag_pos - mid_frag_pos, 1));
    f_color = vec4(triple_ray(vec3(0), camera_ray), 1);
    if (stack_overflow) {
        f_color = RED;
    }
}

// Casts a ray with a triple reflections and refractions
vec3 quad_ray(const vec3 from, const vec3 ray) {
    const HitReport hit = cast_ray(from, ray);
    if (hit.id == NO_HIT) {
        return background_light(ray);
    }
    const Rays next = ray_tracing_data(hit.normal, ray, hit.id);
    const float opacity = bodies[hit.id].color.w;

    vec3 light = AMBIENT * opacity * bodies[hit.id].color.xyz; // Ambient
    light += opacity * triple_ray(next.reflected_pos, next.reflected_ray); // Reflected
    light += (1 - opacity) * triple_ray(next.refracted_pos, next.refracted_ray); // Refracted
    return light;
}
void main() {
    fs_main();
}

// Casts a ray with a double reflections and refractions
vec3 triple_ray(const vec3 from, const vec3 ray) {
    const HitReport hit = cast_ray(from, ray);
    if (hit.id == NO_HIT) {
        return background_light(ray);
    }
    const Rays next = ray_tracing_data(hit.normal, ray, hit.id);
    const float opacity = bodies[hit.id].color.w;

    vec3 light = AMBIENT * opacity * bodies[hit.id].color.xyz; // Ambient
    light += opacity * double_ray(next.reflected_pos, next.reflected_ray); // Reflected
    light += (1 - opacity) * double_ray(next.refracted_pos, next.refracted_ray); // Refracted
    return light;
}

// Casts a ray with a single reflection and refraction
vec3 double_ray(const vec3 from, const vec3 ray) {
    const HitReport hit = cast_ray(from, ray);
    if (hit.id == NO_HIT) {
        return background_light(ray);
    }
    const Rays next = ray_tracing_data(hit.normal, ray, hit.id);
    const float opacity = bodies[hit.id].color.w;

    vec3 light = AMBIENT * opacity * bodies[hit.id].color.xyz; // Ambient
    light += opacity * simple_ray(next.reflected_pos, next.reflected_ray); // Reflected
    light += (1 - opacity) * simple_ray(next.refracted_pos, next.refracted_ray); // Refracted
    return light;
}

vec3 refract3(vec3 incident, vec3 normal, float eta) {
    const float k = 1.0 - eta * eta * (1.0 - dot(normal, incident) * dot(normal, incident));
    if (k < 0.0) {
        return vec3(0);
    } else {
        return eta * incident - (eta * dot(normal, incident) + sqrt(k)) * normal;
    }
}
// Computes values necessary for casting reflected and refracted rays
Rays ray_tracing_data(const vec3 normal, const vec3 ray, const uint hit_id) {
    const vec3 hit_centre = bodies[hit_id].pos;
    const vec3 hit_from_centre = normal * bodies[hit_id].radius;
    const vec3 entry_pos = hit_centre + (1 + EPSILON) * hit_from_centre;

    const vec3 out_of_plane = cross(ray, normal);
    const vec3 internal_ray = refract3(ray, normal, 1/REFRACTIVE_INDEX);
    const vec3 internal_normal = normalize(cross(out_of_plane, internal_ray));
    const vec3 exit_pos = hit_centre + (1 + EPSILON) * reflect(-hit_from_centre, internal_normal);
    const vec3 exit_ray = normalize(reflect(ray, internal_normal));

    return Rays(entry_pos, reflect(ray, normal), exit_pos, exit_ray);
}

// Casts a ray using Blinn-Phong illumination
vec3 simple_ray(const vec3 from, const vec3 ray) {
    HitReport hit = cast_ray(from, ray);
    if (hit.id == NO_HIT) {
        return background_light(ray);
    }
    const vec3 normal = hit.normal;
    const vec3 hit_point = bodies[hit.id].pos + (1 + EPSILON) * bodies[hit.id].radius * normal;
    const vec3 color = bodies[hit.id].color.xyz;
    const float opacity = bodies[hit.id].color.w;

    // Ambient
    vec3 light = AMBIENT * opacity * color;
    if (cast_ray(hit_point, sun_direction).id == NO_HIT) {
        const float alignment = dot(normal, normalize(sun_direction - ray));
        // Diffuse
        light += color * SUN_COLOR * opacity * alignment;
        // Specular
        light += SUN_COLOR * (1 - opacity) * pow(alignment, inversesqrt(SUN_CORONA));
    }
    return light;
}

float softmax(float a, float b, float c) {
    float M = max(max(exp(a), exp(b)), exp(c));
    return (M - 1) / (exp(a) + exp(b) + exp(c));
}
float rings(float x) {
    x = 3.141592 / 4 * (x - 1);
    float a = pow(abs(sin(6*x)), 40);
    float b = pow(abs(sin(17*x)), 40);
    float c = pow(abs(sin(20*x)), 40);
    return softmax(a, b, c);
}

// What color is the background in the [ray] direction?
vec3 background_light(const vec3 ray) {
    const float alignment = max(0, dot(ray, sun_direction));
    vec3 sun = SUN_COLOR * min(1, pow(SUN_SIZE + alignment, 1/SUN_CORONA));
    float rings = 0.04 * rings(dot(ray, sun_direction));
    return sun + vec3(rings);
}

// Cast a ray by traversing the body tree. Will set [stack_overflow] on overflow
HitReport cast_ray(const vec3 from, const vec3 ray) {
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
                    return HitReport(vec3(0), NO_HIT);
                }
                stack[++stack_ptr] = right;
            }
            if (l_hit > 0) {
                if (stack_ptr + 1 == STACK_SIZE) {
                    stack_overflow = true;
                    return HitReport(vec3(0), NO_HIT);
                }
                stack[++stack_ptr] = left;
            }
        }
    }
    const vec3 hit_pos = from + ray * first_hit_time;
    return HitReport(normalize(hit_pos - bodies[first_hit_target].pos), first_hit_target);
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
        return -1.0;
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
