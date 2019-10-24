// this fails because once there are no more events actively being processed,
// the stream doesn't have anything else to do (so it can't return Ready), but
// also no underlying future or stream has returned NotReady (so it can't
// return NotReady), so it has no valid action to take.

mod run;

#[derive(Debug, PartialEq, Eq)]
struct InputEvent(u32);
#[derive(Debug, PartialEq, Eq)]
struct OutputEvent(u32);

impl InputEvent {
    fn into_output_event(
        self,
    ) -> impl futures::future::Future<Item = OutputEvent, Error = String>
    {
        let InputEvent(i) = self;
        futures::future::ok(OutputEvent(i))
    }
}

enum State {
    Waiting,
    Processing(
        Box<
            dyn futures::future::Future<Item = OutputEvent, Error = String>
                + Send,
        >,
    ),
}

struct IdleStream {
    state: State,
}

impl IdleStream {
    fn new() -> Self {
        Self {
            state: State::Waiting,
        }
    }

    fn process(&mut self, event: InputEvent) {
        self.state = State::Processing(Box::new(event.into_output_event()));
    }
}

impl IdleStream {
    #[allow(clippy::type_complexity)]
    const POLL_FNS:
        &'static [&'static dyn for<'a> Fn(
            &'a mut Self,
        )
            -> component_future::Poll<
            Option<OutputEvent>,
            String,
        >] = &[&Self::poll_state];

    fn poll_state(
        &mut self,
    ) -> component_future::Poll<Option<OutputEvent>, String> {
        if let State::Processing(fut) = &mut self.state {
            let output_event = component_future::try_ready!(fut.poll());
            self.state = State::Waiting;
            Ok(component_future::Async::Ready(Some(output_event)))
        } else {
            Ok(component_future::Async::NothingToDo)
        }
    }
}

impl futures::stream::Stream for IdleStream {
    type Item = OutputEvent;
    type Error = String;

    fn poll(&mut self) -> futures::Poll<Option<Self::Item>, Self::Error> {
        component_future::poll_stream(self, Self::POLL_FNS)
    }
}

#[test]
#[should_panic]
fn test_panic() {
    let mut stream = IdleStream::new();
    stream.process(InputEvent(1));
    let _ = run::stream(stream);
}
