#version 330 core

layout(location = 0) in vec2 in_pos;
layout(location = 1) in vec4 in_color;
layout(location = 2) in vec2 in_uv;

out vec4 frag_color;
out vec2 frag_uv;

void main() {
    // The draw list uses a top-left origin; OpenGL's NDC has Y pointing up.
    gl_Position = vec4(in_pos.x, -in_pos.y, 0.0, 1.0);
    frag_color = in_color;
    frag_uv = in_uv;
}
