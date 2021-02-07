#version 410
#extension GL_ARB_explicit_uniform_location : enable

precision highp float;

layout(location = 1) uniform sampler2D u_Albedo;

// Varyings
in vec4 v_Color;
in vec2 v_TexCoord;

out vec4 Color;

void main() {
    Color = v_Color * texture(u_Albedo, v_TexCoord);
}
