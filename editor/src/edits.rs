use flits_core::{
    BitmapProperties, FlitsFont, Movie, MovieClip, MovieClipProperties, MovieProperties,
    PlaceSymbol, PlacedSymbolIndex, Symbol, SymbolIndex, SymbolIndexOrRoot,
};

use crate::undo::{ActionEdit, ChangeEdit};

#[derive(Debug, Clone)]
pub enum MovieChange {
    MovieProperties(MovieProperties),
    BitmapProperties(SymbolIndex, BitmapProperties),
    MovieClipProperties(SymbolIndex, MovieClipProperties),
    FontProperties(SymbolIndex, FlitsFont),
    PlacedSymbols(Vec<PlacedSymbolChange>),
}
impl ChangeEdit for MovieChange {
    type Model = Movie;

    fn apply(&self, model: &mut Movie) {
        match self {
            MovieChange::MovieProperties(movie_properties) => {
                model.properties = movie_properties.clone();
            }
            MovieChange::BitmapProperties(symbol_index, bitmap_properties) => {
                let Symbol::Bitmap(bitmap) = &mut model.symbols[*symbol_index] else {
                    panic!("Changed symbol is not a bitmap");
                };
                // reset cache when path changes or when animation settings change
                // (because the same image will be interpreted differently)
                if bitmap.properties.path != bitmap_properties.path
                    || bitmap.properties.animation != bitmap_properties.animation
                {
                    // TODO: reuse file data when animation settings change
                    bitmap.invalidate_cache();
                }
                bitmap.properties = bitmap_properties.clone();
            }
            MovieChange::MovieClipProperties(symbol_index, movieclip_properties) => {
                let Symbol::MovieClip(movieclip) = &mut model.symbols[*symbol_index] else {
                    panic!("Changed symbol is not a movieclip");
                };
                movieclip.properties = movieclip_properties.clone();
            }
            MovieChange::FontProperties(symbol_index, font_properties) => {
                let Symbol::Font(font) = &mut model.symbols[*symbol_index] else {
                    panic!("Changed symbol is not a font");
                };
                // TODO: reload font
                *font = font_properties.clone();
            }
            MovieChange::PlacedSymbols(changes) => {
                for change in changes {
                    let placed_symbols = model.get_placed_symbols_mut(change.editing_symbol_index);
                    let symbol = &mut placed_symbols[change.placed_symbol_index];
                    change.placed_symbol.clone_into(symbol);
                }
            }
        }
    }

    fn existing_value(&self, model: &Movie) -> Self {
        match self {
            MovieChange::MovieProperties(_) => {
                MovieChange::MovieProperties(model.properties.clone())
            }
            MovieChange::BitmapProperties(symbol_index, _) => {
                let Symbol::Bitmap(bitmap) = &model.symbols[*symbol_index] else {
                    panic!("Existing symbol is not a bitmap");
                };
                MovieChange::BitmapProperties(*symbol_index, bitmap.properties.clone())
            }
            MovieChange::MovieClipProperties(symbol_index, _) => {
                let Symbol::MovieClip(movieclip) = &model.symbols[*symbol_index] else {
                    panic!("Existing symbol is not a movieclip");
                };
                MovieChange::MovieClipProperties(*symbol_index, movieclip.properties.clone())
            }
            MovieChange::FontProperties(symbol_index, _) => {
                let Symbol::Font(font) = &model.symbols[*symbol_index] else {
                    panic!("Existing symbol is not a font");
                };
                MovieChange::FontProperties(*symbol_index, font.clone())
            }
            MovieChange::PlacedSymbols(changes) => {
                let mut existing_changes = Vec::with_capacity(changes.len());
                for change in changes {
                    let placed_symbols = model.get_placed_symbols(change.editing_symbol_index);
                    let symbol = &placed_symbols[change.placed_symbol_index];
                    existing_changes.push(PlacedSymbolChange {
                        editing_symbol_index: change.editing_symbol_index,
                        placed_symbol_index: change.placed_symbol_index,
                        placed_symbol: symbol.clone(),
                    });
                }
                MovieChange::PlacedSymbols(existing_changes)
            }
        }
    }
}
#[derive(Debug, Clone)]
pub struct PlacedSymbolChange {
    pub editing_symbol_index: SymbolIndexOrRoot,
    pub placed_symbol_index: PlacedSymbolIndex,

