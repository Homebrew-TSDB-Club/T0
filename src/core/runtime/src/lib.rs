#![feature(once_cell)]
#![feature(cell_update)]
#![feature(can_vector)]

mod error;
mod executor;
pub mod io;

pub use error::RuntimeError;
pub use executor::spawn;
pub use futures_lite::future::yield_now;
pub use futures_lite::stream::StreamExt;
pub use futures_timer::Delay;

use async_channel::{Receiver, Sender};
use core_affinity::CoreId;
use executor::Executor;
use futures_lite::future;
use polling::Poller;
use std::future::Future;
use std::sync::Arc;
use std::thread;
use std::thread::JoinHandle;

#[derive(Debug)]
pub struct Runtime<T> {
    cores: Vec<CoreId>,
    executors: Vec<ExecutorHandler<T>>,
}

impl<T: Send + 'static> Runtime<T> {
    pub fn new(cores: &[usize]) -> Result<Self, RuntimeError> {
        let mut my_cores = Vec::new();
        let core_ids = core_affinity::get_core_ids().unwrap();
        for &core in cores {
            if core >= core_ids.len() {
                return Err(RuntimeError::NoMuchCore {
                    require: core,
                    has: core_ids.len(),
                });
            }
            my_cores.push(CoreId { id: core });
        }
        Ok(Self {
            cores: my_cores,
            executors: Vec::new(),
        })
    }

    pub fn run<F, G>(&mut self, f: F)
    where
        F: FnOnce(usize, Receiver<T>) -> G,
        F: Send + Clone + 'static,
        G: Future<Output = ()>,
    {
        for &id in &self.cores {
            let f = f.clone();
            let poller = Arc::new(Poller::new().unwrap());
            let poller_ex = Arc::clone(&poller);
            let (sender, recv) = async_channel::bounded(256);
            let join = thread::spawn(move || {
                core_affinity::set_for_current(id);
                let fut = f(id.id, recv);
                let mut ex = Executor::new(poller_ex);
                future::block_on(ex.run(async move {
                    fut.await;
                }));
            });

            self.executors.push(ExecutorHandler {
                join,
                sender,
                poller,
            });
        }
    }

    pub async fn send(&self, id: usize, req: T) -> Result<(), async_channel::SendError<T>> {
        let ex = self.executors.get(id).unwrap();
        let result = ex.sender.send(req).await;
        ex.poller.notify().unwrap();
        result
    }
}

impl<T> Drop for Runtime<T> {
    fn drop(&mut self) {
        for ex in self.executors.drain(..) {
            ex.sender.close();
            let _ = ex.poller.notify();
            ex.join.join().unwrap();
        }
    }
}

#[derive(Debug)]
struct ExecutorHandler<T> {
    join: JoinHandle<()>,
    sender: Sender<T>,
    poller: Arc<Poller>,
}

#[cfg(test)]
mod test {
    use crate::{spawn, Delay, Runtime};
    use async_channel::Receiver;
    use std::cell::RefCell;
    use std::rc::Rc;
    use std::time::{Duration, SystemTime};

    #[test]
    fn test_runtime() {
        let cores = (0..1).collect::<Vec<_>>();
        let mut runtime = Runtime::new(&cores).unwrap();
        runtime.run(|_, _: Receiver<()>| async move {
            let printable = Rc::new(RefCell::new(1));
            let p_clone = Rc::clone(&printable);
            spawn(async move {
                *p_clone.borrow_mut() = 2;
            })
            .await;
        });
    }

    #[test]
    fn test_sleep() {
        let cores = (0..1).collect::<Vec<_>>();
        let mut runtime = Runtime::new(&cores).unwrap();
        runtime.run(|_, _: Receiver<()>| async move {
            spawn(async move {
                let start_at = SystemTime::now();
                Delay::new(Duration::from_millis(10)).await;
                assert!(
                    SystemTime::now().duration_since(start_at).unwrap()
                        >= Duration::from_millis(10)
                );
                Delay::new(Duration::from_millis(10)).await;
                assert!(
                    SystemTime::now().duration_since(start_at).unwrap()
                        >= Duration::from_millis(10)
                );
            })
            .await;
        });
    }
}
