use std::{future::Future, marker::PhantomData};

pub mod select2;

#[derive(Default)]
pub struct DummyFuture<O> {
    _tmp: PhantomData<O>,
}

impl<O> Future for DummyFuture<O> {
    type Output = O;

    fn poll(self: std::pin::Pin<&mut Self>, _cx: &mut std::task::Context<'_>) -> std::task::Poll<Self::Output> {
        std::task::Poll::Pending
    }
}
