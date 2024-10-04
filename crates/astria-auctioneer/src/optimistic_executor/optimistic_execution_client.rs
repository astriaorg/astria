use tonic::transport::Uri;
pub(crate) struct OptimisticExecutionClient {
    inner: OptimisticExecutionServiceClient<Channel>,
    uri: Uri,
}
