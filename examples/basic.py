import guilible as gl


def event_callback(event):
    print(event)


if __name__ == "__main__":
    win = gl.Window()
    win.set_callback(event_callback)
    win.start()
