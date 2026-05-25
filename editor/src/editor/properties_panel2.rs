use flits_core::{
    BitmapProperties, EditorColor, FlitsFont, MovieClipProperties, MovieProperties, PlaceSymbol,
    PreloaderType, SymbolIndexOrRoot, TextAlign,
};

use crate::{
    editor::Context,
    edits::{MovieAction, MovieChange, PlacedSymbolChange},
    message::EditorMessage,
    undo::EditMessage,
};

#[derive(Default)]
pub struct PropertiesPanel2 {}
impl PropertiesPanel2 {
    pub fn do_ui(&mut self, ui: &mut egui::Ui, ctx: &Context) {
        match ctx.selection.placed_symbols.len() {
            0 => match ctx.selection.properties_symbol_index {
                None => self.show_panel(
                    ui,
                    ctx,
                    ctx.movie.properties.clone(),
                    ctx.selection.properties_symbol_index,
                    |model| EditMessage::Change(MovieChange::MovieProperties(model)),
                    (),
                ),
                Some(properties_symbol_index) => {
                    match &ctx.movie.symbols[properties_symbol_index] {
                        flits_core::Symbol::Bitmap(bitmap) => self.show_panel(
                            ui,
                            ctx,
                            bitmap.properties.clone(),
                            ctx.selection.properties_symbol_index,
                            |model| {
                                EditMessage::Change(MovieChange::BitmapProperties(
                                    properties_symbol_index,
                                    model,
                                ))
                            },
                            BitmapPropertiesAdditionalInfo {
                                error: match &bitmap.cache {
                                    flits_core::BitmapCacheStatus::Invalid(error) => {
                                        Some(error.clone())
                                    }
                                    _ => None,
                                },
                            },
                        ),
                        flits_core::Symbol::MovieClip(movie_clip) => self.show_panel(
                            ui,
                            ctx,
                            movie_clip.properties.clone(),
                            ctx.selection.properties_symbol_index,
                            |model| {
                                EditMessage::Change(MovieChange::MovieClipProperties(
                                    properties_symbol_index,
                                    model,
                                ))
                            },
                            (),
                        ),
                        flits_core::Symbol::Font(flits_font) => self.show_panel(
                            ui,
                            ctx,
                            flits_font.clone(),
                            ctx.selection.properties_symbol_index,
                            |model| {
                                EditMessage::Change(MovieChange::FontProperties(
                                    properties_symbol_index,
                                    model,
                                ))
                            },
                            (),
                        ),
                    }
                }
            },
            1 => self.show_panel(
                ui,
                ctx,
                ctx.movie
                    .get_placed_symbols(ctx.selection.properties_symbol_index)
                    [ctx.selection.placed_symbols[0]]
                    .clone(),
                None,
                |model| {
                    EditMessage::Change(MovieChange::PlacedSymbols(vec![PlacedSymbolChange {
                        editing_symbol_index: ctx.selection.properties_symbol_index,
                        placed_symbol_index: ctx.selection.placed_symbols[0],
                        placed_symbol: model,
                    }]))
                },
                (),
            ),
            _ => {
                ui.label("Multiple items selected");
                return;
            }
        }
    }
    fn show_panel<T: PanelType<I>, I>(
        &mut self,
        ui: &mut egui::Ui,
        ctx: &Context,
        model: T,
        symbol_index: SymbolIndexOrRoot,
        edit_message: impl FnOnce(T) -> EditMessage<MovieChange, MovieAction>,
        additional_info: I,
    ) {
        ui.horizontal(|ui| {
            ui.heading(&model.name());
            if model.has_context_menu() {
                ui.with_layout(
                    egui::Layout::default().with_cross_align(egui::Align::RIGHT),
                    |ui| {
                        egui::menu::menu_button(ui, "...", |ui| {
                            model.context_menu(symbol_index, ui, ctx);
                        });
                    },
                );
            }
        });

        let blocks = model.property_blocks(additional_info);
        let panel_name = model.name();

        let mut model_clone = model;
        let mut commit_needed = false;
        let mut propery_changed = false;

        for (block_index, block) in blocks.iter().enumerate() {
            let (needs_change, needs_commit) =
                block.do_ui(ui, &mut model_clone, block_index, &panel_name);
            if needs_change {
                propery_changed = true;
            }
            if needs_commit {
                commit_needed = true;
            }
        }

        if propery_changed {
            // only send change message when the properties actually changed
            ctx.message_bus
                .publish(EditorMessage::NewEdit((edit_message)(model_clone)));
        }
        if commit_needed {
            ctx.message_bus
                .publish(EditorMessage::NewEdit(EditMessage::Commit));
        }
    }
}

