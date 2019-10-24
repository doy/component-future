//! This crate implements the inner future protocol documented in [the tokio
//! docs](https://tokio.rs/docs/futures/getting_asynchronous/).
//!
//! # Overview
//!
//! If you are implementing a complicated future or stream which contains many
//! inner futures and streams, it can be difficult to keep track of when
//! looping is necessary, when it is safe to return `NotReady`, etc. This
//! provides an interface similar to the existing `poll` interface for futures
//! and streams, but extended to include additional state about how the inner
//! future or stream affected the outer future or stream's state. This allows
//! you to easily split your `poll` implementation into multiple methods, and
//! ensure that they interact properly.
//!
//! # Synopsis
//!
//! ```
//! enum OutputEvent {
//!     // ...
//! }
//!
//! struct Server {
//!     // ...
//! #   some_future: Box<
//! #       dyn futures::future::Future<Item = OutputEvent, Error = String>
//! #           + Send,
//! #   >,
//! #   other_future: Option<
//! #       Box<
//! #           dyn futures::future::Future<Item = OutputEvent, Error = String>
//! #               + Send,
//! #       >,
//! #   >,
//! }
//!
//! impl Server {
//!     fn process_thing(&self, thing: OutputEvent) {
//!         // ...
//!     }
//!
//!     fn process_other_thing(&self, thing: OutputEvent) -> OutputEvent {
//!         // ...
//! #       unimplemented!()
//!     }
//! }
//!
//! impl Server {
//!     const POLL_FNS:
//!         &'static [&'static dyn for<'a> Fn(
//!             &'a mut Self,
//!         )
//!             -> component_future::Poll<
//!             Option<OutputEvent>,
//!             String,
//!         >] = &[&Self::poll_thing, &Self::poll_other_thing];
//!
//!     fn poll_thing(
//!         &mut self,
//!     ) -> component_future::Poll<Option<OutputEvent>, String> {
//!         let thing = component_future::try_ready!(self.some_future.poll());
//!         self.process_thing(thing);
//!         Ok(component_future::Async::DidWork)
//!     }
//!
//!     fn poll_other_thing(
//!         &mut self,
//!     ) -> component_future::Poll<Option<OutputEvent>, String> {
//!         if let Some(other_future) = &mut self.other_future {
//!             let other_thing = component_future::try_ready!(
//!                 other_future.poll()
//!             );
//!             let processed_thing = self.process_other_thing(other_thing);
//!             self.other_future.take();
//!             Ok(component_future::Async::Ready(Some(processed_thing)))
//!         }
//!         else {
//!             Ok(component_future::Async::NothingToDo)
//!         }
//!     }
//! }
//!
//! impl futures::stream::Stream for Server {
//!     type Item = OutputEvent;
//!     type Error = String;
//!
//!     fn poll(&mut self) -> futures::Poll<Option<Self::Item>, Self::Error> {
//!         component_future::poll_stream(self, Self::POLL_FNS)
//!     }
//! }
//! ```

// XXX this is broken with ale
// #![warn(clippy::cargo)]
#![warn(clippy::pedantic)]
#![warn(clippy::nursery)]
#![allow(clippy::multiple_crate_versions)]
#![allow(clippy::type_complexity)]

const _DUMMY_DEPENDENCY: &str = include_str!("../Cargo.toml");

/// Return type of a component of a future or stream, indicating whether a
/// value is ready, or if not, what actions were taken.
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum Async<Item> {
    /// We have a value for the main loop to return immediately.
    Ready(Item),

    /// One of our inner futures returned `futures::Async::NotReady`. If all
    /// of our other components return either `NothingToDo` or `NotReady`,
    /// then our overall future should return `NotReady` and wait to be polled
    /// again.
    NotReady,

    /// We did some work (moved our internal state closer to being ready to
    /// return a value), but we aren't ready to return a value yet. We should
    /// re-run all of the poll functions to see if the state modification made
    /// any of them also able to make progress.
    DidWork,

    /// We didn't poll any inner futures or otherwise change our internal
    /// state at all, so rerunning is unlikely to make progress. If all
    /// components return either `NothingToDo` or `NotReady` (and at least one
    /// returns `NotReady`), then we should just return `NotReady` and wait to
    /// be polled again. It is an error (panic) for all component poll methods
    /// to return `NothingToDo`.
    NothingToDo,
}

