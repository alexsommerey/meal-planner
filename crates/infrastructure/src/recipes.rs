use std::collections::HashMap;
use std::sync::Mutex;

use application::recipes::{RecipeRepository, RepoError};
use domain::{Recipe, RecipeId};

#[derive(Default)]
pub struct InMemoryRecipeRepository {
    inner: Mutex<HashMap<RecipeId, Recipe>>,
}

impl InMemoryRecipeRepository {
    pub fn new() -> Self {
        Self::default()
    }
}

impl RecipeRepository for InMemoryRecipeRepository {
    async fn save(&self, recipe: &Recipe) -> Result<(), RepoError> {
        let mut map = self
            .inner
            .lock()
            .map_err(|_| RepoError::Storage("mutex poisoned".into()))?;
        map.insert(recipe.id.clone(), recipe.clone());
        Ok(())
    }

    async fn list(&self) -> Result<Vec<Recipe>, RepoError> {
        let map = self
            .inner
            .lock()
            .map_err(|_| RepoError::Storage("mutex poisoned".into()))?;
        Ok(map.values().cloned().collect())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use domain::{IngredientId, Quantity, RecipeIngredient};

    fn sample_recipe(id: &str, name: &str) -> Recipe {
        Recipe {
            id: RecipeId(id.into()),
            name: name.into(),
            ingredients: vec![RecipeIngredient {
                ingredient: IngredientId("bread".into()),
                quantity: Quantity { grams: 50.0 },
            }],
        }
    }

    #[tokio::test]
    async fn save_then_list_returns_saved_recipe() {
        let repo = InMemoryRecipeRepository::new();
        repo.save(&sample_recipe("r1", "Toast")).await.unwrap();

        let listed = repo.list().await.unwrap();
        assert_eq!(listed.len(), 1);
        assert_eq!(listed[0].name, "Toast");
    }

    #[tokio::test]
    async fn list_on_empty_repo_returns_empty() {
        let repo = InMemoryRecipeRepository::new();
        let listed = repo.list().await.unwrap();
        assert!(listed.is_empty());
    }

    #[tokio::test]
    async fn save_with_existing_id_overwrites() {
        let repo = InMemoryRecipeRepository::new();
        repo.save(&sample_recipe("r1", "Toast")).await.unwrap();
        repo.save(&sample_recipe("r1", "Toast II")).await.unwrap();

        let listed = repo.list().await.unwrap();
        assert_eq!(listed.len(), 1);
        assert_eq!(listed[0].name, "Toast II");
    }
}