impl EnumProperty for PreloaderType {
    fn enumerate() -> Vec<Self> {
        vec![
            PreloaderType::None,
            PreloaderType::StartAfterLoading,
            PreloaderType::WithPlayButton,
        ]
    }
}
impl EnumProperty for TextAlign {
    fn enumerate() -> Vec<Self> {
        vec![
            TextAlign::Left,
            TextAlign::Right,
            TextAlign::Center,
            TextAlign::Justify,
        ]
    }
}

macro_rules! property {
    ($name:literal, $model: ident, $type:expr ) => {
        Box::new(Property {
            name: $name.into(),
            get: |$model: &Self| $type.clone(),
            set: |$model: &mut Self, value| $type = value,
            settings: None,
            invalid: false,
        })
    };
}
/// A property that's inside an option. Only create the property when the option is Some.
macro_rules! property_option {
    ($name:literal, $model: ident, $option:expr, $inner_model: ident, $type:expr ) => {
        Box::new(Property {
            name: $name.into(),
            get: |$model: &Self| {
                let $inner_model = $option.as_ref().unwrap();
                $type.clone()
            },
            set: |$model: &mut Self, value| {
                let $inner_model = $option.as_mut().unwrap();
                $type = value;
            },
            settings: None,
            invalid: false,
        })
    };
}

type PropertyBox<T> = Box<dyn PropertyTrait<T>>;
enum GridDirectorion {
    Horizontal,
    Vertical,
}
struct Block<T: ?Sized> {
    properties: Vec<PropertyBox<T>>,
    heading: Option<String>,
    grid_direction: GridDirectorion,
    // the condition needs to be evaluated when showing the ui because
    // it might change depending on an earlier property.
    condition: Option<fn(model: &T) -> bool>,
    error: Option<String>,
}
impl<T> Block<T> {
    fn new(properties: Vec<PropertyBox<T>>) -> Self {
        Self {
            properties,
            heading: None,
            grid_direction: GridDirectorion::Horizontal,
            condition: None,
            error: None,
        }
    }
    fn do_ui(
        &self,
        ui: &mut egui::Ui,
        model: &mut T,
        index: usize,
        // to let egui differentiate different panels
        panel_heading: &str,
    ) -> (bool, bool) {
        if let Some(condition) = self.condition {
            if !condition(model) {
                return (false, false);
            }
        }

        if let Some(heading) = &self.heading {
            ui.heading(heading);
        }

        struct PropertyContext<T> {
            model_clone: T,
            commit_needed: bool,
            propery_changed: bool,
        }
        let mut context = PropertyContext {
            model_clone: model,
            commit_needed: false,
            propery_changed: false,
        };
        let iterator = self.properties.iter().map(|property| {
            |ui: &mut egui::Ui, context: &mut PropertyContext<&mut T>| {
                let (needs_change, needs_commit) = property.do_ui(ui, &mut context.model_clone);
                if needs_change {
                    context.propery_changed = true;
                }
                if needs_commit {
                    context.commit_needed = true;
                }
            }
        });

        // TODO: more accurate width estimate
        let padding = 10.0;
        let properties_width = self
            .properties
            .iter()
            .fold(0.0, |acc, p| acc + p.width() + padding);
        if ui.available_width() > properties_width {
            horizontal_layout(ui, &mut context, iterator);
        } else {
            // TODO: change amount of rows based on available width
            match self.grid_direction {
                GridDirectorion::Horizontal => {
                    grid_layout(ui, &mut context, iterator, index, &panel_heading)
                }
                GridDirectorion::Vertical => {
                    vertical_grid_layout(ui, &mut context, iterator, index, &panel_heading)
                }
            }
        }

        if let Some(error) = &self.error {
            ui.colored_label(ui.style().visuals.error_fg_color, error);
        }

        (context.propery_changed, context.commit_needed)
    }
    fn with_heading(mut self, heading: String) -> Self {
        self.heading = Some(heading);
        self
    }
    fn with_vertial_direction(mut self) -> Self {
        self.grid_direction = GridDirectorion::Vertical;
        self
    }
    fn with_condition(mut self, condition: fn(model: &T) -> bool) -> Self {
        self.condition = Some(condition);
        self
    }
    fn with_error(mut self, error: Option<String>) -> Self {
        self.error = error;
        self
    }
}

