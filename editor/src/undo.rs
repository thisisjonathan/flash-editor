/// There are 2 types of undoable edits:
/// # Changes
/// Changes can be used for previewing an edit before commiting to it (adding an undo point).
/// Changes know how to invert automatically (just go back to the previous value).
/// # Actions
/// Actions are for changes where the inverse needs to be implemented manually, for example adding or removing things.
/// Actions always add a undo point and don't need to be commited.
#[derive(Debug, Clone)]
pub enum EditMessage<Change, Action>
where
    Change: ChangeEdit,
    Action: ActionEdit,
{
    Action(Action),
    Change(Change),
    Commit,
    Undo,
    Redo,
}
#[derive(Debug, Clone)]
enum Edit<Change: ChangeEdit, Action: ActionEdit> {
    Value(ValueEdit<Change>),
    Action(Action),
}
impl<Change: ChangeEdit, Action: ActionEdit<Model = Change::Model, Output = Change::Output>>
    Edit<Change, Action>
{
    fn apply(&self, model: &mut Change::Model) {
        match self {
            Edit::Value(value_edit) => value_edit.after.apply(model),
            Edit::Action(action) => action.apply(model),
        }
    }
    fn invert(self) -> Self {
        match self {
            Edit::Value(value_edit) => Edit::Value(ValueEdit {
                before: value_edit.after,
                after: value_edit.before,
            }),
            Edit::Action(action_edit) => Edit::Action(action_edit.invert()),
        }
    }
    fn output(&self) -> Change::Output {
        match self {
            Edit::Value(value_edit) => value_edit.after.output(),
            Edit::Action(action) => action.output(),
        }
    }
}
pub trait ActionEdit: std::fmt::Debug + Clone {
    type Model;
    type Output;

    fn apply(&self, model: &mut Self::Model);
    fn invert(self) -> Self;
    fn output(&self) -> Self::Output;
}
pub trait ChangeEdit: std::fmt::Debug + Clone {
    type Model;
    type Output;

    fn apply(&self, model: &mut Self::Model);
    fn existing_value(&self, model: &Self::Model) -> Self;
    fn output(&self) -> Self::Output;
}
#[derive(Debug, Clone)]
struct ValueEdit<Change: std::fmt::Debug + Clone> {
    before: Change,
    after: Change,
}

pub struct UndoStack<Change, Action>
where
    Change: ChangeEdit + 'static,
    Action: ActionEdit + 'static,
{
    undo_stack: Vec<Edit<Change, Action>>,
    redo_stack: Vec<Edit<Change, Action>>,
    preview_edit: Option<ValueEdit<Change>>,
}
impl<Change: ChangeEdit, Action: ActionEdit<Model = Change::Model, Output = Change::Output>>
    UndoStack<Change, Action>
{
    pub fn new() -> Self {
        Self {
            undo_stack: Vec::new(),
            redo_stack: Vec::new(),
            preview_edit: None,
        }
    }
    pub fn update(
        &mut self,
        model: &mut Change::Model,
        message: EditMessage<Change, Action>,
    ) -> Option<Action::Output> {
        match message {
            EditMessage::Action(action) => {
                let output = action.output();
                action.apply(model);
                self.undo_stack.push(Edit::Action(action));
                self.redo_stack.clear();
                Some(output)
            }
            EditMessage::Change(change) => {
                if let Some(preview_edit) = &self.preview_edit {
                    if std::mem::discriminant(&change)
                        != std::mem::discriminant(&preview_edit.before)
                    {
                        panic!(
                            "Made change overriding change of a different type. Types: {:?} {:?}",
                            std::mem::discriminant(&change),
                            std::mem::discriminant(&preview_edit.before)
                        );
                    }
                    change.apply(model);
                    self.preview_edit = Some(ValueEdit {
                        before: preview_edit.before.clone(),
                        after: change.clone(),
                    });
                    Some(change.output())
                } else {
                    self.preview_edit = Some(ValueEdit {
                        before: change.existing_value(model),
                        after: change.clone(),
                    });
                    change.apply(model);
                    Some(change.output())
                }
            }
            EditMessage::Commit => {
                if let Some(preview_edit) = self.preview_edit.take() {
                    self.undo_stack.push(Edit::Value(preview_edit));
                    self.redo_stack.clear();
                }
                None
            }
            EditMessage::Undo => {
                let Some(edit) = self.undo_stack.pop() else {
                    return None;
                };
                self.redo_stack.push(edit.clone());
                let inverted = edit.invert();
                inverted.apply(model);
                Some(inverted.output())
            }
            EditMessage::Redo => {
                let Some(edit) = self.redo_stack.pop() else {
                    return None;
                };
                edit.apply(model);
                let output = edit.output();
                self.undo_stack.push(edit);
                Some(output)
            }
        }
    }

    pub fn debug_ui(&self, ui: &mut egui::Ui) {
        egui::ScrollArea::vertical().show(ui, |ui| {
            ui.label(format!("Preview edit: {:#?}", self.preview_edit));
            ui.heading("Undo");
            for edit in &self.undo_stack {
                ui.label(format!("{:#?}", edit));
            }
            ui.heading("Redo");
            for edit in &self.redo_stack {
                ui.label(format!("{:#?}", edit));
            }
        });
    }
}
