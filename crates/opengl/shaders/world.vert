#version 330 core

layout(location = 0) in vec3 in_position;
layout(location = 1) in vec3 in_color;
layout(location = 2) in vec3 in_normal;
layout(location = 3) in vec2 in_uv;
layout(location = 4) in float in_emission;
layout(location = 5) in vec2 in_material;

uniform mat4 u_mvp;

out vec3 frag_color;
out vec3 frag_normal;
out vec3 frag_pos;
out vec2 frag_uv;
out float frag_emission;
out vec2 frag_material;

void main() {
    gl_Position = u_mvp * vec4(in_position, 1.0);
    frag_color = in_color;
    frag_normal = in_normal;
    frag_pos = in_position;
    frag_uv = in_uv;
    frag_emission = in_emission;
    frag_material = in_material;
}
