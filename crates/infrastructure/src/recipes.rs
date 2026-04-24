use std::str::FromStr;

use application::recipes::{CreateRecipeInput, RecipeRepository, RepoError};
use domain::{Ingredient, IngredientId, Quantity, Recipe, RecipeId, RecipeIngredient, Unit};
use sqlx::sqlite::{SqliteConnectOptions, SqlitePool, SqlitePoolOptions};
use uuid::Uuid;

pub struct SqlxRecipeRepository {
    pool: SqlitePool,
}

impl SqlxRecipeRepository {
    /// Create the pool, run pending migrations, and return a ready-to-use
    /// repository. The URL accepts anything sqlx understands — file paths via
    /// `sqlite:meal-planner.db?mode=rwc`, or `sqlite::memory:` for tests.
    pub async fn connect(database_url: &str) -> Result<Self, RepoError> {
        let opts = SqliteConnectOptions::from_str(database_url).map_err(map_err)?;
        let pool = SqlitePoolOptions::new()
            .connect_with(opts)
            .await
            .map_err(map_err)?;
        sqlx::migrate!("../../migrations")
            .run(&pool)
            .await
            .map_err(map_err)?;
        Ok(Self { pool })
    }
}

impl RecipeRepository for SqlxRecipeRepository {
    async fn create(&self, input: CreateRecipeInput) -> Result<Recipe, RepoError> {
        let mut tx = self.pool.begin().await.map_err(map_err)?;

        let recipe_id = Uuid::new_v4();
        sqlx::query("INSERT INTO recipes (id, name) VALUES (?1, ?2)")
            .bind(recipe_id.to_string())
            .bind(&input.name)
            .execute(&mut *tx)
            .await
            .map_err(map_err)?;

        let mut ingredients = Vec::with_capacity(input.items.len());
        for (position, item) in input.items.iter().enumerate() {
            let ingredient_id = upsert_ingredient(&mut tx, &item.ingredient_name).await?;
            sqlx::query(
                "INSERT INTO recipe_ingredients
                    (recipe_id, ingredient_id, amount, unit, position)
                 VALUES (?1, ?2, ?3, ?4, ?5)",
            )
            .bind(recipe_id.to_string())
            .bind(ingredient_id.to_string())
            .bind(item.quantity.amount)
            .bind(item.quantity.unit.as_str())
            .bind(i64::try_from(position).expect("position fits in i64"))
            .execute(&mut *tx)
            .await
            .map_err(map_err)?;

            ingredients.push(RecipeIngredient {
                ingredient: Ingredient {
                    id: IngredientId(ingredient_id),
                    name: item.ingredient_name.clone(),
                },
                quantity: item.quantity,
            });
        }

        tx.commit().await.map_err(map_err)?;

        Ok(Recipe {
            id: RecipeId(recipe_id),
            name: input.name,
            ingredients,
        })
    }

    async fn list(&self) -> Result<Vec<Recipe>, RepoError> {
        // Two queries + an in-memory join keep the row shape simple. With a
        // tiny dataset this is fine; if recipe counts grow we can switch to a
        // single LEFT JOIN and group in Rust.
        let recipe_rows: Vec<(String, String)> =
            sqlx::query_as("SELECT id, name FROM recipes ORDER BY name")
                .fetch_all(&self.pool)
                .await
                .map_err(map_err)?;

        let item_rows: Vec<(String, String, String, f64, String, i64)> = sqlx::query_as(
            "SELECT ri.recipe_id,
                    i.id,
                    i.name,
                    ri.amount,
                    ri.unit,
                    ri.position
               FROM recipe_ingredients ri
               JOIN ingredients i ON i.id = ri.ingredient_id
              ORDER BY ri.recipe_id, ri.position",
        )
        .fetch_all(&self.pool)
        .await
        .map_err(map_err)?;

        let mut recipes = Vec::with_capacity(recipe_rows.len());
        for (id_str, name) in recipe_rows {
            let id = parse_uuid(&id_str)?;
            let ingredients = item_rows
                .iter()
                .filter(|(rid, ..)| rid == &id_str)
                .map(|(_, iid, iname, amount, unit, _)| {
                    Ok(RecipeIngredient {
                        ingredient: Ingredient {
                            id: IngredientId(parse_uuid(iid)?),
                            name: iname.clone(),
                        },
                        quantity: Quantity {
                            amount: *amount,
                            unit: parse_unit(unit)?,
                        },
                    })
                })
                .collect::<Result<Vec<_>, RepoError>>()?;
            recipes.push(Recipe {
                id: RecipeId(id),
                name,
                ingredients,
            });
        }

        Ok(recipes)
    }
}

