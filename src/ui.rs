use std::sync::Arc;
use std::vec;

use pyo3::prelude::*;
use winit::event;

use crate::event::KeyboardEvent;

#[derive(Clone)]
#[pyclass]
pub struct UIElement {
    _children: Vec<Arc<UIElement>>,
    event_callback: Option<Arc<Py<PyAny>>>,
}

impl UIElement {
    pub fn base() -> Self {
        UIElement {
            _children: vec![],
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
                device_id: _,
                event,
                is_synthetic: _,
            } => match event.physical_key {
                winit::keyboard::PhysicalKey::Code(code) => {
                    return Python::with_gil(|py| {
                        match callback_arc.call1(
                            py,
                            (KeyboardEvent {
                                key: code.into(),
                                state: event.state.into(),
                            },),
                        ) {
                            Ok(val) => return val.is_truthy(py).unwrap(),
                            Err(e) => {
                                e.print_and_set_sys_last_vars(py);
                                false
                            }
                        }
                    });
                }
                _ => {
                    println!("Unrecognized key: {:?}", event.physical_key);
                    return false;
                }
            },
            _ => return false,
        }
    }

    pub fn set_callback(&mut self, callback: Py<PyAny>) {
        self.event_callback = Some(Arc::new(callback));
    }
}