    pub placed_symbol: PlaceSymbol,
}
#[derive(Debug, Clone)]
pub enum MovieAction {
    AddSymbol(SymbolIndex, Symbol, Vec<PlacedSymbolAction>),
    RemoveSymbol(SymbolIndex, Symbol, Vec<PlacedSymbolAction>),

    AddPlacedSymbols(Vec<PlacedSymbolAction>),
    RemovePlacedSymbols(Vec<PlacedSymbolAction>),
}
impl MovieAction {
    pub fn add_movieclip(movie: &Movie, name: String) -> Self {
        Self::AddSymbol(
            movie.symbols.len(),
            Symbol::MovieClip(MovieClip {
                properties: MovieClipProperties {
                    name,
                    class_name: String::new(),
                },
                place_symbols: Vec::new(),
            }),
            Vec::new(),
        )
    }
    pub fn remove_movieclip(movie: &Movie, index: SymbolIndex) -> Self {
        let mut place_symbol_actions = Vec::new();
        Self::remove_placed_symbols_of_symbol(movie, None, index, &mut place_symbol_actions);
        for i in 0..movie.symbols.len() {
            match movie.symbols[i] {
                Symbol::MovieClip(_) => {
                    Self::remove_placed_symbols_of_symbol(
                        movie,
                        Some(i),
                        index,
                        &mut place_symbol_actions,
                    );
                }
                _ => {}
            }
        }

        Self::RemoveSymbol(index, movie.symbols[index].clone(), place_symbol_actions)
    }

    fn remove_placed_symbols_of_symbol(
        movie: &Movie,
        symbol_index_to_check: SymbolIndexOrRoot,
        symbol_index_to_remove: SymbolIndex,
        placed_symbol_actions: &mut Vec<PlacedSymbolAction>,
    ) {
        let placed_symbols = movie.get_placed_symbols(symbol_index_to_check);
        for i in 0..placed_symbols.len() {
            // if the placed symbol is the symbol we are removing
            if placed_symbols[i].symbol_index == symbol_index_to_remove {
                // remove the placed symbol
                placed_symbol_actions.push(PlacedSymbolAction {
                    editing_symbol_index: symbol_index_to_check,
                    placed_symbol_index: i,
                    placed_symbol: placed_symbols[i].clone(),
                });
            }
        }
    }

    pub fn remove_placed_symbol(
        movie: &Movie,
        symbol_index: SymbolIndexOrRoot,
        placed_symbol_index: PlacedSymbolIndex,
    ) -> Self {
        MovieAction::RemovePlacedSymbols(vec![PlacedSymbolAction {
            editing_symbol_index: symbol_index,
            placed_symbol: movie.get_placed_symbols(symbol_index)[placed_symbol_index].clone(),
            placed_symbol_index,
        }])
    }
}
impl ActionEdit for MovieAction {
    type Model = Movie;

