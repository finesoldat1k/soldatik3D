// Sky vertex shader - full-screen triangle using gl_VertexIndex
// Output z=1.0 (far plane) so sky renders at infinite depth behind all geometry
#version 450

layout(location = 0) out vec2 frag_uv;

void main() {
    // Full-screen triangle using gl_VertexIndex (no vertex buffer needed)
    vec2 positions[3] = vec2[3](
        vec2(-1.0, -1.0),
        vec2( 3.0, -1.0),
        vec2(-1.0,  3.0)
    );
    vec2 uvs[3] = vec2[3](
        vec2(0.0, 0.0),
        vec2(2.0, 0.0),
        vec2(0.0, 2.0)
    );
    
    gl_Position = vec4(positions[gl_VertexIndex], 1.0, 1.0);
    frag_uv = uvs[gl_VertexIndex];
}