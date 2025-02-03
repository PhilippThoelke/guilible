from typing import Tuple

from guilible.ui.base import UIElement


class Rectangle(UIElement):

    VERTICES = [-1, -1, 0, 1, -1, 0, 1, 1, 0, -1, -1, 0, 1, 1, 0, -1, 1, 0]

    VERTEX_SHADER = """
    #version 330
    in vec3 pos;
    in vec4 rect;
    in vec3 color;

    out vec3 v_color;

    void main() {
        vec2 p = vec2(pos.x * rect.z, pos.y * rect.w) + vec2(rect.x, rect.y);
        gl_Position = vec4(p, 0.0, 1.0);
        v_color = color;
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