trait PanelType<I> {
    fn name(&self) -> String;
    fn property_blocks(&self, additional_info: I) -> Vec<Block<Self>>;
    fn has_context_menu(&self) -> bool {
        false
    }
    fn context_menu(&self, _symbol_index: SymbolIndexOrRoot, _ui: &mut egui::Ui, _ctx: &Context) {}
}
fn symbol_context_menu(symbol_index: SymbolIndexOrRoot, ui: &mut egui::Ui, ctx: &Context) {
    let Some(symbol_index) = symbol_index else {
        panic!("Context menu doesn't work for root");
    };
    if ui.button("Delete").clicked() {
        ctx.message_bus
            .publish(EditorMessage::NewEdit(EditMessage::Action(
                MovieAction::remove_movieclip(ctx.movie, symbol_index),
            )));
        ui.close_menu();
    }
}

impl PanelType<()> for MovieProperties {
    fn name(&self) -> String {
        "Movie properties".into()
    }

    fn property_blocks(&self, _additional_info: ()) -> Vec<Block<Self>> {
        vec![Block::new(vec![
            property!("Width", model, model.width),
            property!("Height", model, model.height),
            property!("Framerate", model, model.frame_rate),
            // if i remember correctly, the spec specifies this as rgb. the alpha is ignored (TODO: check)
            property!("Background color", model, model.background_color),
            property!("Preloader", model, model.preloader),
        ])
        .with_vertial_direction()]
    }
}

struct BitmapPropertiesAdditionalInfo {
    error: Option<String>,
}
impl PanelType<BitmapPropertiesAdditionalInfo> for BitmapProperties {
    fn name(&self) -> String {
        "Bitmap properties".into()
    }

    fn property_blocks(&self, additional_info: BitmapPropertiesAdditionalInfo) -> Vec<Block<Self>> {
        vec![
            Block::new(vec![
                property!("Name", model, model.name),
                property!("Path", model, model.path).with_invalid(additional_info.error.is_some()),
            ])
            .with_error(additional_info.error),
            Block::new(vec![property!("Animated", model, model.animation)]),
            Block::new(vec![
                // TODO: slower drag speed?
                property_option!(
                    "Frames",
                    model,
                    model.animation,
                    inner_model,
                    inner_model.frame_count
                )
                .with_settings(NumericPropertySettings { minimum: Some(1) }),
                property_option!(
                    "Frames delay after each frame",
                    model,
                    model.animation,
                    inner_model,
                    inner_model.frame_delay
                ),
            ])
            .with_condition(|model| model.animation.is_some()),
            // separate block because the label is long and lining the other items up to that looks bad
            Block::new(vec![property_option!(
                "On last frame call (e.g. 'stop' or 'removeMovieClip')",
                model,
                model.animation,
                inner_model,
                inner_model.end_action
            )])
            .with_condition(|model| model.animation.is_some()),
        ]
    }

    fn has_context_menu(&self) -> bool {
        true
    }
    fn context_menu(&self, symbol_index: SymbolIndexOrRoot, ui: &mut egui::Ui, ctx: &Context) {
        symbol_context_menu(symbol_index, ui, ctx);
    }
}

