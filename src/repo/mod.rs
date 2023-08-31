mod storage;
mod repo;

use crate::feed::Feed;
pub use repo::Repository;

#[derive(Clone, Debug)]
pub enum RepositoryEvent {
    Refresh,
    RetrievedAll(Vec<Feed>),
    RetrievedOne(Feed),
    Requesting(usize),
    Requested((usize, usize)),
    Errored,
    Aborted,
}
