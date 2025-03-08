use pyo3::prelude::*;

use crate::guilible;

#[pyclass]
pub struct KeyboardEvent {
    #[pyo3(get)]
    pub key: guilible::Keys,
    #[pyo3(get)]
    pub state: KeyState,
}

#[pyclass(eq, eq_int)]
#[derive(Clone, Copy, PartialEq, Eq)]
pub enum KeyState {
    Down,
    Up,
}

impl From<winit::event::ElementState> for KeyState {
    fn from(state: winit::event::ElementState) -> Self {
        match state {
            winit::event::ElementState::Pressed => KeyState::Down,
            winit::event::ElementState::Released => KeyState::Up,
        }
    }
}
