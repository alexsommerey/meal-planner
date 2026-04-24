//! Pure domain types. No I/O, no async, no frameworks.

use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// Canonical units used for storage and inside the domain. The set is small on
/// purpose: pick a single base unit per dimension (mass, volume, count) and
/// normalize to it on the way in.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Unit {
    Kilogram,
    Liter,
    Piece,
}

impl Unit {
    pub fn as_str(self) -> &'static str {
        match self {
            Unit::Kilogram => "kg",
            Unit::Liter => "l",
            Unit::Piece => "piece",
        }
    }
}

/// Cooking-friendly units accepted at the API boundary. Each one maps to a
/// canonical [`Quantity`] via [`Quantity::from_input`]. We don't store these
/// — once converted, the original choice is lost. Add `Pinch` / `ToTaste`
/// here when fuzzy amounts become a real requirement.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum InputUnit {
    G,
    Ml,
    Tsp,
    Tbsp,
    Cup,
    Piece,
}

/// Quantities are always in canonical units (kg / l / piece). Conversion from
/// cooking units happens at the boundary; presentation layers are free to
/// display these back as g / ml / etc.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct Quantity {
    pub amount: f64,
    pub unit: Unit,
}

impl Quantity {
    /// Convert a user-supplied amount + cooking unit to the canonical form.
    /// Cup uses the metric definition (1 cup = 250 ml); tsp/tbsp use the
    /// international metric values (5 ml / 15 ml).
    pub fn from_input(amount: f64, unit: InputUnit) -> Self {
        match unit {
            InputUnit::G => Self {
                amount: amount / 1000.0,
                unit: Unit::Kilogram,
            },
            InputUnit::Ml => Self {
                amount: amount / 1000.0,
                unit: Unit::Liter,
            },
            InputUnit::Tsp => Self {
                amount: amount * 0.005,
                unit: Unit::Liter,
            },
            InputUnit::Tbsp => Self {
                amount: amount * 0.015,
                unit: Unit::Liter,
            },
            InputUnit::Cup => Self {
                amount: amount * 0.25,
                unit: Unit::Liter,
            },
            InputUnit::Piece => Self {
                amount,
                unit: Unit::Piece,
            },
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct IngredientId(pub Uuid);

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Ingredient {
    pub id: IngredientId,
    pub name: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct RecipeId(pub Uuid);

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Recipe {
    pub id: RecipeId,
    pub name: String,
    pub ingredients: Vec<RecipeIngredient>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RecipeIngredient {
    pub ingredient: Ingredient,
    pub quantity: Quantity,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn grams_normalize_to_kilograms() {
        let q = Quantity::from_input(120.0, InputUnit::G);
        assert_eq!(q.unit, Unit::Kilogram);
        assert!((q.amount - 0.12).abs() < 1e-9);
    }

    #[test]
    fn milliliters_normalize_to_liters() {
        let q = Quantity::from_input(250.0, InputUnit::Ml);
        assert_eq!(q.unit, Unit::Liter);
        assert!((q.amount - 0.25).abs() < 1e-9);
    }

    #[test]
    fn tablespoons_use_metric_15ml() {
        let q = Quantity::from_input(2.0, InputUnit::Tbsp);
        assert_eq!(q.unit, Unit::Liter);
        assert!((q.amount - 0.030).abs() < 1e-9);
    }

    #[test]
    fn pieces_pass_through_unchanged() {
        let q = Quantity::from_input(3.0, InputUnit::Piece);
        assert_eq!(q.unit, Unit::Piece);
        assert_eq!(q.amount, 3.0);
    }
}
