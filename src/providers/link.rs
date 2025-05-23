use tokio_util::sync::CancellationToken;

#[derive(Default)]
pub struct Link(pub(crate) CancellationToken);

impl Drop for Link {
    fn drop(&mut self) {
        self.0.cancel();
    }
}
