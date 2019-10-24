extern crate component_future;

mod run;

#[test]
fn test_basic_future() {
    struct TwoFutures {
        fut1: Option<
            Box<
                dyn futures::future::Future<Item = u32, Error = String>
                    + Send,
            >,
        >,
        fut2: Option<
            Box<
                dyn futures::future::Future<Item = u32, Error = String>
                    + Send,
            >,
        >,
        val: u32,
    }

    impl TwoFutures {
        fn new<F1, F2>(fut1: F1, fut2: F2) -> Self
        where
            F1: futures::future::Future<Item = u32, Error = String>
                + Send
                + 'static,
            F2: futures::future::Future<Item = u32, Error = String>
                + Send
                + 'static,
        {
            Self {
                fut1: Some(Box::new(fut1)),
                fut2: Some(Box::new(fut2)),
                val: 1,
            }
        }
    }

    #[allow(clippy::type_complexity)]
    impl TwoFutures {
        const POLL_FNS:
            &'static [&'static dyn for<'a> Fn(
                &'a mut Self,
            )
                -> component_future::Poll<
                u32,
                String,
            >] = &[
            &Self::poll_future_1,
            &Self::poll_future_2,
            &Self::poll_return,
        ];

        fn poll_future_1(&mut self) -> component_future::Poll<u32, String> {
            if let Some(fut1) = &mut self.fut1 {
                let val = component_future::try_ready!(fut1.poll());
                self.val += val;
                self.fut1.take();
                Ok(component_future::Async::DidWork)
            } else {
                Ok(component_future::Async::NothingToDo)
            }
        }

        fn poll_future_2(&mut self) -> component_future::Poll<u32, String> {
            if self.fut1.is_some() {
                return Ok(component_future::Async::NothingToDo);
            }

            if let Some(fut2) = &mut self.fut2 {
                let val = component_future::try_ready!(fut2.poll());
                self.val *= val;
                self.fut2.take();
                Ok(component_future::Async::DidWork)
            } else {
                Ok(component_future::Async::NothingToDo)
            }
        }

        fn poll_return(&mut self) -> component_future::Poll<u32, String> {
            if self.fut1.is_some() || self.fut2.is_some() {
                return Ok(component_future::Async::NothingToDo);
            }

            Ok(component_future::Async::Ready(self.val))
        }
    }

    impl futures::future::Future for TwoFutures {
        type Item = u32;
        type Error = String;

        fn poll(&mut self) -> futures::Poll<Self::Item, Self::Error> {
            component_future::poll_future(self, Self::POLL_FNS)
        }
    }

    let cfut =
        TwoFutures::new(futures::future::ok(3), futures::future::ok(5));
    let i = run::future(cfut);
    assert_eq!(i, Ok(20));
}

#[test]
fn test_basic_stream() {
    struct TwoFutures {
        fut1: Option<
            Box<
                dyn futures::future::Future<Item = u32, Error = String>
                    + Send,
            >,
        >,
        fut2: Option<
            Box<
                dyn futures::future::Future<Item = u32, Error = String>
                    + Send,
            >,
        >,
    }

    impl TwoFutures {
        fn new<F1, F2>(fut1: F1, fut2: F2) -> Self
        where
            F1: futures::future::Future<Item = u32, Error = String>
                + Send
                + 'static,
            F2: futures::future::Future<Item = u32, Error = String>
                + Send
                + 'static,
        {
            Self {
                fut1: Some(Box::new(fut1)),
                fut2: Some(Box::new(fut2)),
            }
        }
    }

    impl TwoFutures {
        #[allow(clippy::type_complexity)]
        const POLL_FNS:
            &'static [&'static dyn for<'a> Fn(
                &'a mut Self,
            )
                -> component_future::Poll<
                Option<u32>,
                String,
            >] = &[
            &Self::poll_future_1,
            &Self::poll_future_2,
            &Self::poll_return,
        ];

        fn poll_future_1(
            &mut self,
        ) -> component_future::Poll<Option<u32>, String> {
            if let Some(fut1) = &mut self.fut1 {
                let val = component_future::try_ready!(fut1.poll());
                self.fut1.take();
                Ok(component_future::Async::Ready(Some(val)))
            } else {
                Ok(component_future::Async::NothingToDo)
            }
        }

        fn poll_future_2(
            &mut self,
        ) -> component_future::Poll<Option<u32>, String> {
            if self.fut1.is_some() {
                return Ok(component_future::Async::NothingToDo);
            }

            if let Some(fut2) = &mut self.fut2 {
                let val = component_future::try_ready!(fut2.poll());
                self.fut2.take();
                Ok(component_future::Async::Ready(Some(val)))
            } else {
                Ok(component_future::Async::NothingToDo)
            }
        }

        fn poll_return(
            &mut self,
        ) -> component_future::Poll<Option<u32>, String> {
            if self.fut1.is_some() || self.fut2.is_some() {
                return Ok(component_future::Async::NothingToDo);
            }

            Ok(component_future::Async::Ready(None))
        }
    }

    impl futures::stream::Stream for TwoFutures {
        type Item = u32;
        type Error = String;

        fn poll(&mut self) -> futures::Poll<Option<Self::Item>, Self::Error> {
            component_future::poll_stream(self, Self::POLL_FNS)
        }
    }
    let cstream =
        TwoFutures::new(futures::future::ok(3), futures::future::ok(5));
    let is = run::stream(cstream);
    assert_eq!(is, Ok(vec![3, 5]));
}
