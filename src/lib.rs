use pyo3::prelude::*;
use std::sync::{mpsc, Arc};
use std::{sync::Mutex, thread};
use winit::event_loop::{ControlFlow, EventLoop};

mod construct;
mod render;
mod ui;
mod utils;
mod window;

#[pymodule]
pub mod guilible {
    use super::*;

    #[pyclass]
    pub struct Window {
        base: Arc<Mutex<ui::UIElement>>,
        ui_sender: Option<mpsc::SyncSender<ui::UIState>>,
    }

    #[pymethods]
    impl Window {
        #[new]
        pub fn new() -> Self {
            Window {
                base: Arc::new(Mutex::new(ui::UIElement::base())),
                ui_sender: None,
            }
        }

        fn set_callback(&mut self, callback: Py<PyAny>) {
            self.base.lock().unwrap().set_callback(callback);
        }

        pub fn start(&mut self, py: Python) {
            let (ui_sender, ui_receiver) = mpsc::sync_channel(1);
            let (event_sender, event_receiver) = mpsc::channel();

            self.ui_sender = Some(ui_sender);

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
                let _ =
                    event_loop.run_app(&mut window::Application::new(ui_receiver, event_sender));
            });
        }
    }
}
