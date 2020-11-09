#version 450

const vec2 corner[4] = vec2[] (
    vec2(-1, -1),
    vec2(-1, 1),
    vec2(1, -1),
    vec2(1, 1)
);


// Not actually used, since we have no vertices
void main() {
    gl_Position = vec4(corner[gl_VertexIndex], 0.0, 1.0);
}
