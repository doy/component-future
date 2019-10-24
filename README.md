# component-future

This crate implements the inner future protocol documented in
[the tokio docs](https://tokio.rs/docs/futures/getting_asynchronous/).

## Overview

If you are implementing a complicated future or stream which contains many
inner futures and streams, it can be difficult to keep track of when looping
is necessary, when it is safe to return `NotReady`, etc. This provides an
interface similar to the existing `poll` interface for futures and streams,
but extended to include additional state about how the inner future or stream
affected the outer future or stream's state. This allows you to easily split
your `poll` implementation into multiple methods, and ensure that they
interact properly.

## Synopsis

```rust
enum OutputEvent {
    // ...
}

struct Server {
    // ...
}

impl Server {
    fn process_thing(&self, thing: OutputEvent) {
        // ...
    }

    fn process_other_thing(&self, thing: OutputEvent) -> OutputEvent {
        // ...
    }
}

impl Server {
    const POLL_FNS:
        &'static [&'static dyn for<'a> Fn(
            &'a mut Self,
        )
            -> component_future::Poll<
            Option<OutputEvent>,
            String,
        >] = &[&Self::poll_thing, &Self::poll_other_thing];

    fn poll_thing(
        &mut self,
    ) -> component_future::Poll<Option<OutputEvent>, String> {
        let thing = component_future::try_ready!(self.some_future.poll());
        self.process_thing(thing);
        Ok(component_future::Async::DidWork)
    }

    fn poll_other_thing(
        &mut self,
    ) -> component_future::Poll<Option<OutputEvent>, String> {
        if let Some(other_future) = &mut self.other_future {
            let other_thing = component_future::try_ready!(
                other_future.poll()
            );
            let processed_thing = self.process_other_thing(other_thing);
            self.other_future.take();
            Ok(component_future::Async::Ready(Some(processed_thing)))
        }
        else {
            Ok(component_future::Async::NothingToDo)
        }
    }
}

impl futures::stream::Stream for Server {
    type Item = OutputEvent;
    type Error = String;

    fn poll(&mut self) -> futures::Poll<Option<Self::Item>, Self::Error> {
        component_future::poll_stream(self, Self::POLL_FNS)
    }
}
```
