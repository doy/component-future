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
    Reading,
    Processing(
        Box<
            dyn futures::future::Future<Item = OutputEvent, Error = String>
                + Send,
        >,
    ),
}

struct Stream {
    input: Box<
        dyn futures::stream::Stream<Item = InputEvent, Error = String> + Send,
    >,
    state: State,
}

impl Stream {
    fn new(
        input: Box<
            dyn futures::stream::Stream<Item = InputEvent, Error = String>
                + Send,
        >,
    ) -> Self {
        Self {
            input,
            state: State::Reading,
        }
    }
}

impl Stream {
    #[allow(clippy::type_complexity)]
    const POLL_FNS:
        &'static [&'static dyn for<'a> Fn(
            &'a mut Self,
        )
            -> component_future::Poll<
            Option<OutputEvent>,
            String,
        >] = &[&Self::poll_input, &Self::poll_state];

    fn poll_input(
        &mut self,
    ) -> component_future::Poll<Option<OutputEvent>, String> {
        if let State::Processing(..) = self.state {
            return Ok(component_future::Async::NothingToDo);
        }

        if let Some(input_event) =
            component_future::try_ready!(self.input.poll())
        {
            self.state =
                State::Processing(Box::new(input_event.into_output_event()));
            Ok(component_future::Async::DidWork)
        } else {
            Ok(component_future::Async::Ready(None))
        }
    }

    fn poll_state(
        &mut self,
    ) -> component_future::Poll<Option<OutputEvent>, String> {
        if let State::Processing(fut) = &mut self.state {
            let output_event = component_future::try_ready!(fut.poll());
            self.state = State::Reading;
            Ok(component_future::Async::Ready(Some(output_event)))
        } else {
            Ok(component_future::Async::NothingToDo)
        }
    }
}

impl futures::stream::Stream for Stream {
    type Item = OutputEvent;
    type Error = String;

    fn poll(&mut self) -> futures::Poll<Option<Self::Item>, Self::Error> {
        component_future::poll_stream(self, Self::POLL_FNS)
    }
}

#[test]
fn test_processing() {
    let vals = vec![5, 3, 1, 35];
    let stream = Stream::new(Box::new(futures::stream::iter_ok(
        vals.clone().into_iter().map(InputEvent),
    )));
    let events = run::stream(stream);
    assert_eq!(
        events,
        Ok(vals.clone().into_iter().map(OutputEvent).collect())
    )
}
