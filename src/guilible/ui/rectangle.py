from typing import Tuple, Union

from guilible.ui.base import UIElement


class Rectangle(UIElement):

    VERTEX_SHADER = """
    #version 330
    in float params[7]; // x, y, w, h, r, g, 

    out vec3 v_color;

    vec3 vertices[6] = vec3[](
        vec3(-1, -1, 0),
        vec3(1, -1, 0),
        vec3(1, 1, 0),
        vec3(-1, -1, 0),
        vec3(1, 1, 0),
        vec3(-1, 1, 0)
    );

    void main() {
        vec3 pos = vertices[gl_VertexID] * vec3(params[2], params[3], 1);
        pos.xy += vec2(params[0], params[1]);
        gl_Position = vec4(pos, 1.0);
        v_color = vec3(params[4], params[5], params[6]);
    }
    """

    FRAGMENT_SHADER = """
    #version 330
    in vec3 v_color;
    out vec4 fragColor;

    void main() {
        fragColor = vec4(v_color, 1.0);
    }
    """

    def __init__(self, x: float, y: float, w: float, h: float, color: Tuple[float, float, float]):
        super().__init__(x, y, w, h)
        self.color = color

    @property
    def params(self) -> Tuple[float, ...]:
        return self.x, self.y, self.w, self.h, *self.color
