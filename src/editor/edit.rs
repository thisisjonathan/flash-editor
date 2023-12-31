use undo::Edit;

use crate::core::{Movie, PlaceSymbol, PlacedSymbolIndex, SymbolIndexOrRoot};

pub enum MovieEdit {
    MovePlacedSymbol(MovePlacedSymbolEdit),
    AddPlacedSymbol(AddPlacedSymbolEdit),
    RemovePlacedSymbol(RemovePlacedSymbolEdit),
}
impl Edit for MovieEdit {
    type Target = Movie;
    type Output = SymbolIndexOrRoot; // the symbol that has changed so the editor can show it

    fn edit(&mut self, target: &mut Movie) -> SymbolIndexOrRoot {
        match self {
            MovieEdit::MovePlacedSymbol(edit) => edit.edit(target),
            MovieEdit::AddPlacedSymbol(edit) => edit.edit(target),
            MovieEdit::RemovePlacedSymbol(edit) => edit.edit(target),
        }
    }

    fn undo(&mut self, target: &mut Movie) -> SymbolIndexOrRoot {
        match self {
            MovieEdit::MovePlacedSymbol(edit) => edit.undo(target),
            MovieEdit::AddPlacedSymbol(edit) => edit.undo(target),
            MovieEdit::RemovePlacedSymbol(edit) => edit.undo(target),
        }
    }
}
pub struct MovePlacedSymbolEdit {
    pub editing_symbol_index: SymbolIndexOrRoot,
    pub placed_symbol_index: PlacedSymbolIndex,

    pub start_x: f64,
    pub start_y: f64,
    pub end_x: f64,
    pub end_y: f64,
}
impl MovePlacedSymbolEdit {
    fn edit(&mut self, target: &mut Movie) -> SymbolIndexOrRoot {
        let placed_symbols = target.get_placed_symbols_mut(self.editing_symbol_index);
        let symbol = &mut placed_symbols[self.placed_symbol_index];
        symbol.x = self.end_x;
        symbol.y = self.end_y;
        self.editing_symbol_index
    }

    fn undo(&mut self, target: &mut Movie) -> SymbolIndexOrRoot {
        let placed_symbols = target.get_placed_symbols_mut(self.editing_symbol_index);
        let symbol = &mut placed_symbols[self.placed_symbol_index];
        symbol.x = self.start_x;
        symbol.y = self.start_y;
        self.editing_symbol_index
    }
}
pub struct AddPlacedSymbolEdit {
    pub editing_symbol_index: SymbolIndexOrRoot,
    pub placed_symbol: PlaceSymbol,
    pub placed_symbol_index: Option<PlacedSymbolIndex>, // for removing when undoing
}
impl AddPlacedSymbolEdit {
    fn edit(&mut self, target: &mut Movie) -> SymbolIndexOrRoot {
        target
            .get_placed_symbols_mut(self.editing_symbol_index)
            .push(self.placed_symbol.clone());
        self.placed_symbol_index =
            Some(target.get_placed_symbols(self.editing_symbol_index).len() - 1);
        self.editing_symbol_index
    }

    fn undo(&mut self, target: &mut Movie) -> SymbolIndexOrRoot {
        let Some(placed_symbol_index) = self.placed_symbol_index else {
            panic!("Undoing AddPlacedSymbolEdit without placed_symbol_index");
        };
        target
            .get_placed_symbols_mut(self.editing_symbol_index)
            .remove(placed_symbol_index);
        self.editing_symbol_index
    }
}
pub struct RemovePlacedSymbolEdit {
    pub editing_symbol_index: SymbolIndexOrRoot,
    pub placed_symbol_index: PlacedSymbolIndex,
    pub placed_symbol: PlaceSymbol, // for adding when undoing
}
impl RemovePlacedSymbolEdit {
    fn edit(&mut self, target: &mut Movie) -> SymbolIndexOrRoot {
        target
            .get_placed_symbols_mut(self.editing_symbol_index)
            .remove(self.placed_symbol_index);
        self.editing_symbol_index
    }

    fn undo(&mut self, target: &mut Movie) -> SymbolIndexOrRoot {
        target
            .get_placed_symbols_mut(self.editing_symbol_index)
            .insert(self.placed_symbol_index, self.placed_symbol.clone());
        self.editing_symbol_index
    }
}
