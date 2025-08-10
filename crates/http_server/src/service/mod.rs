pub trait Service<Request>: Send + Sync + 'static {
    type Error;
    type Response;
    type Future;

    fn poll_ready(&self);
    fn call(
        &self,
        req: Request,
    ) -> impl Future<Output = Result<Self::Response, Self::Error>> + Send;
}
