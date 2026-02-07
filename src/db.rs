use anyhow::Result;
use serde::{de::DeserializeOwned, Serialize};
use std::sync::Arc;
use surrealdb::engine::any::{connect, Any};
use surrealdb::opt::auth::{Database, Namespace, Record, Root};
use surrealdb::Surreal;

/// A wrapper around SurrealDB connection that provides type-safe operations
/// for the Onyx knowledge graph.
#[derive(Clone)]
pub struct OnyxDatabase {
    db: Arc<Surreal<Any>>,
}

impl OnyxDatabase {
    /// Create a new in-memory database instance.
    pub async fn new_memory() -> Result<Self> {
        let db = connect("memory").await?;
        db.use_ns("onyx").use_db("onyx").await?;

        let db = Arc::new(db);
        Self::init_schema(&db).await?;

        Ok(Self { db })
    }

    /// Create a new database instance connecting to a SurrealDB server.
    pub async fn new_remote(
        url: &str,
        username: &str,
        password: &str,
    ) -> Result<Self> {
        let db = connect(url).await?;

        // Sign in as root
        db.signin(Root {
            username: username.to_string(),
            password: password.to_string(),
        })
        .await?;

        db.use_ns("onyx").use_db("onyx").await?;

        let db = Arc::new(db);
        Self::init_schema(&db).await?;

        Ok(Self { db })
    }

    /// Initialize the database schema (tables, indexes, etc.).
    async fn init_schema(db: &Surreal<Any>) -> Result<()> {
        // Define tables
        db.query("DEFINE TABLE node SCHEMAFULL").await?;
        db.query("DEFINE TABLE edge SCHEMAFULL").await?;
        db.query("DEFINE TABLE version SCHEMAFULL").await?;
        db.query("DEFINE TABLE branch SCHEMAFULL").await?;
        db.query("DEFINE TABLE embedding SCHEMAFULL").await?;

        // Define indexes for nodes
        db.query("DEFINE INDEX node_name ON node FIELDS name").await?;
        db.query("DEFINE INDEX node_type ON node FIELDS node_type").await?;
        db.query("DEFINE INDEX node_content_hash ON node FIELDS content_hash").await?;

        // Define indexes for edges
        db.query("DEFINE INDEX edge_source ON edge FIELDS source_id").await?;
        db.query("DEFINE INDEX edge_target ON edge FIELDS target_id").await?;
        db.query("DEFINE INDEX edge_type ON edge FIELDS edge_type").await?;

        // Define indexes for versions
        db.query("DEFINE INDEX version_entity ON version FIELDS entity_id").await?;
        db.query("DEFINE INDEX version_branch ON version FIELDS branch").await?;

        Ok(())
    }

    /// Get a reference to the underlying SurrealDB instance.
    pub fn inner(&self) -> &Surreal<Any> {
        &self.db
    }

