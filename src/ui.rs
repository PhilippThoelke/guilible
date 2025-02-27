use std::sync::Weak;

pub struct UIElement {
    pub parent: Option<Weak<UIElement>>,
    pub children: Vec<UIElement>,
}

impl UIElement {
    pub fn base() -> Self {
        UIElement {
            parent: None,
            children: vec![],
        }
    }
}
