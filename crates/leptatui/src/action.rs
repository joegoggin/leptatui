//! Signal-backed async action state for terminal apps.
//!
//! Actions run POST-like asynchronous writes and expose the latest pending,
//! input, and result state as a signal-friendly value.

use std::{
    future::Future,
    pin::Pin,
    sync::{
        Arc,
        atomic::{AtomicU64, Ordering},
    },
};

use leptos::prelude::{
    Get, GetUntracked, ReadSignal, Set, With, WithUntracked, WriteSignal, signal,
};

type BoxActionFuture<O, E> = Pin<Box<dyn Future<Output = std::result::Result<O, E>> + Send>>;
type ActionHandler<I, O, E> = Arc<dyn Fn(I) -> BoxActionFuture<O, E> + Send + Sync>;

/// Current state for an asynchronous mutation action.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ActionState<I, O, E> {
    pending: bool,
    input: Option<I>,
    result: Option<std::result::Result<O, E>>,
}

impl<I, O, E> ActionState<I, O, E> {
    /// Returns an idle action state with no input or result.
    pub fn idle() -> Self {
        Self {
            pending: false,
            input: None,
            result: None,
        }
    }

    /// Returns a pending action state for a dispatched input.
    pub fn pending(input: I) -> Self {
        Self {
            pending: true,
            input: Some(input),
            result: None,
        }
    }

    /// Returns a completed action state for a dispatched input and result.
    pub fn completed(input: I, result: std::result::Result<O, E>) -> Self {
        Self {
            pending: false,
            input: Some(input),
            result: Some(result),
        }
    }

    /// Returns whether the action is currently pending.
    pub fn is_pending(&self) -> bool {
        self.pending
    }

    /// Returns the latest dispatched input by reference.
    pub fn input(&self) -> Option<&I> {
        self.input.as_ref()
    }

    /// Returns the latest action result by reference.
    pub fn result(&self) -> Option<&std::result::Result<O, E>> {
        self.result.as_ref()
    }

    /// Returns the successful action output by reference.
    pub fn value(&self) -> Option<&O> {
        match self.result() {
            Some(Ok(value)) => Some(value),
            Some(Err(_)) | None => None,
        }
    }

    /// Returns the action error by reference.
    pub fn error(&self) -> Option<&E> {
        match self.result() {
            Some(Err(error)) => Some(error),
            Some(Ok(_)) | None => None,
        }
    }
}

/// Reactive handle for an async mutation.
pub struct Action<I, O, E> {
    state: ReadSignal<ActionState<I, O, E>>,
    set_state: WriteSignal<ActionState<I, O, E>>,
    handler: ActionHandler<I, O, E>,
    latest_dispatch: Arc<AtomicU64>,
}

impl<I, O, E> Clone for Action<I, O, E> {
    /// Clones the action signal and mutation handles.
    fn clone(&self) -> Self {
        Self {
            state: self.state,
            set_state: self.set_state,
            handler: Arc::clone(&self.handler),
            latest_dispatch: Arc::clone(&self.latest_dispatch),
        }
    }
}

impl<I, O, E> Action<I, O, E>
where
    I: Clone + Send + Sync + 'static,
    O: Send + Sync + 'static,
    E: Send + Sync + 'static,
{
    /// Creates an action from an async mutation handler.
    ///
    /// # Panics
    ///
    /// Panics if called outside a Tokio runtime, because action dispatches are
    /// scheduled onto Tokio.
    pub fn new<F, Fut>(handler: F) -> Self
    where
        F: Fn(I) -> Fut + Send + Sync + 'static,
        Fut: Future<Output = std::result::Result<O, E>> + Send + 'static,
    {
        init_tokio_executor();

        let (state, set_state) = signal(ActionState::idle());
        let handler: ActionHandler<I, O, E> =
            Arc::new(move |input| -> BoxActionFuture<O, E> { Box::pin(handler(input)) });

        Self {
            state,
            set_state,
            handler,
            latest_dispatch: Arc::new(AtomicU64::new(0)),
        }
    }

    /// Dispatches the action with an input value.
    ///
    /// The latest dispatch controls the visible action state. Older in-flight
    /// tasks are not cancelled, but their completions are ignored if a newer
    /// dispatch has started.
    pub fn dispatch(&self, input: I) {
        let dispatch_id = self.latest_dispatch.fetch_add(1, Ordering::AcqRel) + 1;
        let task_input = input.clone();
        let completion_input = input.clone();
        let _ = self.set_state.try_set(ActionState::pending(input));

        let handler = Arc::clone(&self.handler);
        let latest_dispatch = Arc::clone(&self.latest_dispatch);
        let set_state = self.set_state;

        tokio::spawn(async move {
            let result = handler(task_input).await;

            if latest_dispatch.load(Ordering::Acquire) == dispatch_id {
                let _ = set_state.try_set(ActionState::completed(completion_input, result));
            }
        });
    }

    /// Returns the read signal containing this action's state.
    pub fn state(&self) -> ReadSignal<ActionState<I, O, E>> {
        self.state
    }

    /// Reads the current action state reactively by reference.
    pub fn with<R>(&self, read: impl FnOnce(&ActionState<I, O, E>) -> R) -> R {
        self.state.with(read)
    }

    /// Reads the current action state without tracking it.
    pub fn with_untracked<R>(&self, read: impl FnOnce(&ActionState<I, O, E>) -> R) -> R {
        self.state.with_untracked(read)
    }

    /// Returns whether the action is currently pending.
    pub fn is_pending(&self) -> bool {
        self.with(ActionState::is_pending)
    }
}

impl<I, O, E> Action<I, O, E>
where
    I: Clone + Send + Sync + 'static,
    O: Clone + Send + Sync + 'static,
    E: Clone + Send + Sync + 'static,
{
    /// Returns the current action state reactively.
    pub fn get(&self) -> ActionState<I, O, E> {
        self.state.get()
    }

    /// Returns the current action state without tracking it.
    pub fn get_untracked(&self) -> ActionState<I, O, E> {
        self.state.get_untracked()
    }

    /// Returns the latest dispatched input reactively.
    pub fn input(&self) -> Option<I> {
        self.with(|state| state.input().cloned())
    }

    /// Returns the latest dispatched input without tracking it.
    pub fn input_untracked(&self) -> Option<I> {
        self.with_untracked(|state| state.input().cloned())
    }

    /// Returns the latest action result reactively.
    pub fn result(&self) -> Option<std::result::Result<O, E>> {
        self.with(|state| state.result().cloned())
    }

    /// Returns the latest action result without tracking it.
    pub fn result_untracked(&self) -> Option<std::result::Result<O, E>> {
        self.with_untracked(|state| state.result().cloned())
    }

    /// Returns the successful action output, when available.
    pub fn value(&self) -> Option<O> {
        self.with(|state| state.value().cloned())
    }

    /// Returns the action error, when available.
    pub fn error(&self) -> Option<E> {
        self.with(|state| state.error().cloned())
    }
}

/// Creates an action from an async mutation handler.
pub fn create_action<I, O, E, F, Fut>(handler: F) -> Action<I, O, E>
where
    I: Clone + Send + Sync + 'static,
    O: Send + Sync + 'static,
    E: Send + Sync + 'static,
    F: Fn(I) -> Fut + Send + Sync + 'static,
    Fut: Future<Output = std::result::Result<O, E>> + Send + 'static,
{
    Action::new(handler)
}

fn init_tokio_executor() {
    let _ = any_spawner::Executor::init_tokio();
}
