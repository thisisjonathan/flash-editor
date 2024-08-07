use egui::Vec2;

use super::edit::{AddMovieClipEdit, MovieEdit};

#[derive(Default)]
pub struct NewSymbolWindow {
    name: String,
    has_requestion_focus: bool,
}
impl NewSymbolWindow {
    pub fn do_ui(&mut self, egui_ctx: &egui::Context) -> Option<MovieEdit> {
        let mut result = None;
        // title says new movieclip because there are no other options yet
        egui::Window::new("New movieclip")
            .resizable(false)
            .show(egui_ctx, |ui| {
                egui::Grid::new("symbol_properties_grid").show(ui, |ui| {
                    ui.label("Name:");
                    let response = ui.add(
                        egui::TextEdit::singleline(&mut self.name).min_size(Vec2::new(200.0, 0.0)),
                    );
                    if !self.has_requestion_focus {
                        response.request_focus();
                        self.has_requestion_focus = true;
                    }
                    let user_confirmed_form = response.lost_focus()
                        && response.ctx.input(|i| i.key_pressed(egui::Key::Enter));
                    ui.end_row();

                    if ui
                        .add_enabled(!self.name.is_empty(), egui::Button::new("Create"))
                        .clicked()
                        || (user_confirmed_form && !self.name.is_empty())
                    {
                        result = Some(MovieEdit::AddMovieClip(AddMovieClipEdit {
                            name: self.name.clone(),
                        }));
                    }
                    ui.end_row();
                });
            });
        result
    }
}
