use crate::state::State;

pub trait CustomWidget<T>
where
    T: Clone,
{
    fn view(&self, state: &State) -> iced::Element<'_, T>;

    fn update(&mut self, message: T, state: &State) -> iced::Task<T>;

    fn handle_key(
        &mut self,
        _key: iced::keyboard::key::Named,
        _modifiers: iced::keyboard::Modifiers,
    ) -> Option<iced::Task<T>> {
        None
    }
}
