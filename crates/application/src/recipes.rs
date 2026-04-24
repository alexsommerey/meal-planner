use std::future::Future;

use domain::{Quantity, Recipe};
use thiserror::Error;

#[derive(Debug, Error)]
pub enum RepoError {
    #[error("storage error: {0}")]
    Storage(String),
}

/// Input shape for creating a recipe. Ingredient identity lives in the
/// adapter — it upserts by name to find or mint an `IngredientId`, so use
/// cases never have to think about the catalog.
#[derive(Debug, Clone)]
pub struct CreateRecipeInput {
    pub name: String,
    pub items: Vec<CreateRecipeItem>,
}

#[derive(Debug, Clone)]
pub struct CreateRecipeItem {
    pub ingredient_name: String,
    pub quantity: Quantity,
}

/// `Send` bound is added on the returned future so axum handlers (which
/// require `Send` futures for multi-threaded executors) can `.await` these
/// methods. Native AFIT does not include auto traits by default.
pub trait RecipeRepository: Send + Sync {
    fn create(
        &self,
        input: CreateRecipeInput,
    ) -> impl Future<Output = Result<Recipe, RepoError>> + Send;
    fn list(&self) -> impl Future<Output = Result<Vec<Recipe>, RepoError>> + Send;
}

pub async fn create_recipe<R: RecipeRepository>(
    repo: &R,
    input: CreateRecipeInput,
) -> Result<Recipe, RepoError> {
    repo.create(input).await
}

pub async fn list_recipes<R: RecipeRepository>(repo: &R) -> Result<Vec<Recipe>, RepoError> {
    repo.list().await
}
