use pyo3::prelude::*;
use winit::event_loop::{ControlFlow, EventLoop};

mod construct;
mod render;
mod ui;
mod utils;
mod window;

#[pymodule]
mod guilible {
    use super::*;

    #[pyclass]
    struct Window {
        pub base: ui::UIElement,
    }

    #[pymethods]
    impl Window {
        #[new]
        fn new() -> Self {
            Window {
                base: ui::UIElement::base(),
            }
        }

        fn start(&mut self) {
            let event_loop = EventLoop::new().unwrap();
            event_loop.set_control_flow(ControlFlow::Wait);
            let _ = event_loop.run_app(&mut window::Application::default());
        }
    }
}
