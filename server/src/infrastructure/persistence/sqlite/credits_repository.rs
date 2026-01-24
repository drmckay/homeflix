//! SQLite implementation of CreditsRepository

use async_trait::async_trait;
use sqlx::{Pool, Sqlite, Row};
use crate::domain::repositories::{CreditsRepository, CreditEntry, CreditType};
use crate::shared::error::RepositoryError;

/// SQLite-based credits repository implementation
pub struct SqliteCreditsRepository {
    pool: Pool<Sqlite>,
}

impl SqliteCreditsRepository {
    pub fn new(pool: Pool<Sqlite>) -> Self {
        Self { pool }
    }
}

#[async_trait]
impl CreditsRepository for SqliteCreditsRepository {
    async fn get_credits(&self, media_id: i64) -> Result<Vec<CreditEntry>, RepositoryError> {
        let rows = sqlx::query(
            r#"
            SELECT person_id, person_name, role, character_name, department,
                   profile_url, credit_order, credit_type
            FROM media_credits
            WHERE media_id = ?
            ORDER BY credit_type, credit_order
            "#,
        )
        .bind(media_id)
        .fetch_all(&self.pool)
        .await
        .map_err(|e| RepositoryError::Database(e.to_string()))?;

        Ok(rows
            .into_iter()
            .map(|row| CreditEntry {
                person_id: row.get("person_id"),
                person_name: row.get("person_name"),
                role: row.get("role"),
                character_name: row.get("character_name"),
                department: row.get("department"),
                profile_url: row.get("profile_url"),
                credit_order: row.get("credit_order"),
                credit_type: CreditType::from_str(row.get::<String, _>("credit_type").as_str()),
            })
            .collect())
    }

    async fn get_cast(&self, media_id: i64) -> Result<Vec<CreditEntry>, RepositoryError> {
        let rows = sqlx::query(
            r#"
            SELECT person_id, person_name, role, character_name, department,
                   profile_url, credit_order, credit_type
            FROM media_credits
            WHERE media_id = ? AND credit_type = 'cast'
            ORDER BY credit_order
            "#,
        )
        .bind(media_id)
        .fetch_all(&self.pool)
        .await
        .map_err(|e| RepositoryError::Database(e.to_string()))?;

        Ok(rows
            .into_iter()
            .map(|row| CreditEntry {
                person_id: row.get("person_id"),
                person_name: row.get("person_name"),
                role: row.get("role"),
                character_name: row.get("character_name"),
                department: row.get("department"),
                profile_url: row.get("profile_url"),
                credit_order: row.get("credit_order"),
                credit_type: CreditType::Cast,
            })
            .collect())
    }

    async fn get_crew(&self, media_id: i64) -> Result<Vec<CreditEntry>, RepositoryError> {
        let rows = sqlx::query(
            r#"
            SELECT person_id, person_name, role, character_name, department,
                   profile_url, credit_order, credit_type
            FROM media_credits
            WHERE media_id = ? AND credit_type = 'crew'
            ORDER BY credit_order
            "#,
        )
        .bind(media_id)
        .fetch_all(&self.pool)
        .await
        .map_err(|e| RepositoryError::Database(e.to_string()))?;

        Ok(rows
            .into_iter()
            .map(|row| CreditEntry {
                person_id: row.get("person_id"),
                person_name: row.get("person_name"),
                role: row.get("role"),
                character_name: row.get("character_name"),
                department: row.get("department"),
                profile_url: row.get("profile_url"),
                credit_order: row.get("credit_order"),
                credit_type: CreditType::Crew,
            })
            .collect())
    }

    async fn save_credits(&self, media_id: i64, credits: &[CreditEntry]) -> Result<(), RepositoryError> {
        // Delete existing credits first
        sqlx::query("DELETE FROM media_credits WHERE media_id = ?")
            .bind(media_id)
            .execute(&self.pool)
            .await
            .map_err(|e| RepositoryError::Database(e.to_string()))?;

        // Insert new credits
        for credit in credits {
            sqlx::query(
                r#"
                INSERT INTO media_credits
                (media_id, person_id, person_name, role, character_name, department, profile_url, credit_order, credit_type)
                VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?)
                "#,
            )
            .bind(media_id)
            .bind(credit.person_id)
            .bind(&credit.person_name)
            .bind(&credit.role)
            .bind(&credit.character_name)
            .bind(&credit.department)
            .bind(&credit.profile_url)
            .bind(credit.credit_order)
            .bind(credit.credit_type.as_str())
            .execute(&self.pool)
            .await
            .map_err(|e| RepositoryError::Database(e.to_string()))?;
        }

        Ok(())
    }

    async fn has_credits(&self, media_id: i64) -> Result<bool, RepositoryError> {
        let result: (i64,) = sqlx::query_as(
            "SELECT COUNT(*) FROM media_credits WHERE media_id = ?",
        )
        .bind(media_id)
        .fetch_one(&self.pool)
        .await
        .map_err(|e| RepositoryError::Database(e.to_string()))?;

        Ok(result.0 > 0)
    }

    async fn delete_credits(&self, media_id: i64) -> Result<(), RepositoryError> {
        sqlx::query("DELETE FROM media_credits WHERE media_id = ?")
            .bind(media_id)
            .execute(&self.pool)
            .await
            .map_err(|e| RepositoryError::Database(e.to_string()))?;

        Ok(())
    }
}
