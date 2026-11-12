#version 450
layout(location = 0) in vec3 in_position;
layout(push_constant) uniform PC { mat4 light_vp; } pc;
void main() { gl_Position = pc.light_vp * vec4(in_position, 1.0); }
