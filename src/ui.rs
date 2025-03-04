use std::sync::Arc;
use std::vec;

use pyo3::{pyclass, Py, PyAny, Python};
use winit::event;

#[derive(Clone)]
#[pyclass]
pub struct UIElement {
    children: Vec<Arc<UIElement>>,
    event_callback: Option<Arc<Py<PyAny>>>,
}

impl UIElement {
    pub fn base() -> Self {
        UIElement {
            children: vec![],
            event_callback: None,
        }
    }

    pub fn handle_event(&self, event: event::WindowEvent) -> bool {
        let callback_arc = match &self.event_callback {
            Some(cb) => cb.clone(),
            None => return true,
        };

        match event {
            event::WindowEvent::KeyboardInput {
                device_id,
                event,
                is_synthetic,
            } => {
                return Python::with_gil(|py| match callback_arc.call1(py, (KeyboardEvent {},)) {
                    Ok(val) => return val.is_truthy(py).unwrap(),
                    Err(e) => {
                        e.print_and_set_sys_last_vars(py);
                        false
                    }
                });
            }
            _ => return false,
        }
    }

    pub fn set_callback(&mut self, callback: Py<PyAny>) {
        self.event_callback = Some(Arc::new(callback));
    }
}

#[pyclass]
struct KeyboardEvent {}

// Dummy struct to avoid unused warnings.
pub struct UIState {}
