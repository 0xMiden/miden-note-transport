use std::time::Duration;

use miden_objects::address::Address;
use miden_private_transport_client::{
    TransportLayerClient,
    database::{Database, DatabaseConfig},
    grpc::GrpcClient,
    test_utils::mock_address,
};
use miden_private_transport_node::{
    Node, NodeConfig, database::DatabaseConfig as NodeDatabaseConfig, node::grpc::GrpcServerConfig,
};
use tokio::{task::JoinHandle, time::sleep};

pub async fn spawn_test_server(port: u16) -> JoinHandle<()> {
    let database_config = NodeDatabaseConfig {
        url: format!("file::memory:?cache=shared&name=test_{port}"),
        ..Default::default()
    };
    let config = NodeConfig {
        grpc: GrpcServerConfig { port, ..Default::default() },
        database: database_config,
    };

    let server = Node::init(config).await.unwrap();
    let handle = tokio::spawn(server.entrypoint());
    // Wait for startup
    sleep(Duration::from_millis(100)).await;
    handle
}

pub async fn test_client(port: u16) -> (TransportLayerClient, Address) {
    let timeout_ms = 1000;
    let url = format!("http://127.0.0.1:{port}");

    let grpc_client = Box::new(GrpcClient::connect(url, timeout_ms).await.unwrap());

    let address = mock_address();

    let db_config = DatabaseConfig {
        url: format!("file::memory:?cache=shared&name=test_{port}"),
        ..Default::default()
    };
    let db = Database::new_sqlite(db_config).await.unwrap();
    let client = TransportLayerClient::new(grpc_client, db, vec![address.clone()]);

    (client, address)
}
