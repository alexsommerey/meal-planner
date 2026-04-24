//! Use cases. Orchestrates `domain` via traits that `infrastructure` implements.
//!
//! Example shape for when you add your first use case:
//!
//! ```ignore
//! pub trait RecipeRepository {
//!     async fn find(&self, id: &RecipeId) -> Result<Option<Recipe>, RepoError>;
//!     async fn save(&self, recipe: &Recipe) -> Result<(), RepoError>;
//! }
//! ```