/// Each component poll method should return a value of this type.
///
/// * `Ok(Async::Ready(t))` means that the overall future or stream is ready
///   to return a value.
/// * `Ok(Async::NotReady)` means that a poll method called by one of the
///   component futures or streams returned `NotReady`, and so it's safe for
///   the overall future or stream to also return `NotReady`.
/// * `Ok(Async::DidWork)` means that the overall future made progress by
///   updating its internal state, but isn't yet ready to return a value.
/// * `Ok(Async::NothingToDo)` means that no work was done at all.
/// * `Err(e)` means that the overall future or stream is ready to return an
///   error.
pub type Poll<Item, Error> = Result<Async<Item>, Error>;

/// A macro for extracting the successful type of a `futures::Poll<T, E>` and
/// turning it into a `component_future::Poll<T, E>`.
///
/// This macro propagates both errors and `NotReady` values by returning
/// early.
#[macro_export]
macro_rules! try_ready {
    ($e:expr) => {
        match $e {
            Ok(futures::Async::Ready(t)) => t,
            Ok(futures::Async::NotReady) => {
                return Ok($crate::Async::NotReady)
            }
            Err(e) => return Err(From::from(e)),
        }
    };
}

/// The body of a `futures::future::Future::poll` method.
///
/// It will repeatedly call the given component poll functions until none of
/// them returns `Ok(Async::Ready(t))`, `Ok(Async::DidWork)`, or `Err(e)` and
/// at least one of them returns `Ok(Async::NotReady)`.
///
/// # Panics
///
/// Panics if all component poll methods return `Ok(Async::NothingToDo)`.
///
/// # Examples
///
/// ```
/// # use futures::future::Future;
/// # struct Foo;
/// # impl Foo {
/// #     const POLL_FNS:
/// #         &'static [&'static dyn for<'a> Fn(
/// #             &'a mut Self,
/// #         ) -> component_future::Poll<(), ()>] = &[];
/// # }
/// impl Future for Foo {
///     type Item = ();
///     type Error = ();
///
///     fn poll(&mut self) -> futures::Poll<Self::Item, Self::Error> {
///         component_future::poll_future(self, Self::POLL_FNS)
///     }
/// }
/// ```
pub fn poll_future<'a, T, Item, Error>(
    future: &mut T,
    poll_fns: &'a [&'a dyn for<'b> Fn(&'b mut T) -> Poll<Item, Error>],
) -> futures::Poll<Item, Error>
where
    T: futures::future::Future<Item = Item, Error = Error>,
{
    loop {
        let mut not_ready = false;
        let mut did_work = false;

        for f in poll_fns {
            match f(future)? {
                Async::Ready(e) => return Ok(futures::Async::Ready(e)),
                Async::NotReady => not_ready = true,
                Async::NothingToDo => {}
                Async::DidWork => did_work = true,
            }
        }

        if !did_work {
            if not_ready {
                return Ok(futures::Async::NotReady);
            } else {
                unreachable!()
            }
        }
    }
}

/// The body of a `futures::stream::Stream::poll` method.
///
/// It will repeatedly call the given component poll functions until none of
/// them returns `Ok(Async::Ready(t))`, `Ok(Async::DidWork)`, or `Err(e)` and
/// at least one of them returns `Ok(Async::NotReady)`.
///
/// # Panics
///
/// Panics if all component poll methods return `Ok(Async::NothingToDo)`.
///
/// # Examples
///
/// ```
/// # use futures::stream::Stream;
/// # struct Foo;
/// # impl Foo {
/// #     const POLL_FNS:
/// #         &'static [&'static dyn for<'a> Fn(
/// #             &'a mut Self,
/// #         ) -> component_future::Poll<Option<()>, ()>] = &[];
/// # }
/// impl Stream for Foo {
///     type Item = ();
///     type Error = ();
///
///     fn poll(&mut self) -> futures::Poll<Option<Self::Item>, Self::Error> {
///         component_future::poll_stream(self, Self::POLL_FNS)
///     }
/// }
/// ```
pub fn poll_stream<'a, T, Item, Error>(
    stream: &mut T,
    poll_fns: &'a [&'a dyn for<'b> Fn(
        &'b mut T,
    ) -> Poll<Option<Item>, Error>],
) -> futures::Poll<Option<Item>, Error>
where
    T: futures::stream::Stream<Item = Item, Error = Error>,
{
    loop {
        let mut not_ready = false;
        let mut did_work = false;

        for f in poll_fns {
            match f(stream)? {
                Async::Ready(e) => return Ok(futures::Async::Ready(e)),
                Async::NotReady => not_ready = true,
                Async::NothingToDo => {}
                Async::DidWork => did_work = true,
            }
        }

        if !did_work {
            if not_ready {
                return Ok(futures::Async::NotReady);
            } else {
                unreachable!()
            }
        }
    }
}