impl PanelType<()> for MovieClipProperties {
    fn name(&self) -> String {
        "MovieClip properties".into()
    }

    fn property_blocks(&self, _additional_info: ()) -> Vec<Block<Self>> {
        vec![Block::new(vec![
            property!("Name", model, model.name),
            property!("Class", model, model.class_name),
        ])]
    }

    fn has_context_menu(&self) -> bool {
        true
    }
    fn context_menu(&self, symbol_index: SymbolIndexOrRoot, ui: &mut egui::Ui, ctx: &Context) {
        symbol_context_menu(symbol_index, ui, ctx);
    }
}

impl PanelType<()> for FlitsFont {
    fn name(&self) -> String {
        "Font properties".into()
    }

    fn property_blocks(&self, _additional_info: ()) -> Vec<Block<Self>> {
        vec![Block::new(vec![
            property!("Path", model, model.path),
            // TODO: unchecking this doesn't seem to work for some fonts?
            property!("ASCII characters", model, model.characters.ascii),
            property!(
                "Additional characters",
                model,
                model.characters.additional_characters
            ),
        ])]
    }

    fn has_context_menu(&self) -> bool {
        true
    }
    fn context_menu(&self, symbol_index: SymbolIndexOrRoot, ui: &mut egui::Ui, ctx: &Context) {
        symbol_context_menu(symbol_index, ui, ctx);
    }
}

impl PanelType<()> for PlaceSymbol {
    fn name(&self) -> String {
        "Placed symbol properties".into()
    }

    fn property_blocks(&self, _additional_info: ()) -> Vec<Block<Self>> {
        let mut blocks: Vec<Block<Self>> = vec![Block::new(vec![
            property!("x", model, model.transform.x),
            property!("y", model, model.transform.y),
            property!("X scale", model, model.transform.x_scale),
            property!("Y scale", model, model.transform.y_scale),
            property!("Instance name", model, model.instance_name),
        ])
        .with_vertial_direction()];
        if self.text.is_some() {
            blocks.push(
                Block::new(vec![
                    property_option!("Width", model, model.text, inner_model, inner_model.width),
                    property_option!("Height", model, model.text, inner_model, inner_model.height),
                    property_option!("Size", model, model.text, inner_model, inner_model.size),
                    property_option!("Color", model, model.text, inner_model, inner_model.color),
                    property_option!("Align", model, model.text, inner_model, inner_model.align),
                ])
                .with_vertial_direction()
                .with_heading("Text properties".into()),
            );
            blocks.push(Block::new(vec![
                // TODO: text field that are editable but not selectable are jank, maybe don't allow that combination?
                property_option!("Editable", model, model.text, im, im.editable),
                property_option!("Selectable", model, model.text, im, im.selectable),
                property_option!("Password", model, model.text, im, im.is_password),
                property_option!("HTML", model, model.text, im, im.is_html),
                property_option!("Multiline", model, model.text, im, im.is_multiline),
                property_option!("Word wrap", model, model.text, im, im.word_wrap),
            ]));
            blocks.push(Block::new(vec![property_option!(
                "Text",
                model,
                model.text,
                inner_model,
                inner_model.text
            )
            .with_settings(StringPropertySettings {
                multiline: self.text.as_ref().unwrap().is_multiline,
            })]));
        }

        blocks
    }
}

