from typing import List, Optional, Tuple, Type

import moderngl as mgl
import numpy as np

from guilible import BaseWindow


class LinePlot:
    def __init__(self, x: np.ndarray, y: np.ndarray, lw: float = 0.001):
        self._data = None
        self.lw = lw

        self.update(x, y)

        self._vertex_cache = None

    def update(self, x: np.ndarray, y: np.ndarray):
        x = np.asarray(x, dtype="f4")
        y = np.asarray(y, dtype="f4")

        if x.shape != y.shape:
            raise ValueError("X and Y must have the same shape")
        if x.ndim == 1:
            x = x[None]
        elif x.ndim != 2:
            raise ValueError("Data must be 1D or 2D")
        if y.ndim == 1:
            y = y[None]
        elif y.ndim != 2:
            raise ValueError("Data must be 1D or 2D")

        self._data = np.stack((x, y), axis=1)

    def get_vertices(self) -> Tuple[int, np.ndarray, np.ndarray]:
        normal = np.empty_like(self._data)
        normal[..., :-1] = np.diff(self._data, axis=-1)
        normal[..., -1] = normal[..., -2]
        normal = np.stack((normal[:, 1], -normal[:, 0]), axis=1)
        normal[..., 1:-1] = (normal[..., 1:-1] + normal[..., :-2]) / 2
        normal /= np.linalg.norm(normal, axis=1, keepdims=True)

        vertices = np.empty((self._data.shape[0], 4, self._data.shape[2]), dtype="f4")
        vertices[:, :2] = self._data - normal * self.lw
        vertices[:, 2:] = self._data + normal * self.lw
        vertices = vertices.transpose(0, 2, 1).reshape(-1)

        indices = np.array([0, 1, 2, 1, 3, 2], dtype="i4")
        indices = np.tile(indices, self._data.shape[0] * self._data.shape[2])
        indices += np.repeat(np.arange(self._data.shape[0] * self._data.shape[2]) * 2, 6)

        return indices.size, vertices, indices


class Window(BaseWindow):
    NUM_SHADER_INPUTS = 2
    INITIAL_BUFFER_SIZE = 1024

    def setup(self):
        program = self.ctx.program(
            vertex_shader="""
            #version 330

            in vec2 in_pos;

            void main() {
                gl_Position = vec4(in_pos, 0.0, 1.0);
            }
            """,
            fragment_shader="""
            #version 330

            out vec4 out_color;

            void main() {
                out_color = vec4(1.0);
            }
            """,
        )

        # Initialize the VBO and IBO
        self.vbo = self.ctx.buffer(reserve=self.INITIAL_BUFFER_SIZE * self.NUM_SHADER_INPUTS * 4, dynamic=True)
        self.ibo = self.ctx.buffer(reserve=self.INITIAL_BUFFER_SIZE * 4, dynamic=True)
        # Create the VAO
        self.vao = self.ctx.vertex_array(program, [(self.vbo, "2f", "in_pos")], self.ibo)

        # Create the line plot
        n_points, n_seg = 100_000, 10
        self.x = np.array([np.linspace(-0.8, 0.8, n_points)] * n_seg)
        self.y = np.array([np.random.rand(n_points) / n_seg / 2 + (i - n_seg // 2) / n_seg * 1.5 for i in range(n_seg)])
        self.lp = LinePlot(self.x, self.y, lw=0.0005)

        self.n_vertices, vertices, indices = self.lp.get_vertices()
        self.write(vertices, indices)

    def write(self, vertices, indices):
        while self.vbo.size < vertices.nbytes:
            self.vbo.orphan(self.vbo.size * 2)
        while self.ibo.size < indices.nbytes:
            self.ibo.orphan(self.ibo.size * 2)
        self.vbo.write(vertices)
        self.ibo.write(indices)

    def update(self, delta):
        # Update the window title with the FPS
        self.wnd.title = f"{1/delta:,.1f} FPS"

        # Clear the screen
        self.ctx.clear(0.1, 0.1, 0.1)

        self.lp.update(self.x, self.y + np.sin(self.time * 2) * 0.1)
        self.n_vertices, vertices, indices = self.lp.get_vertices()
        self.write(vertices, indices)

        # Render the quads
        self.vao.render(mgl.TRIANGLES, vertices=self.n_vertices)


if __name__ == "__main__":
    Window.run(init_guilible_components=False)
