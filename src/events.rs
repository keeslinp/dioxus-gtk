use dioxus_core::UiEvent;

pub struct PressData {}

pub type PressEvent = UiEvent<PressData>;

pub struct TextChangeData {
    pub value: String,
}

pub type TextChangeEvent = UiEvent<TextChangeData>;
