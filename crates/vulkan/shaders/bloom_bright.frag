#version 450

layout(binding = 0) uniform sampler2D hdr_input;
layout(location = 0) in vec2 frag_uv;
layout(location = 0) out vec4 out_color;

// Только лампы (emission=15) дают значения > 3.0
// Спекулярные блики на стенах (~1.0-2.0) не должны попадать
const float BRIGHT_THRESHOLD = 2.0;

void main() {
    vec3 color = texture(hdr_input, frag_uv).rgb;
    float luminance = dot(color, vec3(0.2126, 0.7152, 0.0722));
    if (luminance > BRIGHT_THRESHOLD) {
        out_color = vec4(color, 1.0);
    } else {
        out_color = vec4(0.0, 0.0, 0.0, 1.0);
    }
}
