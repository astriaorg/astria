use futures::channel::mpsc::{self, UnboundedReceiver, UnboundedSender};
use futures::StreamExt;
use serde::{de::DeserializeOwned, Serialize};
use serde_json::value::RawValue;
use std::fmt::Debug;
use std::sync::{Arc, Mutex};
use std::{borrow::Borrow, collections::HashMap};

use async_trait::async_trait;
use core::marker::Send;
use ethers::{
    abi::Hash,
    providers::{
        JsonRpcClient, JsonRpcError, Middleware, MockError, MockProvider, Provider, ProviderError,
        PubsubClient, RpcError,
    },
    types::{Transaction, U256},
};

use super::test_constants;

#[derive(Debug, thiserror::Error)]
pub enum ComposerMockError {
    #[error(transparent)]
    MockProviderError(MockError),
    /// Generic Stream Error
    #[error("Could not convert internal representation into a stream")]
    StreamError,
}

impl RpcError for ComposerMockError {
    fn as_error_response(&self) -> Option<&JsonRpcError> {
        match self {
            ComposerMockError::MockProviderError(MockError::JsonRpcError(e)) => Some(e),
            _ => None,
        }
    }

    fn as_serde_error(&self) -> Option<&serde_json::Error> {
        match self {
            ComposerMockError::MockProviderError(MockError::SerdeJson(e)) => Some(e),
            _ => None,
        }
    }
}

impl From<ComposerMockError> for ProviderError {
    fn from(src: ComposerMockError) -> Self {
        ProviderError::JsonRpcClientError(Box::new(src))
    }
}

#[derive(Debug, Clone)]
pub struct ComposerMockProvider {
    mock_provider: MockProvider,
    current_stream_handle: Arc<Mutex<U256>>,
    stream_handles: Arc<Mutex<HashMap<
        U256,
        (
            UnboundedSender<Box<RawValue>>,
            Arc<Mutex<UnboundedReceiver<Box<RawValue>>>>,
        ),
    >>>,
}

impl ComposerMockProvider {
    fn new() -> Self {
        Self {
            mock_provider: MockProvider::new(),
            stream_handles: Arc::new(Mutex::new(HashMap::new())),
            current_stream_handle: Arc::new(Mutex::new(0.into())),
        }
    }

    pub fn init_provider() -> (Provider<Self>, ComposerMockProvider) {
        let mock = Self::new();
        let mock_clone = mock.clone();
        (Provider::new(mock), mock_clone)
    }

    pub fn push<T: Serialize + Send + Sync, K: Borrow<T>>(
        &self,
        data: K,
    ) -> Result<(), ComposerMockError> {
        self.mock_provider
            .push(data)
            .map_err(|e| ComposerMockError::MockProviderError(e))
    }

    async fn drain_sync_queue_to_stream(&mut self, stream_id: U256) {
        let stream_handles = self.stream_handles.lock().unwrap();
        let stream = stream_handles.get(&stream_id);
        assert!(stream.is_some());
        let (mut stream, _) = stream.unwrap().clone();

        while let Ok(value) = self
            .mock_provider
            .request::<[u64; 0], Box<RawValue>>("", [])
            .await
        {
            stream.start_send(value).unwrap();
        }
    }

    fn init_stream(&mut self) -> U256 {
        let (stream_handle, sink_handle) = mpsc::unbounded::<Box<RawValue>>();

        let mutex_sink = Arc::new(Mutex::new(sink_handle));
        let mut current_stream_handle = self.current_stream_handle.lock().unwrap();
        *current_stream_handle += 1.into();

        let mut mock_stream_handles = self.stream_handles.lock().unwrap();

        mock_stream_handles
            .insert(*current_stream_handle, (stream_handle, mutex_sink));

        *current_stream_handle
    }

