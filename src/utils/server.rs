use std::{future::Future, pin::Pin};

pub struct Server {
    fut: BoxFuture<'static, std::io::Result<()>>,
}

type BoxFuture<'a, T> = Pin<Box<dyn Future<Output = T> + Send + 'a>>;

impl Future for Server {
    type Output = std::io::Result<()>;

    #[inline]
    fn poll(
        self: Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
    ) -> std::task::Poll<Self::Output> {
        Pin::new(&mut Pin::into_inner(self).fut).poll(cx)
    }
}

impl Server {
    pub fn new(fut: impl Future<Output = std::io::Result<()>> + Send + 'static) -> Self {
        Self { fut: Box::pin(fut) }
    }
}
