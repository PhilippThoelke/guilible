import logging
import weakref
from abc import ABC, abstractmethod
from typing import Tuple

import moderngl_window as mglw

from guilible.ui import Rectangle
from guilible.ui.base import RenderComponentRegistry


class BaseWindow(mglw.WindowConfig, ABC):
    """
    Base class for creating a window with moderngl_window and initializing the moderngl context
    """

    title = "guilible"
    aspect_ratio = None

    def __init__(self, *args, **kwargs):
        if not hasattr(self, "ctx") and "ctx" not in kwargs:
            raise ValueError("For basic usage, run `BaseWindow.run()` instead of instantiating the class directly")
        super().__init__(*args, **kwargs)

        # set up render component registry
        self.ctx.extra = RenderComponentRegistry()

        self._time = None

        self.ui = Rectangle(0, 0, 1, 1, (0, 0, 0))
        self.ui.ctx = self.ctx
        self.ctx.extra.register(self.ui)

    @abstractmethod
    def setup(self) -> None:
        """
        Initialize all the resources and components here
        """
        pass

    @abstractmethod
    def update(self, delta: float) -> None:
        """
        Update the window state here
        """

    @property
    def time(self):
        return self._time

    @property
    def background_color(self):
        return self.ui.color

    @background_color.setter
    def background_color(self, color: Tuple[float, float, float]):
        self.ui.color = color

    def on_render(self, time, frame_time):
        self._time = time
        self._delta_time = frame_time
        self.update(frame_time)
        self.ctx.extra.render()

    @classmethod
    def run(config_cls: mglw.WindowConfig, window_provider: str = "glfw", log_level: int = logging.DEBUG) -> mglw.WindowConfig:
        """
        Initialize the window and the configuration class and run the main loop

        Parameters
        ----------
        config_cls : mglw.WindowConfig
            The configuration class to use
        window_provider : str
            The window provider to use (run `mglw.find_window_classes()` to see available providers)
        log_level : int
            Set moderngl_window logging level

        Returns
        -------
        mglw.WindowConfig
            The configuration instance
        """
        # make sure we're not the base class
        if config_cls == BaseWindow:
            raise ValueError("BaseWindow is abstract. Make sure to call run() on a subclass")

        # setup mglw logging
        mglw.setup_basic_logging(log_level)

        # start the window
        window_cls = mglw.get_local_window_cls(window_provider)
        window = window_cls(
            title=config_cls.title,
            size=config_cls.window_size,
            fullscreen=config_cls.fullscreen,
            resizable=config_cls.resizable,
            visible=config_cls.visible,
            gl_version=config_cls.gl_version,
            aspect_ratio=config_cls.aspect_ratio,
            vsync=config_cls.vsync,
            samples=config_cls.samples,
            cursor=config_cls.cursor,
            context_creation_func=config_cls.init_mgl_context,
        )
        window.print_context_info()
        mglw.activate_context(window)

        config = config_cls(ctx=window.ctx, wnd=window, timer=mglw.Timer())
        # avoid circular reference between window and config
        window._config = weakref.ref(config)

        # swap buffer before main loop to update buffer size
        window.swap_buffers()
        window.set_default_viewport()

        # run the setup function
        config.setup()

        # run the main loop
        mglw.run_window_config_instance(config)


__all__ = ["BaseWindow"]
