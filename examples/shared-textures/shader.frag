#version 450

layout(set = 0, binding = 0) uniform texture2D t_Image;
layout(set = 0, binding = 1) uniform sampler s_Image;

layout(location = 0) in vec2 v_Uv;
layout(location = 0) out vec4 f_Color;

void main() {
    f_Color = texture(sampler2D(t_Image, s_Image), v_Uv);
}
