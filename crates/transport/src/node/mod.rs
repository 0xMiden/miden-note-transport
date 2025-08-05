use self::grpc::{GrpcServer, GrpcServerConfig};
use crate::{
    Result,
    database::{Database, DatabaseConfig},
};
use std::sync::Arc;
use tracing::{error, info};

pub mod grpc;

/// Miden Private Transport Node
pub struct Node {
    /// Serve client requests
    grpc: GrpcServer,

    // To be used in other services, .e.g. P2P, DB maintenance
    _database: Arc<Database>,
}

#[derive(Debug, Default, Clone)]
pub struct NodeConfig {
    pub grpc: GrpcServerConfig,
    pub database: DatabaseConfig,
}

impl Node {
    pub async fn init(config: NodeConfig) -> Result<Self> {
        let database = Arc::new(Database::connect(config.database).await?);

        let grpc = GrpcServer::new(database.clone(), config.grpc);

        Ok(Self {
            grpc,
            _database: database,
        })
    }

    pub async fn entrypoint(self) {
        info!("Starting Miden Transport Node");
        if let Err(e) = self.grpc.serve().await {
            error!("Server error: {e}");
        }
    }
}
