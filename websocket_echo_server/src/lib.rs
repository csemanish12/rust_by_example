// ============================================================================
//  lib.rs — Thread Pool (reused from multithreaded_server project)
//
//  Exactly the same ThreadPool we built in Step 5 of multithreaded_server.
//  This shows a key Rust principle: code is modular and reusable.
//
//  Three pieces:
//    1. Job        — type alias: any boxed, sendable, one-shot closure
//    2. Worker     — one OS thread looping forever, waiting for jobs
//    3. ThreadPool — public API: new(size) + execute(f)
// ============================================================================

use std::sync::{mpsc, Arc, Mutex};
use std::thread;

// A Job is any closure we can send to a worker thread and run once.
// Box<dyn FnOnce() + Send + 'static>
//     ───              ────   ──────
//     heap ptr         thread-  no short-lived
//     (fixed size)     safe     borrows
type Job = Box<dyn FnOnce() + Send + 'static>;

// ----------------------------------------------------------------------------
// Worker — wraps one OS thread that loops forever on recv()
// ----------------------------------------------------------------------------
struct Worker {
    id: usize,
    thread: Option<thread::JoinHandle<()>>, // Option so we can take() it in Drop
}

impl Worker {
    fn new(id: usize, receiver: Arc<Mutex<mpsc::Receiver<Job>>>) -> Worker {
        let thread = thread::spawn(move || loop {
            let message = receiver
                .lock()   // acquire Mutex — only this worker reads right now
                .unwrap() // unwrap poisoned lock (another thread panicked)
                .recv();  // BLOCK until a Job arrives, then release lock

            match message {
                Ok(job) => {
                    println!("[Worker {id}] picked up a WebSocket client.");
                    job(); // run the closure — handles one WebSocket client
                }
                Err(_) => {
                    println!("[Worker {id}] shutting down.");
                    break; // sender dropped → pool shutting down → exit loop
                }
            }
        });

        Worker {
            id,
            thread: Some(thread),
        }
    }
}

// ----------------------------------------------------------------------------
// ThreadPool — the public API
// ----------------------------------------------------------------------------
pub struct ThreadPool {
    workers: Vec<Worker>,
    sender: Option<mpsc::Sender<Job>>, // Option so we can drop it in Drop
}

impl ThreadPool {
    /// Create a pool of `size` worker threads.
    /// Panics if size == 0.
    pub fn new(size: usize) -> ThreadPool {
        assert!(size > 0, "ThreadPool size must be > 0");

        let (sender, receiver) = mpsc::channel::<Job>();
        let receiver = Arc::new(Mutex::new(receiver)); // shared, locked receiver

        let mut workers = Vec::with_capacity(size);
        for id in 0..size {
            workers.push(Worker::new(id, Arc::clone(&receiver)));
        }

        ThreadPool {
            workers,
            sender: Some(sender),
        }
    }

    /// Send a job to the pool — a free worker will pick it up.
    pub fn execute<F>(&self, f: F)
    where
        F: FnOnce() + Send + 'static,
    {
        let job = Box::new(f);
        self.sender.as_ref().unwrap().send(job).unwrap();
    }
}

// ----------------------------------------------------------------------------
// Drop — graceful shutdown: finish in-flight clients, then stop workers
// ----------------------------------------------------------------------------
impl Drop for ThreadPool {
    fn drop(&mut self) {
        drop(self.sender.take()); // close the channel → workers get Err on recv()
        for worker in &mut self.workers {
            if let Some(thread) = worker.thread.take() {
                thread.join().unwrap(); // wait for each worker to finish cleanly
            }
        }
    }
}
