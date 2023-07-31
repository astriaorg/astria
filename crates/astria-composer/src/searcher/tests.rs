mod tests {
    use ethers::providers::Ws;

    use crate::{
        config::Config,
        searcher::Searcher,
    };

    #[tokio::test]
    async fn new_from_valid_config() {
        let cfg = Config::default();

        // FIXME: fails because we need to mock ETH provider
        let searcher = Searcher::<Ws>::new_ws(&cfg).await;
        assert!(searcher.is_ok());
    }
}