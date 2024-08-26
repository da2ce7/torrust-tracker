use crate::service::Service;
use crate::starting::Starting;

pub struct Stopped<S> {
    service: S,
}

impl<S> Stopped<S>
where
    S: Service,
{
    pub fn start<'a>(self) -> Result<Starting<'a, S>, S::Error> {
        let starting_service = self.service.start()?;
    }
}
