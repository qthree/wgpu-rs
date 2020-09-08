#version 450

out gl_PerVertex {
    vec4 gl_Position;
};

layout(location = 0) out vec2 v_Uv;

void main() {
    v_Uv = vec2(gl_VertexIndex >> 1, (gl_VertexIndex & 1));
    gl_Position = vec4((v_Uv - 0.5) * -1.5, 0.0, 1.0);
}