fn grid_layout<T>(
    ui: &mut egui::Ui,
    context: &mut T,
    iterator: impl ExactSizeIterator<Item = impl FnOnce(&mut egui::Ui, &mut T)>,
    index: usize,
    heading: &str,
) {
    egui::Grid::new(format!(
        "{}_properties_horizontal_grid_layout_{}",
        heading, index
    ))
    .min_col_width(10.0)
    .show(ui, |ui| {
        let length = iterator.len();
        for (index, callback) in iterator.enumerate() {
            (callback)(ui, context);
            if index == length / 2 {
                ui.end_row();
            }
        }
    });
}
/// A grid but vertical to let labels line up nicer
fn vertical_grid_layout<T>(
    ui: &mut egui::Ui,
    context: &mut T,
    iterator: impl ExactSizeIterator<Item = impl FnOnce(&mut egui::Ui, &mut T)>,
    index: usize,
    panel_type: &str,
) {
    let mut iterator = iterator.peekable();
    egui::Grid::new(format!(
        "{}_properties_vertical_grid_layout_{}",
        panel_type, index
    ))
    .show(ui, |ui| {
        let mut column_index = 0;
        loop {
            egui::Grid::new(format!(
                "{}_properties_vertical_grid_layout_{}_inner_{}",
                panel_type, index, column_index
            ))
            .min_col_width(10.0)
            .show(ui, |ui| {
                for _ in 0..2 {
                    if let Some(callback) = iterator.next() {
                        (callback)(ui, context);
                        ui.end_row();
                    }
                }
            });
            column_index += 1;
            if iterator.peek().is_none() {
                break;
            }
        }
    });
}
fn horizontal_layout<T>(
    ui: &mut egui::Ui,
    context: &mut T,
    iterator: impl ExactSizeIterator<Item = impl FnOnce(&mut egui::Ui, &mut T)>,
) {
    ui.horizontal(|ui| {
        for callback in iterator {
            (callback)(ui, context);
            ui.add_space(5.0);
        }
    });
}

struct Property<Model, ValueType, Settings = ()> {
    name: String,
    get: fn(model: &Model) -> ValueType,
    set: fn(model: &mut Model, value: ValueType),
    settings: Option<Settings>,
    invalid: bool,
}
impl<Model, ValueType, Settings> Property<Model, ValueType, Settings> {
    fn with_settings(mut self, settings: Settings) -> Box<Self> {
        self.settings = Some(settings);
        Box::new(self)
    }
    fn with_invalid(mut self, invalid: bool) -> Box<Self> {
        self.invalid = invalid;
        Box::new(self)
    }
}
impl<Model, ValueType, Settings> LabelWidth for Property<Model, ValueType, Settings> {
    fn label_width(&self) -> f32 {
        self.name.len() as f32 * 7.3
    }
}

trait LabelWidth {
    fn label_width(&self) -> f32;
}
trait PropertyTrait<Model>: LabelWidth {
    fn do_ui(&self, ui: &mut egui::Ui, model: &mut Model) -> (bool, bool);
    fn width(&self) -> f32 {
        self.label_width() + self.control_width()
    }
    fn control_width(&self) -> f32 {
        50.0
    }
}

struct NumericPropertySettings<T> {
    minimum: Option<T>,
}
// we need to implement these all seperately instead of using a trait to make prove
// the implementation doesn't overlap with EnumProperty
macro_rules! impl_numeric_properties {
    (for $($t:ty),+) => {
        $(
            impl<Model> PropertyTrait<Model> for Property<Model, $t, NumericPropertySettings<$t>> {
                fn do_ui(&self, ui: &mut egui::Ui, model: &mut Model) -> (bool, bool) {
                    ui.label(format!("{}:", self.name));
                    let mut value = (self.get)(&model);
                    let mut drag_value = egui::DragValue::new(&mut value);

                    if let Some(settings) = self.settings.as_ref() {
                        if let Some(minimum) = settings.minimum {
                            let max = <$t>::MAX;
                            drag_value = drag_value.range(minimum..=max);
                        }
                    }

                    let response = ui.add(drag_value);
                    if response.changed() {
                        (self.set)(model, value);
                    }
                    (
                        response.changed(),
                        response.lost_focus() || response.drag_stopped(),
                    )
                }
            }
        )*
    }
}
impl_numeric_properties!(for f32, f64, u32);