    /// Create a new record in the database.
    pub async fn create<T: Serialize + DeserializeOwned + Send + 'static>(
        &self,
        table: &str,
        data: T,
    ) -> Result<Option<T>> {
        let record: Option<T> = self.db.create(table).content(data).await?;
        Ok(record)
    }

    /// Create a record with a specific ID.
    pub async fn create_with_id<T: Serialize + DeserializeOwned + Send + 'static>(
        &self,
        table: &str,
        id: &str,
        data: T,
    ) -> Result<Option<T>> {
        let thing = format!("{}:{}", table, id);
        let record: Option<T> = self.db.create(thing).content(data).await?;
        Ok(record)
    }

    /// Select a record by ID.
    pub async fn select<T: DeserializeOwned + Send + 'static>(
        &self,
        table: &str,
        id: String,
    ) -> Result<Option<T>> {
        let thing = format!("{}:{}", table, id);
        let mut result: Vec<T> = self.db.select(thing).await?;
        Ok(result.pop())
    }

    /// Update a record by ID.
    pub async fn update<T: Serialize + DeserializeOwned + Send + 'static>(
        &self,
        table: &str,
        id: &str,
        data: T,
    ) -> Result<Option<T>> {
        let thing = format!("{}:{}", table, id);
        let mut record: Vec<T> = self.db.update(thing).content(data).await?;
        Ok(record.pop())
    }

    /// Delete a record by ID.
    pub async fn delete(&self, table: &str, id: &str) -> Result<()> {
        let thing = format!("{}:{}", table, id);
        let _: Vec<serde_json::Value> = self.db.delete(thing).await?;
        Ok(())
    }

    /// Execute a custom query.
    pub async fn query(&self, query: &str) -> Result<surrealdb::Response> {
        let response = self.db.query(query).await?;
        Ok(response)
    }

    /// Begin a transaction.
    pub async fn begin_transaction(&self) -> Result<()> {
        self.db.query("BEGIN TRANSACTION").await?;
        Ok(())
    }

    /// Commit the current transaction.
    pub async fn commit_transaction(&self) -> Result<()> {
        self.db.query("COMMIT TRANSACTION").await?;
        Ok(())
    }

    /// Cancel the current transaction.
    pub async fn cancel_transaction(&self) -> Result<()> {
        self.db.query("CANCEL TRANSACTION").await?;
        Ok(())
    }

    /// Check if the database is connected.
    pub async fn health(&self) -> Result<bool> {
        // Simple health check - try to query the info
        let _ = self.db.version().await?;
        Ok(true)
    }

    /// Sign in as a root user.
    pub async fn signin_root(&self, username: &str, password: &str) -> Result<()> {
        self.db
            .signin(Root {
                username: username.to_string(),
                password: password.to_string(),
            })
            .await?;
        Ok(())
    }

    /// Sign in as a namespace user.
    pub async fn signin_namespace(
        &self,
        namespace: &str,
        username: &str,
        password: &str,
    ) -> Result<()> {
        self.db
            .signin(Namespace {
                namespace: namespace.to_string(),
                username: username.to_string(),
                password: password.to_string(),
            })
            .await?;
        Ok(())
    }

    /// Sign in as a database user.
    pub async fn signin_database(
        &self,
        namespace: &str,
        database: &str,
        username: &str,
        password: &str,
    ) -> Result<()> {
        self.db
            .signin(Database {
                namespace: namespace.to_string(),
                database: database.to_string(),
                username: username.to_string(),
                password: password.to_string(),
            })
            .await?;
        Ok(())
    }

    /// Sign up a record-based user and return the auth token.
    pub async fn signup_record<T: Serialize + Send + Sync>(
        &self,
        namespace: &str,
        database: &str,
        access: &str,
        params: T,
    ) -> Result<String> {
        let token = self
            .db
            .signup(Record {
                namespace: namespace.to_string(),
                database: database.to_string(),
                access: access.to_string(),
                params,
            })
            .await?;
        Ok(token)
    }

    /// Sign in a record-based user and return the auth token.
    pub async fn signin_record<T: Serialize + Send + Sync>(
        &self,
        namespace: &str,
        database: &str,
        access: &str,
        params: T,
    ) -> Result<String> {
        let token = self
            .db
            .signin(Record {
                namespace: namespace.to_string(),
                database: database.to_string(),
                access: access.to_string(),
                params,
            })
            .await?;
        Ok(token)
    }

    /// Authenticate using an existing JWT token.
    pub async fn authenticate_token(&self, token: &str) -> Result<()> {
        self.db.authenticate(token).await?;
        Ok(())
    }

    /// Invalidate the current session.
    pub async fn invalidate_session(&self) -> Result<()> {
        self.db.invalidate().await?;
        Ok(())
    }
}

/// Database configuration options.
#[derive(Debug, Clone)]
pub struct DatabaseConfig {
    pub endpoint: DatabaseEndpoint,
    pub namespace: String,
    pub database: String,
}

impl Default for DatabaseConfig {
    fn default() -> Self {
        Self {
            endpoint: DatabaseEndpoint::Memory,
            namespace: "onyx".to_string(),
            database: "onyx".to_string(),
        }
    }
}

/// Database endpoint types.
#[derive(Debug, Clone)]
pub enum DatabaseEndpoint {
    /// In-memory database (for testing).
    Memory,
    /// Remote SurrealDB server.
    Remote {
        url: String,
        username: String,
        password: String,
    },
}

impl DatabaseConfig {
    /// Create a new configuration for an in-memory database.
    pub fn memory() -> Self {
        Self {
            endpoint: DatabaseEndpoint::Memory,
            ..Default::default()
        }
    }

    /// Create a new configuration for a remote SurrealDB server.
    pub fn remote(url: impl Into<String>, username: impl Into<String>, password: impl Into<String>) -> Self {
        Self {
            endpoint: DatabaseEndpoint::Remote {
                url: url.into(),
                username: username.into(),
                password: password.into(),
            },
            ..Default::default()
        }
    }

    /// Connect to the database with this configuration.
    pub async fn connect(&self) -> Result<OnyxDatabase> {
        match &self.endpoint {
            DatabaseEndpoint::Memory => OnyxDatabase::new_memory().await,
            DatabaseEndpoint::Remote {
                url,
                username,
                password,
            } => OnyxDatabase::new_remote(url, username, password).await,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_memory_database() {
        let db = OnyxDatabase::new_memory().await.unwrap();
        assert!(db.health().await.unwrap());
    }
}