    pub async fn setup_subscription(&mut self) {
        // Initialize a mock stream
        let stream_id = self.init_stream();

        // turn mock data into subscription stream
        self.drain_sync_queue_to_stream(stream_id).await;

        // Push the subscription id to the responses queue 
        // Need to do this because the JSONRPC subscription request returns an subscription id
        self.push(stream_id).unwrap();
    }
}

#[cfg_attr(target_arch = "wasm32", async_trait(?Send))]
#[cfg_attr(not(target_arch = "wasm32"), async_trait)]
impl JsonRpcClient for ComposerMockProvider {
    type Error = ComposerMockError;

    async fn request<T: Debug + Serialize + Send + Sync, R: Send + DeserializeOwned>(
        &self,
        method: &str,
        params: T,
    ) -> Result<R, ComposerMockError> {
        self.mock_provider
            .request(method, params)
            .await
            .map_err(|e| ComposerMockError::MockProviderError(e))
    }
}

impl PubsubClient for ComposerMockProvider {
    type NotificationStream = mpsc::UnboundedReceiver<Box<RawValue>>;
    fn subscribe<T: Into<ethers::types::U256>>(
        &self,
        id: T,
    ) -> Result<Self::NotificationStream, Self::Error> {
        let (mut stream_handle, sink_handle) = mpsc::unbounded::<Box<RawValue>>();

        let stream_handles = self.stream_handles.lock().unwrap();
        let (_, receiver) = stream_handles.get(&id.into()).unwrap().clone();

        // Spawn a task that forwards items from a mock stream to the subscription stream
        tokio::task::spawn(async move {
            let mut receiver_clone = receiver.lock().unwrap();
            while let Ok(Some(x)) = receiver_clone.try_next() {
                // This should always succeed
                stream_handle.start_send(x).unwrap();
            }
        });

        Ok(sink_handle)
    }

    fn unsubscribe<T: Into<ethers::types::U256>>(&self, id: T) -> Result<(), Self::Error> {
        Ok(())
    }
}

#[tokio::test]
pub async fn custom_provider_works_with_jsonrpc_calls() {
    let mut file_path = std::env::current_dir().unwrap();
    file_path.push("src/tests");
    file_path.push(test_constants::MOCK_FILE_DIR);
    file_path.push(test_constants::PENDING_TX_SERIALIZATION_FILE);

    let file = std::fs::File::open(file_path).unwrap();
    let reader = std::io::BufReader::new(file);

    let pending_tx_vec: Vec<Transaction> = serde_json::from_reader(reader).unwrap();
    let num_tx = pending_tx_vec.len();

    let (pr, mock) = ComposerMockProvider::init_provider();

    for tx in &pending_tx_vec {
        mock.push(tx.clone()).unwrap();
    }

    for i in (num_tx - 1)..0 {
        assert_eq!(
            pr.get_transaction(Hash::from_low_u64_be(32))
                .await
                .unwrap()
                .unwrap()
                .hash,
            pending_tx_vec[i].hash
        );
    }
}

#[tokio::test]
pub async fn custom_provider_works_with_pubsub_calls() {
    let mut file_path = std::env::current_dir().unwrap();
    file_path.push("src/tests");
    file_path.push(test_constants::MOCK_FILE_DIR);
    file_path.push(test_constants::PENDING_TX_SERIALIZATION_FILE);

    let file = std::fs::File::open(file_path).unwrap();
    let reader = std::io::BufReader::new(file);

    let pending_tx_vec: Vec<Transaction> = serde_json::from_reader(reader).unwrap();
    let num_tx = pending_tx_vec.len();

    let (pr, mut mock) = ComposerMockProvider::init_provider();

    // Send all the mock data to the mock provider
    for tx in &pending_tx_vec {
        mock.push(tx.clone().hash).unwrap();
    }

    mock.setup_subscription().await;

    let mut subscription = pr.subscribe_pending_txs().await.unwrap();

    for i in (num_tx - 1)..0 {
        let val = subscription.next().await.unwrap(); // This value has to be present in stream
        assert_eq!(pending_tx_vec[i].hash, val);
    }
}