struct StringPropertySettings {
    multiline: bool,
}
impl<Model> PropertyTrait<Model> for Property<Model, String, StringPropertySettings> {
    fn do_ui(&self, ui: &mut egui::Ui, model: &mut Model) -> (bool, bool) {
        ui.label(format!("{}:", self.name));
        let mut value = (self.get)(&model);
        let mut text_edit = if self
            .settings
            .as_ref()
            .is_some_and(|settings| settings.multiline)
        {
            egui::TextEdit::multiline(&mut value).min_size(egui::Vec2::new(200.0, 0.0))
        } else {
            egui::TextEdit::singleline(&mut value).min_size(egui::Vec2::new(200.0, 0.0))
        };
        if self.invalid {
            text_edit = text_edit.text_color(ui.style().visuals.error_fg_color);
        }
        let response = ui.add(text_edit);
        if response.changed() {
            (self.set)(model, value);
        }
        (response.changed(), response.lost_focus())
    }
    fn control_width(&self) -> f32 {
        300.0
    }
}

impl<Model> PropertyTrait<Model> for Property<Model, bool> {
    fn do_ui(&self, ui: &mut egui::Ui, model: &mut Model) -> (bool, bool) {
        let mut value = (self.get)(&model);
        let response = ui.checkbox(&mut value, &self.name);
        if response.changed() {
            (self.set)(model, value);
        }
        (response.changed(), response.changed())
    }
    fn control_width(&self) -> f32 {
        20.0
    }
}

impl<Model, T: Default> PropertyTrait<Model> for Property<Model, Option<T>> {
    fn do_ui(&self, ui: &mut egui::Ui, model: &mut Model) -> (bool, bool) {
        let mut value = (self.get)(&model).is_some();
        let response = ui.checkbox(&mut value, &self.name);
        if response.changed() {
            if value {
                (self.set)(model, Some(T::default()));
            } else {
                (self.set)(model, None);
            }
        }
        (response.changed(), response.changed())
    }
}

trait EnumProperty: ToString + PartialEq + Clone
where
    Self: Sized,
{
    fn enumerate() -> Vec<Self>;
}
impl<Model, ValueType> PropertyTrait<Model> for Property<Model, ValueType>
where
    ValueType: EnumProperty,
{
    fn do_ui(&self, ui: &mut egui::Ui, model: &mut Model) -> (bool, bool) {
        ui.label(format!("{}:", self.name));
        let value_before = (self.get)(&model);
        let mut value = value_before.clone();
        egui::ComboBox::from_id_salt(&self.name)
            .selected_text(format!("{:}", value.to_string()))
            .show_ui(ui, |ui| {
                for variant in ValueType::enumerate() {
                    ui.selectable_value(&mut value, variant.clone(), variant.to_string());
                }
            });
        let changed = value != value_before;
        if changed {
            (self.set)(model, value);
        }
        (changed, changed)
    }
    fn control_width(&self) -> f32 {
        100.0
    }
}

impl<Model> PropertyTrait<Model> for Property<Model, EditorColor> {
    fn do_ui(&self, ui: &mut egui::Ui, model: &mut Model) -> (bool, bool) {
        ui.label(format!("{}:", self.name));

        let original_value = (self.get)(&model);
        let mut value = original_value.clone();

        let mut color = egui::Color32::from_rgba_unmultiplied(value.r, value.g, value.b, value.a);
        let response = egui::color_picker::color_edit_button_srgba(
            ui,
            &mut color,
            // the alpha doesn't do anything for all places we currently use the color picker
            egui::color_picker::Alpha::Opaque,
        );
        let color_data = color.to_srgba_unmultiplied();
        value.r = color_data[0];
        value.g = color_data[1];
        value.b = color_data[2];
        value.a = color_data[3];

        let changed = value != original_value;
        if changed {
            (self.set)(model, value);
        }

        // response.clicked_elsewhere() is true even when you don't have the color picker selected
        // and you click anywhere in the program
        // TODO: this causes unnecessary commits, but a commit without changes is a noop so it might not be a problem?
        // response.clicked_elsewhere() is false when you press escape, we need to handle that seperately
        // might be fixed after the popup refactor in egui: https://github.com/emilk/egui/issues/5189
        (
            changed,
            ui.input(|i| i.key_pressed(egui::Key::Escape)) || response.clicked_elsewhere(),
        )
    }
}
