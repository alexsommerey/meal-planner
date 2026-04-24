-- Schema for the recipe domain. STRICT mode rejects rows that don't match
-- the declared types — closer to what we'd get from Postgres than SQLite's
-- usual permissive typing. UUIDs are stored as TEXT (36-char hyphenated)
-- for browseability; flip to BLOB if storage size matters at some point.

CREATE TABLE ingredients (
    id   TEXT PRIMARY KEY,
    name TEXT NOT NULL UNIQUE
) STRICT;

CREATE TABLE recipes (
    id   TEXT PRIMARY KEY,
    name TEXT NOT NULL
) STRICT;

CREATE TABLE recipe_ingredients (
    recipe_id     TEXT    NOT NULL REFERENCES recipes(id)     ON DELETE CASCADE,
    ingredient_id TEXT    NOT NULL REFERENCES ingredients(id) ON DELETE RESTRICT,
    -- Quantities are stored in canonical SI base units (kg / l) plus `piece`
    -- for counts. Conversion from cooking units (g, ml, tsp, tbsp, cup) happens
    -- at the API boundary; once written, the original input unit is lost.
    amount        REAL    NOT NULL,
    unit          TEXT    NOT NULL,
    position      INTEGER NOT NULL,
    PRIMARY KEY (recipe_id, position)
) STRICT;

CREATE INDEX idx_recipe_ingredients_recipe_id ON recipe_ingredients(recipe_id);
