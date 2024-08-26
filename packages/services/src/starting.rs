use futures::future::BoxFuture;

use crate::service::Service;

pub struct Starting<'a, S>
where
    S: Service,
{
    pub(super) starting_service: BoxFuture<'a, Result<S, S::Error>>,
}
