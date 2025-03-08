use pyo3::prelude::*;
use std::sync::{mpsc, Arc};
use std::{sync::Mutex, thread};
use winit::event_loop::{ControlFlow, EventLoop};

mod construct;
mod event;
mod render;
mod ui;
mod utils;
mod window;

#[pymodule]
mod guilible {
    use super::*;

    #[pyclass]
    pub struct Window {
        base: Arc<Mutex<ui::UIElement>>,
    }

    #[pymethods]
    impl Window {
        #[new]
        pub fn new() -> Self {
            Window {
                base: Arc::new(Mutex::new(ui::UIElement::base())),
            }
        }

        fn set_callback(&mut self, callback: Py<PyAny>) {
            self.base.lock().unwrap().set_callback(callback);
        }

        pub fn start(&mut self, py: Python) {
            let (event_sender, event_receiver) = mpsc::channel();

            let base_clone = self.base.clone();
            thread::Builder::new()
                .name("event worker".to_string())
                .spawn(move || loop {
                    match event_receiver.recv() {
                        Ok(event) => {
                            base_clone.lock().unwrap().handle_event(event);
                        }
                        Err(_) => break,
                    }
                })
                .expect("failed to spawn event worker");

            py.allow_threads(move || {
                let event_loop = EventLoop::new().unwrap();
                event_loop.set_control_flow(ControlFlow::Wait);
                let _ = event_loop.run_app(&mut window::Application::new(event_sender));
            });
        }
    }

    #[pymodule_init]
    fn init_mod(m: &Bound<'_, PyModule>) -> PyResult<()> {
        // add Keys class
        m.add_class::<Keys>()
    }

    // codegen for pyclass enum Keys
    include!(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/src/codegen/keycode.rs"
    ));
}
