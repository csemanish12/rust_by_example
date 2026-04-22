// ============================================================================
//  lib.rs — The Thread Pool Implementation
//
//  Three structs work together:
//    1. ThreadPool  — the public API; holds workers + the channel sender
//    2. Worker      — wraps a real OS thread; loops forever waiting for jobs
//    3. Job         — a type alias for any boxed, sendable, one-shot closure
// ============================================================================

use std::sync::{Arc, Mutex, mpsc};
use std::thread;

// ----------------------------------------------------------------------------
// Job — "any function we can send to a worker thread and run once"
//
// Box<dyn FnOnce()>  — heap-allocated closure, callable once
// + Send             — safe to move across thread boundary
// + 'static          — contains no short-lived borrows
// ----------------------------------------------------------------------------
type Job = Box<dyn FnOnce() + Send + 'static>;

// ----------------------------------------------------------------------------
// Worker — one OS thread that sits in a loop waiting for jobs
//
// Python equivalent:
//   class Worker(threading.Thread):
//       def run(self):
//           while True:
//               job = self.queue.get()
//               job()
// ----------------------------------------------------------------------------
struct Worker {
    id: usize,
    thread: Option<thread::JoinHandle<()>>,
    //      ──────                          ← Option so Drop can take() and join() it
}

impl Worker {
    fn new(id: usize, receiver: Arc<Mutex<mpsc::Receiver<Job>>>) -> Worker {
        //                        ─┬─  ─────┬──── ─────────┬──
        //                         │         │              └── the channel receiver end
        //                         │         └─────────────── Mutex: only one worker reads at a time
        //                         └───────────────────────── Arc: shared ownership across all workers

        let thread = thread::spawn(move || {
            // This loop runs FOREVER inside the worker thread.
            // The worker blocks on recv() until a job arrives, runs it, then loops back.
            loop {
                // Step 1: Lock the Mutex to get exclusive access to the receiver
                // Step 2: Call recv() — BLOCKS here until a job is sent down the channel
                // Step 3: Release the Mutex lock (lock dropped at end of let statement)
                let message = receiver
                    .lock()        // acquire Mutex lock  (like Python's queue.mutex.acquire())
                    .unwrap()      // unwrap lock result  (panics if another thread panicked)
                    .recv();       // block until a Job arrives (like queue.Queue.get())

                match message {
                    Ok(job) => {
                        println!("[Worker {id}] Received a job. Executing...");
                        job(); // run the closure — this is the actual request handler
                    }
                    Err(_) => {
                        // The sender was dropped — ThreadPool is shutting down.
                        // Break the loop so the thread exits cleanly.
                        println!("[Worker {id}] Sender disconnected. Shutting down.");
                        break;
                    }
                }
            }
        });

        Worker { id, thread: Some(thread) }
    }
}

// ----------------------------------------------------------------------------
// ThreadPool — the public-facing API
//
// Python equivalent:
//   pool = ThreadPoolExecutor(max_workers=4)
//   pool.submit(handle_connection, stream)
// ----------------------------------------------------------------------------
pub struct ThreadPool {
    workers: Vec<Worker>,
    sender: Option<mpsc::Sender<Job>>,
    //      ──────                     ← Option so we can take() it during Drop (graceful shutdown)
}

impl ThreadPool {
    /// Create a new ThreadPool with `size` worker threads.
    ///
    /// # Panics
    /// Panics if size is 0.
    pub fn new(size: usize) -> ThreadPool {
        assert!(size > 0, "ThreadPool size must be greater than 0");

        // Create the channel — one sender, one receiver
        // The sender stays in ThreadPool; the receiver is shared between all workers
        let (sender, receiver) = mpsc::channel::<Job>();

        // Wrap receiver in Arc<Mutex<...>> so all workers can safely share it
        //
        // Arc  — allows multiple workers to each hold a reference-counted pointer
        // Mutex — ensures only one worker calls recv() at a time
        let receiver = Arc::new(Mutex::new(receiver));

        // Pre-allocate the workers vector
        let mut workers = Vec::with_capacity(size);

        for id in 0..size {
            // Clone the Arc for each worker — cheap! Just increments the reference count.
            // Each worker gets its own Arc pointer, all pointing to the SAME receiver.
            workers.push(Worker::new(id, Arc::clone(&receiver)));
        }

        ThreadPool {
            workers,
            sender: Some(sender),
        }
    }

    /// Send a job to the thread pool to be executed by the next free worker.
    ///
    /// Python equivalent:
    ///   pool.submit(f)
    pub fn execute<F>(&self, f: F)
    where
        F: FnOnce() + Send + 'static,
        // ────────────────────────────
        // FnOnce()  — callable once (our handler consumes the stream)
        // Send      — can be moved to another thread
        // 'static   — no short-lived borrows (thread might outlive the caller)
    {
        let job = Box::new(f); // box the closure so it has a fixed, known size

        self.sender
            .as_ref()
            .unwrap()
            .send(job) // send down the channel — one of the waiting workers will pick it up
            .unwrap();
    }
}

// ----------------------------------------------------------------------------
// Drop — graceful shutdown when ThreadPool goes out of scope
//
// Python equivalent:
//   with ThreadPoolExecutor() as pool:  ← __exit__ calls shutdown(wait=True)
//       ...
// ----------------------------------------------------------------------------
impl Drop for ThreadPool {
    fn drop(&mut self) {
        // Drop the sender first — this closes the channel.
        // All workers currently blocking on recv() will get an Err() and break their loop.
        drop(self.sender.take());

        // Now join every worker thread — wait for each one to finish its current job cleanly.
        for worker in &mut self.workers {
            println!("[ThreadPool] Shutting down Worker {}", worker.id);

            // take() replaces the JoinHandle with None so we can call join() (which consumes it)
            if let Some(thread) = worker.thread.take() {
                thread.join().unwrap();
            }
        }
    }
}
