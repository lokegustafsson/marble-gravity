#version 450

struct body {
    vec3 pos;
    float radius;
};

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


// Ray tracing with parameters:
// - Roughness
// - Index of refraction relative to air
void main() {
    vec2 frag_pos = gl_FragCoord.xy / window_size.y;
    vec2 mid_frag_pos = vec2(0.5 * window_size.x / window_size.y, 0.5);
    vec3 camera_ray = vec3(frag_pos - mid_frag_pos, 1);
    bool ray_hits_body = false;

    for (int i = 0; i < bodies.length(); i++) {
        vec3 body_pos = bodies[i].pos;
        float r = bodies[i].radius;
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
        float t1 = (B + sqrt(det))/A;
        float t2 = (B - sqrt(det))/A;
        if (det >= 0 && (t1 > 0 || t2 > 0)) {
            ray_hits_body = true;
            break;
        }
    }
    if (ray_hits_body) {
        f_color = vec4(1);
    } else {
        f_color = vec4(0, 0, 0, 1);
    }
}
