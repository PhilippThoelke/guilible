import math
import random

import numpy as np

from guilible import BaseWindow
from guilible.ui import Rectangle


class Window(BaseWindow):
    def setup(self):
        self.background_color = (0.1, 0.15, 0.15)

        self.damp = 0.9
        self.force = np.array([0, -0.01])
        self.vel = np.zeros(2)
        self.rot = 0

        self.outer_box = Rectangle(0, 0.2, 0.2, 0.2, (1, 1, 1))
        self.inner_box = Rectangle(0, 0, 0.95, 0.95, (0.5, 0, 1))
        self.chain = [Rectangle(0, 0, 0.1, 0.1, (0, 1, 0)) for _ in range(20)]
        self.ui.add(self.outer_box)
        self.outer_box.add(self.inner_box)
        self.inner_box.add(*self.chain)
        for box in self.chain:
            box.add(Rectangle(0, 0, 0.5, 0.5, (0, 0, 0)))

    def update(self, delta: float):
        self.wnd.title = f"{1/delta:.2f} FPS"

        self.vel += self.force
        self.outer_box.x += delta * self.vel[0]
        self.outer_box.y += delta * self.vel[1]

        if self.outer_box.y + self.outer_box.h > 1:
            self.outer_box.y = 1 - self.outer_box.h
            self.vel[1] *= -self.damp
            self.inner_box.color = (random.random(), random.random(), random.random())
        if self.outer_box.y - self.outer_box.h < -1:
            self.outer_box.y = -1 + self.outer_box.h
            self.vel[1] *= -self.damp
            self.inner_box.color = (random.random(), random.random(), random.random())
        if self.outer_box.x + self.outer_box.w > 1:
            self.outer_box.x = 1 - self.outer_box.w
            self.vel[0] *= -self.damp
            self.inner_box.color = (random.random(), random.random(), random.random())
        if self.outer_box.x - self.outer_box.w < -1:
            self.outer_box.x = -1 + self.outer_box.w
            self.vel[0] *= -self.damp
            self.inner_box.color = (random.random(), random.random(), random.random())

        speed = np.linalg.norm(self.vel)
        self.rot += speed**2 * delta * 5 * np.sign(self.vel[0])
        for i, box in enumerate(self.chain):
            box.x = math.sin(self.rot + math.pi * 2 / len(self.chain) * i) * (1 - box.w) * 0.75
            box.y = math.cos(self.rot + math.pi * 2 / len(self.chain) * i) * (1 - box.h) * 0.75
            box.color = (
                1 - self.inner_box.color[0] + (math.sin(self.time + math.pi * 2 / len(self.chain) * i) / 2 + 0.5),
                1 - self.inner_box.color[1] + (math.cos(self.time + math.pi * 2 / len(self.chain) * i) / 2 + 0.5),
                1 - self.inner_box.color[2] + (math.tan(self.time + math.pi * 2 / len(self.chain) * i) / 2 + 0.5),
            )

    def on_mouse_scroll_event(self, x_offset, y_offset):
        self.outer_box.w += x_offset * 0.1
        self.outer_box.w = max(0.02, self.outer_box.w)
        self.outer_box.h += y_offset * -0.1
        self.outer_box.h = max(0.02, self.outer_box.h)

    def on_key_event(self, key, action, modifiers):
        if self.wnd.is_key_pressed(self.wnd.keys.SPACE):
            self.force[1] = 0.01
        else:
            self.force[1] = -0.01

        self.force[0] = 0
        if self.wnd.is_key_pressed(self.wnd.keys.LEFT):
            self.force[0] = -0.01
        elif self.wnd.is_key_pressed(self.wnd.keys.RIGHT):
            self.force[0] = 0.01


if __name__ == "__main__":
    Window.run()
