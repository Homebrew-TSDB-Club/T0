use async_task::Runnable;
use async_task::Task;
use concurrent_queue::ConcurrentQueue;
use futures_lite::future::{self, yield_now};
use polling::{Event, Poller};
use std::lazy::OnceCell;
use std::marker::PhantomData;
use std::os::unix::io::AsRawFd;
use std::task::Waker;
use std::{cell::RefCell, sync::Arc};
use std::{collections::VecDeque, time::Duration};
use std::{future::Future, mem::replace};

const NR_TASKS: usize = 256;

thread_local! {
    pub(crate) static CONTEXT: OnceCell<Context> = OnceCell::new()
}

#[derive(Debug)]
pub(crate) struct Context {
    pub(crate) runnable: Arc<ConcurrentQueue<Runnable>>,
    pub(crate) polling: RefCell<Polling>,
}

impl Context {
    fn new(poller: Arc<Poller>) -> Self {
        Context {
            runnable: Arc::new(ConcurrentQueue::unbounded()),
            polling: RefCell::new(Polling::new(poller)),
        }
    }
}

#[derive(Debug)]
pub struct Executor<'a> {
    _marker: PhantomData<std::cell::UnsafeCell<&'a ()>>,
}

impl<'a> Executor<'a> {
    /// Creates a new executor.
    pub fn new(poller: Arc<Poller>) -> Self {
        CONTEXT.with(|context| context.set(Context::new(poller)).unwrap());
        Executor {
            _marker: PhantomData,
        }
    }

    pub async fn run(&mut self, future: impl Future<Output = ()>) {
        // A future that runs tasks forever.
        let run_forever = async move {
            let mut events = Vec::with_capacity(NR_TASKS);
            loop {
                CONTEXT.with(|context| {
                    let context = context.get().unwrap();
                    for _ in 0..NR_TASKS {
                        if let Ok(runnable) = context.runnable.pop() {
                            runnable.run();
                        } else {
                            break;
                        }
                    }
                });

                yield_now().await;

                CONTEXT.with(|context| {
                    let context = context.get().unwrap();
                    let duration = if context.runnable.is_empty() {
                        None
                    } else {
                        Some(Duration::ZERO)
                    };
                    context.polling.borrow_mut().wait(&mut events, duration);
                    events.clear();
                });
            }
        };

        let future = spawn(future);
        // Run `future` and `run_forever` concurrently until `future` completes.
        future::or(future, run_forever).await;
    }
}

#[derive(Debug)]
pub(crate) struct Polling {
    poller: Arc<Poller>,
    wakers: Vec<Option<Waker>>,
    recycle_ids: VecDeque<usize>,
    id: usize,
}

impl Polling {
    pub(crate) fn new(poller: Arc<Poller>) -> Self {
        Polling {
            poller,
            wakers: Vec::new(),
            recycle_ids: VecDeque::new(),
            id: 0,
        }
    }

    pub(crate) fn add<T: AsRawFd>(&mut self, fd: &T) -> usize {
        let id = if let Some(id) = self.recycle_ids.pop_front() {
            id
        } else {
            let id = self.id;
            self.id += 1;
            self.wakers.push(None);
            id
        };
        self.poller.add(fd, Event::none(id)).unwrap();
        self.poller.notify().unwrap();
        id
    }

    pub(crate) fn modify<T: AsRawFd, G: Fn(usize) -> Event>(
        &mut self,
        id: usize,
        fd: &T,
        event: G,
        waker: Waker,
    ) {
        self.wakers[id] = Some(waker);
        self.poller.modify(fd, event(id)).unwrap();
    }

    fn wait(&mut self, events: &mut Vec<Event>, timeout: Option<Duration>) {
        self.poller.wait(events, timeout).unwrap();
        for event in events {
            if let Some(waker) = replace(&mut self.wakers[event.key], None) {
                waker.wake();
            }
        }
    }

    pub(crate) fn delete<T: AsRawFd>(&mut self, fd: &T, id: usize) {
        self.poller.delete(fd).unwrap();
        self.wakers[id] = None;
        self.recycle_ids.push_back(id);
    }
}

pub fn spawn<T>(future: impl Future<Output = T>) -> Task<T> {
    let (runnable, task) = unsafe { async_task::spawn_unchecked(future, schedule()) };
    runnable.schedule();
    task
}

/// Returns a function that schedules a runnable task when it gets woken up.
fn schedule() -> impl Fn(Runnable) {
    CONTEXT.with(|context| {
        let context = context.get().unwrap();
        let queue = Arc::clone(&context.runnable);
        let poller = Arc::clone(&context.polling.borrow().poller);
        move |runnable| {
            queue.push(runnable).unwrap();
            poller.notify().unwrap();
        }
    })
}

#[cfg(test)]
mod test {
    use super::{spawn, Executor};
    use futures_lite::future;
    use futures_lite::future::yield_now;
    use polling::Poller;
    use std::rc::Rc;
    use std::{cell::RefCell, sync::Arc};

    #[test]
    fn test_runtime() {
        let mut ex = Executor::new(Arc::new(Poller::new().unwrap()));

        let task = spawn(async { 1 + 2 });
        future::block_on(ex.run(async {
            let res = task.await * 2;
            assert_eq!(res, 6);
        }));
    }

    #[test]
    fn test_yield() {
        let mut ex = Executor::new(Arc::new(Poller::new().unwrap()));

        let counter = Rc::new(RefCell::new(0));
        let counter1 = Rc::clone(&counter);
        let task = spawn(async move {
            {
                let mut c = counter1.borrow_mut();
                assert_eq!(*c, 0);
                *c = 1;
            }
            let counter_clone = Rc::clone(&counter1);
            let t = spawn(async move {
                {
                    let mut c = counter_clone.borrow_mut();
                    assert_eq!(*c, 1);
                    *c = 2;
                }
                yield_now().await;
                {
                    let mut c = counter_clone.borrow_mut();
                    assert_eq!(*c, 3);
                    *c = 4;
                }
            });
            yield_now().await;
            {
                let mut c = counter1.borrow_mut();
                assert_eq!(*c, 2);
                *c = 3;
            }
            t.await;
        });
        future::block_on(ex.run(task));
        assert_eq!(*counter.as_ref().borrow(), 4);
    }
}