    fn apply(&self, model: &mut Movie) {
        match self {
            MovieAction::AddSymbol(index, symbol, placed_symbol_actions) => {
                // increase symbol index of placed symbols with an index equal to or higher that the one we insert
                // to make room for our symbol
                Self::for_each_symbol_and_root(
                    model,
                    *index,
                    |movie: &mut Movie,
                     symbol_index_to_check: SymbolIndexOrRoot,
                     symbol_index_to_add: SymbolIndex| {
                        Self::change_placed_symbol_index_of_placed_symbols_with_higher_symbol_index(
                            movie,
                            symbol_index_to_check,
                            symbol_index_to_add,
                            1,
                        );
                    },
                );

                model.symbols.insert(*index, symbol.clone());

                Self::add_placed_symbols(model, placed_symbol_actions);
            }
            MovieAction::RemoveSymbol(index, _symbol, placed_symbol_actions) => {
                // remove existing placed symbols
                Self::remove_placed_symbols(model, placed_symbol_actions);

                model.symbols.remove(*index);

                // decrease indexes for placed symbols for symbols with a higher index
                Self::for_each_symbol_and_root(
                    model,
                    *index,
                    |movie: &mut Movie,
                     symbol_index_to_check: SymbolIndexOrRoot,
                     symbol_index_to_remove: SymbolIndex| {
                        Self::change_placed_symbol_index_of_placed_symbols_with_higher_symbol_index(
                            movie,
                            symbol_index_to_check,
                            symbol_index_to_remove,
                            // decrease the symbol index because removing the movieclip causes the index of the other moveclips to change
                            -1,
                        );
                    },
                );
            }
            MovieAction::AddPlacedSymbols(actions) => {
                Self::add_placed_symbols(model, actions);
            }
            MovieAction::RemovePlacedSymbols(actions) => {
                Self::remove_placed_symbols(model, actions);
            }
        }
    }

    fn invert(self) -> Self {
        match self {
            MovieAction::AddSymbol(index, symbol, placed_symbol_actions) => {
                MovieAction::RemoveSymbol(index, symbol, placed_symbol_actions)
            }
            MovieAction::RemoveSymbol(index, symbol, placed_symbol_actions) => {
                MovieAction::AddSymbol(index, symbol, placed_symbol_actions)
            }
            MovieAction::AddPlacedSymbols(actions) => MovieAction::RemovePlacedSymbols(actions),
            MovieAction::RemovePlacedSymbols(actions) => MovieAction::AddPlacedSymbols(actions),
        }
    }
}
impl MovieAction {
    fn for_each_symbol_and_root(
        model: &mut Movie,
        symbol_index: SymbolIndex,
        callback: fn(
            model: &mut Movie,
            symbol_index_iteration: SymbolIndexOrRoot,
            symbol_index_argument: SymbolIndex,
        ),
    ) {
        callback(model, None, symbol_index);
        for i in 0..model.symbols.len() {
            match model.symbols[i] {
                Symbol::MovieClip(_) => {
                    callback(model, Some(i), symbol_index);
                }
                _ => {}
            }
        }
    }
    fn change_placed_symbol_index_of_placed_symbols_with_higher_symbol_index(
        movie: &mut Movie,
        symbol_index_to_check: SymbolIndexOrRoot,
        symbol_index_to_compare_with: SymbolIndex,
        change_amount: isize,
    ) {
        // no need to loop trough all the placed symbols if we're inserting at the end of the list
        if symbol_index_to_compare_with == movie.symbols.len() {
            return;
        }
        let placed_symbols = movie.get_placed_symbols_mut(symbol_index_to_check);
        for i in 0..placed_symbols.len() {
            if placed_symbols[i].symbol_index >= symbol_index_to_compare_with {
                placed_symbols[i].symbol_index =
                    (placed_symbols[i].symbol_index as isize + change_amount) as usize;
            }
        }
    }

    fn add_placed_symbols(model: &mut Movie, actions: &Vec<PlacedSymbolAction>) {
        for action in actions {
            model
                .get_placed_symbols_mut(action.editing_symbol_index)
                .insert(action.placed_symbol_index, action.placed_symbol.clone());
        }
    }
    fn remove_placed_symbols(model: &mut Movie, actions: &Vec<PlacedSymbolAction>) {
        // iterate in reverse to make sure the indexes don't change while iterating
        // this assumes the placed symbol indexes are sorted
        for action in actions.iter().rev() {
            model
                .get_placed_symbols_mut(action.editing_symbol_index)
                .remove(action.placed_symbol_index);
        }
    }
}

#[derive(Debug, Clone)]
pub struct PlacedSymbolAction {
    pub editing_symbol_index: SymbolIndexOrRoot,
    pub placed_symbol: PlaceSymbol,
    pub placed_symbol_index: PlacedSymbolIndex,
}
