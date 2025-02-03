from functools import lru_cache

import moderngl as mgl
import numpy as np

from guilible import BaseWindow


class Window(BaseWindow):
    def setup(self):
        self.n = 1
        self.geom = 1

        self.program = self.ctx.program(
            vertex_shader="""
            #version 330

            in vec2 in_pos;
            in vec3 in_col;

            out vec3 color;

            void main() {
                gl_Position = vec4(in_pos, 0.0, 1.0);
                color = in_col;
            }
        """,
            fragment_shader="""
            #version 330

            in vec3 color;

            out vec4 fragColor;

            void main() {
                fragColor = vec4(color, 1.0);
            }
        """,
        )

    def update(self, delta):
        self.n = int(max(1, self.wnd._mouse_pos[0] ** 2.2))

        self.wnd.title = f"{1/delta:,.1f} FPS, n={self.n:,}"
        self.ctx.clear(0.1, 0.1, 0.1)

        # assemble monolithic vertex buffer
        vbo, ibo = [], []
        if (points := self.get_points(self.n, self.geom)) is not None:
            vbo.append(points[0])
            ibo.append(points[1])
        if (lines := self.get_lines(self.n, self.geom)) is not None:
            vbo.append(lines[0])
            ibo.append(lines[1])
        if (triangles := self.get_triangles(self.n, self.geom)) is not None:
            vbo.append(triangles[0])
            ibo.append(triangles[1])

        vertex_offsets = np.cumsum([0] + [len(v) for v in vbo])[:-1].tolist()
        vbo = self.ctx.buffer(np.concatenate(vbo)) if vbo else None

        ibo = np.concatenate(ibo) if len(ibo) > 0 else []
        ibo = self.ctx.buffer(ibo) if len(ibo) > 0 else None

        if vbo is not None:
            # create vertex buffer
            vao = self.ctx.vertex_array(
                self.program,
                [(vbo, "2f 3f", "in_pos", "in_col")],
                index_buffer=ibo,
            )

            # render
            if points is not None:
                vao.render(mgl.POINTS, len(points[0]), vertex_offsets.pop(0))
            if lines is not None:
                vao.render(mgl.LINES, len(lines[0]), vertex_offsets.pop(0))
            if triangles is not None:
                vao.render(mgl.TRIANGLES, len(triangles[0]), vertex_offsets.pop(0))

    def on_key_event(self, key, action, modifiers):
        # set geometry type to number key
        if action == self.wnd.keys.ACTION_PRESS and key == self.wnd.keys.NUMBER_0:
            self.geom = 0
        elif action == self.wnd.keys.ACTION_PRESS and key == self.wnd.keys.NUMBER_1:
            self.geom = 1
        elif action == self.wnd.keys.ACTION_PRESS and key == self.wnd.keys.NUMBER_2:
            self.geom = 2
        elif action == self.wnd.keys.ACTION_PRESS and key == self.wnd.keys.NUMBER_3:
            self.geom = 3
        elif action == self.wnd.keys.ACTION_PRESS and key == self.wnd.keys.NUMBER_4:
            self.geom = 4
        elif action == self.wnd.keys.ACTION_PRESS and key == self.wnd.keys.NUMBER_5:
            self.geom = 5
        elif action == self.wnd.keys.ACTION_PRESS and key == self.wnd.keys.NUMBER_6:
            self.geom = 6
        elif action == self.wnd.keys.ACTION_PRESS and key == self.wnd.keys.NUMBER_7:
            self.geom = 7

    @lru_cache(maxsize=1)
    def get_points(self, n, geom):
        if geom in (1, 4, 6, 7):
            return np.random.rand(n, 5).astype(np.float32) * 2 - 1, np.empty(0, dtype=np.uint32)

    @lru_cache(maxsize=1)
    def get_lines(self, n, geom):
        if geom in (2, 4, 5, 7):
            return np.random.rand(n, 5).astype(np.float32) * 2 - 1, np.arange(n * 2, dtype=np.uint32)

    @lru_cache(maxsize=1)
    def get_triangles(self, n, geom):
        if geom in (3, 5, 6, 7):
            return np.random.rand(n, 5).astype(np.float32) * 2 - 1, np.arange(n * 3, dtype=np.uint32)


if __name__ == "__main__":
    Window.run(init_guilible_components=False)
