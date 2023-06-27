// TODO: tracing

use astria_composer::searcher;

pub struct TestSearcher {
    pub inner: searcher::Searcher,
}

pub async fn spawn_searcher() -> TestSearcher {
    todo!("init tracing");
    todo!("init config");
    todo!("spawn searcher task");
    todo!("init mocks?")
}
