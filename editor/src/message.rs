use flits_core::{SymbolIndex, SymbolIndexOrRoot};

use crate::{edit::MovieEdit, editor::stage::StageMessage, edits::FlitsEditMessage, FlitsEvent};

pub enum EditorMessage {
    Save,
    Export,
    Run,
    OpenNewSymbolWindow,
    ChangeSelectedSymbol(SymbolIndexOrRoot),
    ChangeSelectedPlacedSymbols(Vec<SymbolIndex>),
    SelectAll,
    DeleteSelection,
    ReloadAssets,
    Edit(MovieEdit),
    NewEdit(FlitsEditMessage),
    Stage(StageMessage),
    Event(FlitsEvent),

    ShowUndoDebugUi,
}
