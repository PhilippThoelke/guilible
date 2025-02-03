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
This is a basic example that draws two rectangles, one being the child of the other. They move together in a circle and the inner box changes color randomly when the mouse is clicked.

```python
import math
import random

from guilible import BaseWindow
from guilible.ui import Rectangle


class Window(BaseWindow):
    window_size = (800, 800)

    def setup(self):
        # add a white box in the center
        self.outer = Rectangle(0, 0, 0.2, 0.2, (1, 1, 1))
        self.ui.add(self.outer)

        # add a smaller colored box inside the white box
        self.inner = Rectangle(0, 0, 0.9, 0.9, (0.5, 0, 1))
        self.outer.add(self.inner)

    def update(self, delta: float):
        # move the box around in a circle
        self.outer.x = math.sin(self.time) * 0.5
        self.outer.y = math.cos(self.time) * 0.5

    def on_mouse_press_event(self, x, y, button):
        # randomize the colorwhen the mouse is clicked
        self.inner.color = (random.random(), random.random(), random.random())


if __name__ == "__main__":
    Window.run()
```
The code above generates the following output:
<p align="center">
    <img src="https://github.com/user-attachments/assets/6fe9e323-0951-4c9a-ac59-8c942d0c6f0d">
</p>
