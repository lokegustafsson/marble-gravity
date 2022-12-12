#version 450

const vec2 corner[4] = vec2[] (
    vec2(-1, -1),
    vec2(-1, 1),
    vec2(1, -1),
    vec2(1, 1)
);

out vec4 gl_Position;

void vs_main() {
    gl_Position = vec4(corner[gl_VertexIndex], 0.0, 1.0);
}
void main() {
    vs_main();
}
