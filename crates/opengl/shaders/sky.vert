// Sky vertex shader - full-screen triangle using gl_VertexID (OpenGL 3.3)
// Output z=1.0 (far plane) so sky renders at infinite depth behind all geometry
#version 330 core

out vec2 frag_uv;

void main() {
    // Full-screen triangle using gl_VertexID (no vertex buffer needed)
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
    
    gl_Position = vec4(positions[gl_VertexID], 1.0, 1.0);
    frag_uv = uvs[gl_VertexID];
}