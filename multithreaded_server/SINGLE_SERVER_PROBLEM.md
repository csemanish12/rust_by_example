Step 1 — Understanding the Problem: Why Can't Our Simple Server Handle Multiple Requests?

🐍 Think of it in Python first
Imagine a coffee shop with only one barista:

# Your current simple server behaves like this:
def run_coffee_shop():
    while True:
        customer = wait_for_customer()   # blocks here until someone walks in
        serve_customer(customer)         # serves them COMPLETELY before looking up
        # ☝️ Everyone else is waiting outside!

This is exactly what your current Rust server does:
Request 1 arrives  --> Server handles it (takes 2 seconds)
Request 2 arrives  --> WAITING...
Request 3 arrives  --> WAITING...
Request 4 arrives  --> WAITING...

🦀 Your Current Rust Server
Your simple server loop looks something like this:

// Current simple server - ONE customer at a time
for stream in listener.incoming() {
    let stream = stream.unwrap();
    handle_connection(stream); // 😴 BLOCKS here until fully done
                               // No new connections accepted until this returns!
}

🔴 The Real Problem: Blocking
Blocking means "I am frozen, waiting, doing nothing else".

Scenario	Python (with GIL)	Rust (single thread)
Request takes 5 seconds	Other requests wait 5 seconds	Other requests wait 5 seconds
100 users at once	Queued up	Queued up
One slow request	Slows everyone	Slows everyone

✅ What We Want
Request 1 arrives  --> Worker 1 handles it  ──────────────────► done
Request 2 arrives  --> Worker 2 handles it  ────────► done
Request 3 arrives  --> Worker 3 handles it  ──────────────► done
Request 4 arrives  --> Worker 1 handles it  ──► done
                       ☝️ All happening at the SAME TIME


🧠 Key Concept: Concurrency vs Parallelism
Term	Meaning	Example
Concurrency	Dealing with many things at once	One chef managing multiple orders
Parallelism	Doing many things at once	Multiple chefs each cooking
Our thread pool will give us true parallelism — multiple OS threads running simultaneously.

📋 Summary of Step 1
✅ Your current server is synchronous — one request at a time
✅ Blocking = frozen, waiting, nothing else can happen
✅ The fix = threads — spawn work onto separate threads
✅ But not unlimited threads — we'll use a Thread Pool (coming in Step 4)