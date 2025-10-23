/// Database Module
///
/// This module handles all PostgreSQL database operations including:
/// - Connection pool management
/// - Schema migrations
/// - CRUD operations for blocks, transactions, and instructions
use anyhow::{Context, Result};
use sqlx::{postgres::PgPoolOptions, PgPool};

pub struct Database {
    pool: PgPool,
}

impl Database {
    /// Create a new database connection pool
    pub async fn new(database_url: &str) -> Result<Self> {
        let pool = PgPoolOptions::new()
            .max_connections(5)
            .connect(database_url)
            .await
            .context("Failed to connect to PostgreSQL database")?;

        Ok(Self { pool })
    }

    /// Get a reference to the connection pool
    pub fn pool(&self) -> &PgPool {
        &self.pool
    }

    /// Run database migrations
    pub async fn migrate(&self) -> Result<()> {
        sqlx::migrate!("./migrations").run(&self.pool).await.context("Failed to run database migrations")?;

        tracing::info!("Database migrations completed successfully");
        Ok(())
    }

    /// Test the database connection
    pub async fn test_connection(&self) -> Result<()> {
        sqlx::query("SELECT 1").execute(&self.pool).await.context("Database connection test failed")?;

        Ok(())
    }

    /// Load program registry from database
    pub async fn load_program_registry(&self) -> Result<Vec<ProgramInfo>> {
        let programs = sqlx::query_as::<_, ProgramInfo>(
            "SELECT program_id, program_name, program_type FROM program_registry ORDER BY program_name",
        )
        .fetch_all(&self.pool)
        .await
        .context("Failed to load program registry")?;

        Ok(programs)
    }
}

/// Program information from the registry
#[derive(Debug, Clone, sqlx::FromRow)]
pub struct ProgramInfo {
    pub program_id: String,
    pub program_name: String,
    pub program_type: Option<String>,
}
