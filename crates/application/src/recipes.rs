use std::future::Future;

use domain::{Recipe, RecipeId, RecipeIngredient};
use thiserror::Error;
use uuid::Uuid;

#[derive(Debug, Error)]
pub enum RepoError {
    #[error("storage error: {0}")]
    Storage(String),
}

/// `Send` bound is added on the returned future so axum handlers (which
/// require `Send` futures for multi-threaded executors) can `.await` these
/// methods. Native AFIT does not include auto traits by default.
pub trait RecipeRepository: Send + Sync {
    fn save(&self, recipe: &Recipe) -> impl Future<Output = Result<(), RepoError>> + Send;
    fn list(&self) -> impl Future<Output = Result<Vec<Recipe>, RepoError>> + Send;
}

pub async fn create_recipe<R: RecipeRepository>(
    repo: &R,
    name: String,
    ingredients: Vec<RecipeIngredient>,
) -> Result<Recipe, RepoError> {
    let recipe = Recipe {
        id: RecipeId(Uuid::new_v4().to_string()),
        name,
        ingredients,
    };
    repo.save(&recipe).await?;
    Ok(recipe)
}

pub async fn list_recipes<R: RecipeRepository>(repo: &R) -> Result<Vec<Recipe>, RepoError> {
    repo.list().await
}