async fn upsert_ingredient(
    tx: &mut sqlx::Transaction<'_, sqlx::Sqlite>,
    name: &str,
) -> Result<Uuid, RepoError> {
    if let Some((id_str,)) =
        sqlx::query_as::<_, (String,)>("SELECT id FROM ingredients WHERE name = ?1")
            .bind(name)
            .fetch_optional(&mut **tx)
            .await
            .map_err(map_err)?
    {
        return parse_uuid(&id_str);
    }

    let id = Uuid::new_v4();
    sqlx::query("INSERT INTO ingredients (id, name) VALUES (?1, ?2)")
        .bind(id.to_string())
        .bind(name)
        .execute(&mut **tx)
        .await
        .map_err(map_err)?;
    Ok(id)
}

fn map_err<E: std::fmt::Display>(e: E) -> RepoError {
    RepoError::Storage(e.to_string())
}

fn parse_uuid(s: &str) -> Result<Uuid, RepoError> {
    Uuid::parse_str(s).map_err(map_err)
}

fn parse_unit(s: &str) -> Result<Unit, RepoError> {
    match s {
        "kg" => Ok(Unit::Kilogram),
        "l" => Ok(Unit::Liter),
        "piece" => Ok(Unit::Piece),
        other => Err(RepoError::Storage(format!("unknown unit `{other}`"))),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use domain::{InputUnit, Quantity};

    async fn fresh_repo() -> SqlxRecipeRepository {
        SqlxRecipeRepository::connect("sqlite::memory:")
            .await
            .unwrap()
    }

    fn item(name: &str, amount: f64, unit: InputUnit) -> application::recipes::CreateRecipeItem {
        application::recipes::CreateRecipeItem {
            ingredient_name: name.into(),
            quantity: Quantity::from_input(amount, unit),
        }
    }

    #[tokio::test]
    async fn list_on_empty_repo_returns_empty() {
        let repo = fresh_repo().await;
        assert!(repo.list().await.unwrap().is_empty());
    }

    #[tokio::test]
    async fn create_then_list_round_trip() {
        let repo = fresh_repo().await;
        let created = repo
            .create(CreateRecipeInput {
                name: "Toast".into(),
                items: vec![item("bread", 50.0, InputUnit::G)],
            })
            .await
            .unwrap();

        let listed = repo.list().await.unwrap();
        assert_eq!(listed.len(), 1);
        assert_eq!(listed[0].id, created.id);
        assert_eq!(listed[0].name, "Toast");
        assert_eq!(listed[0].ingredients.len(), 1);
        assert_eq!(listed[0].ingredients[0].ingredient.name, "bread");
        assert_eq!(listed[0].ingredients[0].quantity.unit, Unit::Kilogram);
        assert!((listed[0].ingredients[0].quantity.amount - 0.05).abs() < 1e-9);
    }

    #[tokio::test]
    async fn shared_ingredient_name_is_not_duplicated() {
        let repo = fresh_repo().await;
        repo.create(CreateRecipeInput {
            name: "Toast".into(),
            items: vec![item("bread", 50.0, InputUnit::G)],
        })
        .await
        .unwrap();
        repo.create(CreateRecipeInput {
            name: "Sandwich".into(),
            items: vec![
                item("bread", 100.0, InputUnit::G),
                item("cheese", 30.0, InputUnit::G),
            ],
        })
        .await
        .unwrap();

        let recipes = repo.list().await.unwrap();
        let bread_ids: Vec<_> = recipes
            .iter()
            .flat_map(|r| r.ingredients.iter())
            .filter(|i| i.ingredient.name == "bread")
            .map(|i| i.ingredient.id)
            .collect();
        assert_eq!(bread_ids.len(), 2, "bread should appear in both recipes");
        assert_eq!(
            bread_ids[0], bread_ids[1],
            "bread should reference the same ingredient row"
        );
    }
}
