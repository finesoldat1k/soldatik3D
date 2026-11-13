#version 330 core

uniform sampler2D hdr_input;

in vec2 frag_uv;
out vec4 out_color;

const float BRIGHT_THRESHOLD = 2.0;

void main() {
    vec3 color = texture(hdr_input, frag_uv).rgb;
    float luminance = dot(color, vec3(0.2126, 0.7152, 0.0722));
    out_color = luminance > BRIGHT_THRESHOLD ? vec4(color, 1.0) : vec4(0.0);
}
