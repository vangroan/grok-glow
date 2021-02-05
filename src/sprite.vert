#version 410

// Called attributes in OpenGL's API.
in vec2 a_Pos;
in vec2 a_UV;

// Canvas width and height.
// Allows us to pass vertex position to shader as
// number of pixels.
// This could be a matrix too.
uniform vec2 u_Resolution;

// Varyings are values sent from the vertex shader to
// the fragment shader. The value that reaches the fragment
// shader is interpolated between the vertices.
out vec4 v_Color;
out vec2 v_TexCoord;

void main() {
    // Convert the position from pixels to 0.0 to 1.0
    vec2 normalised_pos = a_Pos / u_Resolution;

    // Convert from 0->1 to 0->2, since clip space is 2 wide and height.
    vec2 normalised_pos_2 = normalised_pos * 2;

    // Convert from 0->2 to -1->+1 (clip space)
    vec2 pos = normalised_pos_2 - 1.0;

    // In clip space the bottom left corner is -1,-1.
    // To get a traditional 2D pixel space where 0,0 is top left, we flip the y.
    gl_Position = vec4(pos * vec2(1, -1), 0.0, 1.0);

    v_Color = vec4(1.0, 1.0, 1.0, 1.0);
    v_TexCoord = a_UV;
}
