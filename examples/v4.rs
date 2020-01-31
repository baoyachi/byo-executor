use std::future::Future;
use std::panic::catch_unwind;
use std::pin::Pin;
use std::task::{Context, Poll};
use std::thread;

use crossbeam::channel;
use once_cell::sync::Lazy;

/// A queue that holds scheduled tasks.
static QUEUE: Lazy<channel::Sender<Task>> = Lazy::new(|| {
    // Create a queue.
    let (sender, receiver) = channel::unbounded::<Task>();

    // Spawn executor threads the first time the queue is created.
    for _ in 0..num_cpus::get().max(1) {
        let receiver = receiver.clone();
        thread::spawn(move || {
            receiver.iter().for_each(|task| {
                let _ = catch_unwind(|| task.run());
            })
        });
    }

    sender
});

/// A spawned future and its current state.
type Task = async_task::Task<()>;

/// Spawns a future on the executor.
fn spawn<F, R>(future: F) -> JoinHandle<R>
where
    F: Future<Output = R> + Send + 'static,
    R: Send + 'static,
{
    // Create a task and schedule it for execution.
    let (task, handle) = async_task::spawn(future, |t| QUEUE.send(t).unwrap(), ());
    task.schedule();

    // Return a join handle that retrieves the output of the future.
    JoinHandle(handle)
}

/// Awaits the output of a spawned future.
struct JoinHandle<R>(async_task::JoinHandle<R, ()>);

impl<R> Future for JoinHandle<R> {
    type Output = R;

    fn poll(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        match Pin::new(&mut self.0).poll(cx) {
            Poll::Pending => Poll::Pending,
            Poll::Ready(output) => Poll::Ready(output.expect("task failed")),
        }
    }
}

fn main() {
    futures::executor::block_on(async {
        // Spawn a future.
        let handle = spawn(async {
            println!("Running task...");
            panic!();
        });

        // Await its output.
        handle.await;
    });
}
