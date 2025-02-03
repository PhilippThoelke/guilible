from guilible import BaseWindow


class Window(BaseWindow):
    def setup(self):
        pass

    def update(self, delta: float):
        if self.time > 1:
            self.wnd.close()


def test_create_window():
    Window.run(window_provider="headless")
