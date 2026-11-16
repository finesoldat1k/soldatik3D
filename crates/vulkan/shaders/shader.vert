#version 450

layout(location = 0) in vec3 in_position;
layout(location = 1) in vec3 in_color;
layout(location = 2) in vec3 in_normal;
layout(location = 3) in vec2 in_uv;
layout(location = 4) in float in_emission;
layout(location = 5) in vec2 in_material;

layout(location = 0) out vec3 frag_color;
layout(location = 1) out vec3 frag_normal;
layout(location = 2) out vec3 frag_pos;
layout(location = 3) out vec2 frag_uv;
layout(location = 4) out float frag_emission;
layout(location = 5) out vec2 frag_material;

layout(push_constant) uniform PushConstants {
    mat4 mvp;
    mat4 light_vp;
    vec4 light_pos;
    vec4 light_color;
    vec4 camera_pos;
    vec4 screen_and_debug;
    vec4 material; // x=metallic, y=roughness, z=unused, w=unused
} pc;

void main() {
    gl_Position = pc.mvp * vec4(in_position, 1.0);
    frag_color = in_color;
    frag_normal = in_normal;
    frag_pos = in_position;
    frag_uv = in_uv;
    frag_emission = in_emission;
    frag_material = in_material;
}
