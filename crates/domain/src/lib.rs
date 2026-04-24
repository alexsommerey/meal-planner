//! Pure domain types. No I/O, no async, no frameworks.

use serde::{Deserialize, Serialize};

/// TODO: extend to handle units (g, kg, ml, cups), density-dependent
/// conversion, and fuzzy amounts ("a pinch"). Start with grams only.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Quantity {
    pub grams: f64,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct IngredientId(pub String);

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Ingredient {
    pub id: IngredientId,
    pub name: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct RecipeId(pub String);

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Recipe {
    pub id: RecipeId,
    pub name: String,
    pub ingredients: Vec<RecipeIngredient>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RecipeIngredient {
    pub ingredient: IngredientId,
    pub quantity: Quantity,
}
