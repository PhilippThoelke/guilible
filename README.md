<h1 align="center">🙃 guilible 🙃</h1>
<h3 align="center">—:— the gui library that believes in you —:—</h3>


> [!CAUTION]
> ### !!! VERY EARLY STAGE OF DEVELOPMENT !!!
> #### This library is not yet ready for use. It lacks most features, is not stable and more of a testing ground at this point.

## Description
`guilible` is a simple and extensible cross-platform GPU-accelerated GUI library for Python. Its goal is to build a node-based editor with powerful real-time data visualization capabilities. The current focus is on creating a reactive rendering system with custom shaders and high-level Pythonic syntax.

## Installation
```bash
pip install git+https://github.com/PhilippThoelke/guilible.git
```
or for development:
```bash
git clone git@github.com:PhilippThoelke/guilible.git
cd guilible
pip install -e .
```

## Example Usage
This is an unnecssarily complicated example that opens a window, draws a box with gravity and a circle of smaller boxes inside that rotate as the outer box moves.

Controls:
- `left` and `right` arrow keys move the box horizontally
- `space` to move the box up
- `scroll` to change the size of the box

```python
import math
import random

import numpy as np

from guilible import BaseWindow
from guilible.ui import Rectangle

class Window(BaseWindow):
    def setup(self):
        self.background_color = (0.1, 0.1, 0.1)

        self.damp = 0.9
        self.force = np.array([0, -0.01])
        self.vel = np.zeros(2)
        self.rot = 0

        self.box1 = Rectangle(0, 0.2, 0.2, 0.2, (1, 0, 0))
        self.chain = [Rectangle(0, 0, 0.1, 0.1, (0, 1, 0)) for _ in range(10)]
        self.ui.add(self.box1)
        self.box1.add(*self.chain)

    def update(self, delta: float):
        self.wnd.title = f"{1/delta:.2f} FPS"

        self.vel += self.force
        self.box1.x += delta * self.vel[0]
        self.box1.y += delta * self.vel[1]

        if self.box1.y + self.box1.h > 1:
            self.box1.y = 1 - self.box1.h
            self.vel[1] *= -self.damp
            self.box1.color = (random.random(), random.random(), random.random())
        if self.box1.y - self.box1.h < -1:
            self.box1.y = -1 + self.box1.h
            self.vel[1] *= -self.damp
            self.box1.color = (random.random(), random.random(), random.random())
        if self.box1.x + self.box1.w > 1:
            self.box1.x = 1 - self.box1.w
            self.vel[0] *= -self.damp
            self.box1.color = (random.random(), random.random(), random.random())
        if self.box1.x - self.box1.w < -1:
            self.box1.x = -1 + self.box1.w
            self.vel[0] *= -self.damp
            self.box1.color = (random.random(), random.random(), random.random())

        speed = np.linalg.norm(self.vel)
        self.rot += speed**2 * delta * 5 * np.sign(self.vel[0])
        for i, box in enumerate(self.chain):
            box.x = math.sin(self.rot + math.pi * 2 / len(self.chain) * i) * (1 - box.w) * 0.75
            box.y = math.cos(self.rot + math.pi * 2 / len(self.chain) * i) * (1 - box.h) * 0.75
            box.color = (
                1 - self.box1.color[0] + (math.sin(self.time + math.pi * 2 / len(self.chain) * i) / 2 + 0.5),
                1 - self.box1.color[1] + (math.cos(self.time + math.pi * 2 / len(self.chain) * i) / 2 + 0.5),
                1 - self.box1.color[2] + (math.tan(self.time + math.pi * 2 / len(self.chain) * i) / 2 + 0.5),
            )

    def on_mouse_scroll_event(self, x_offset, y_offset):
        self.box1.w += x_offset * 0.1
        self.box1.h += y_offset * 0.1

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

Window.run()
```