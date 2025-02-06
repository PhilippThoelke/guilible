import weakref
from abc import ABC, abstractmethod
from typing import List, Optional, Tuple, Type

import moderngl as mgl
import numpy as np
from moderngl import Context


class UIElement(ABC):

    VERTICES: List[float]
    VERTEX_SHADER: str
    FRAGMENT_SHADER: str

    def __init__(self, x: float, y: float, w: float, h: float):
        super().__init__()
        self._ctx = None
        self._parent = None
        self._children = []

        if not hasattr(self, "VERTICES"):
            raise AttributeError(f"Missing VERTICES in class {self.__class__.__name__}")
        if not hasattr(self, "VERTEX_SHADER"):
            raise AttributeError(f"Missing VERTEX_SHADER in class {self.__class__.__name__}")
        if not hasattr(self, "FRAGMENT_SHADER"):
            raise AttributeError(f"Missing FRAGMENT_SHADER in class {self.__class__.__name__}")

        self.x = x
        self.y = y
        self.w = w
        self.h = h

    @property
    @abstractmethod
    def params(self) -> Tuple[float, ...]:
        pass

    @property
    def parent(self) -> Optional["UIElement"]:
        return None if self._parent is None else self._parent()

    @property
    def children(self) -> List["UIElement"]:
        return self._children

    def transformed_params(self) -> Tuple[float, ...]:
        if self.parent is None:
            return self.params

        parent = self.parent.transformed_params()
        new_params = list(self.params)
        new_params[0] = new_params[0] * parent[2] + parent[0]
        new_params[1] = new_params[1] * parent[3] + parent[1]
        new_params[2] *= parent[2]
        new_params[3] *= parent[3]
        return tuple(new_params)

    @property
    def ctx(self):
        return self._ctx

    @ctx.setter
    def ctx(self, ctx: Context):
        if self._ctx is not None:
            raise RuntimeError("Can't change context while the element is active")
        self._ctx = ctx

    @property
    def ui_class(self):
        """The main class implementing the UIElement"""
        # get the UIElement's immediate descendant (object -> ABC -> UIElement -> cls)
        return self.__class__.mro()[-4]

    def add(self, *elements: "UIElement"):
        if self.ctx is None:
            raise RuntimeError("Element must be part of a context before adding children")
        for element in elements:
            if element.ctx is not None:
                raise RuntimeError("Element already has a context, and can't be added to another element")

            element.ctx = self.ctx

            self._children.append(element)
            element._parent = weakref.ref(self)
            self.ctx.extra.register(element)

    def remove(self, element: "UIElement"):
        self._children.remove(element)
        element._parent = None
        element.ctx = None
        self.ctx.extra.unregister(element)


class RenderComponent:
    INITIAL_BUFFER_SIZE = 1024
    SIZE_MULTIPLIER = 2

    def __init__(self, ctx: Context, comp_cls: Type[UIElement]):
        self.ctx = ctx
        self.comp_cls = comp_cls
        self.elements = []

        # create the shader program
        self.program = self.ctx.program(vertex_shader=comp_cls.VERTEX_SHADER, fragment_shader=comp_cls.FRAGMENT_SHADER)

        # parse program attributes
        attrs = [None] * len(self.program._members)
        for key in self.program:
            attrs[self.program._attribute_locations[key]] = (key, self.program[key].dimension * self.program[key].array_length)
        self.instance_size = sum([attr[1] for attr in attrs[1:]])

        # create the instance buffer
        self.buffer = self.ctx.buffer(reserve=RenderComponent.INITIAL_BUFFER_SIZE, dynamic=True)
        self.vao = self.ctx.vertex_array(
            self.program,
            [
                (self.ctx.buffer(np.array(self.comp_cls.VERTICES).astype("f4")), f"{attrs[0][1]}f /v", attrs[0][0]),
                (self.buffer, " ".join([str(a[1]) + "f" for a in attrs[1:]]) + " /i", *[a[0] for a in attrs[1:]]),
            ],
        )

        self.array_buffer = None

    def add(self, element: UIElement):
        # add the element to the list of elements
        self.elements.append(element)

        # recreate the array buffer
        self.array_buffer = np.zeros(len(self.elements) * self.instance_size, dtype=np.float32)

        if self.buffer.size < len(self.elements) * self.instance_size * 4:
            # increase the buffer size by a factor of SIZE_MULTIPLIER
            self.buffer.orphan(self.buffer.size * RenderComponent.SIZE_MULTIPLIER)

    def remove(self, element: UIElement):
        # remove the element from the list of elements
        self.elements.remove(element)

        # recreate the array buffer
        self.array_buffer = np.zeros(len(self.elements) * self.instance_size, dtype=np.float32)

        if self.buffer.size > len(self.elements) * self.instance_size * 4 * RenderComponent.SIZE_MULTIPLIER:
            # decrease the buffer size by a factor of SIZE_MULTIPLIER
            self.buffer.orphan(self.buffer.size // RenderComponent.SIZE_MULTIPLIER)

    def render(self):
        for i, element in enumerate(self.elements):
            self.array_buffer[i * self.instance_size : (i + 1) * self.instance_size] = element.transformed_params()

        self.buffer.write(self.array_buffer)
        self.vao.render(mode=mgl.TRIANGLES, vertices=len(self.comp_cls.VERTICES), instances=len(self.elements))


class RenderComponentRegistry:
    def __init__(self):
        self.registry = {}

    def register(self, element: UIElement):
        cls = element.ui_class
        if cls not in self.registry:
            self.registry[cls] = RenderComponent(element.ctx, cls)
        self.registry[cls].add(element)

    def unregister(self, element: UIElement):
        cls = element.ui_class
        self.registry[cls].remove(element)

    def get(self, name):
        return self.registry[name]

    def render(self):
        for comp in self.registry.values():
            comp.render()
