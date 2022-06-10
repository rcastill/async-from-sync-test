use std::{env::args, thread, time::Duration};

use tokio::{runtime::Handle, task::block_in_place, time::sleep};

async fn loopy(name: &'static str) {
    let mut i = 0;
    loop {
        i += 1;
        eprintln!("{name} = {i}");
        sleep(Duration::from_secs(1)).await;
    }
}

/// This case a takes a handle of the current runtime. This is fine because is
/// called from the main thread (managed by tokio).
///
/// Then it blocks on a future.
///
/// This case fails, because [Handle::block_on](https://docs.rs/tokio/latest/tokio/runtime/struct.Handle.html#method.block_on)
/// will block the current thread (main thread) which is a tokio runtime thread.
///
/// _You should never block a runtime thread._
fn spawn_tokio_task_a() {
    let handle = Handle::current();
    handle.block_on(loopy("A"))
}

/// This case a takes a handle of the current runtime. This is fine because is
/// called from the main thread (managed by tokio).
///
/// Then it blocks on a future _WHILE_ telling tokio that we are going to block
/// the main thread.
///
/// This works because [block_in_place](https://docs.rs/tokio/latest/tokio/task/fn.block_in_place.html)
/// will make tokio move any critical tasks to other worker threads. Hence, this
/// is only possible using tokio's multi-thread runtime.
fn spawn_tokio_task_b() {
    let handle = Handle::current();
    block_in_place(move || handle.block_on(loopy("B")))
}

/// This case a takes a handle of the current runtime. This is fine because is
/// called from the main thread (managed by tokio), _THEN_ it is passed to a
/// user-spawned thread (not managed by tokio). You _CANNOT_ call
/// [Handle::current](https://docs.rs/tokio/latest/tokio/runtime/struct.Handle.html#method.current)
/// outside the tokio runtime (a thread not managed by tokio).
///
/// Then it blocks on a future in a thread outside the tokio runtime, which is
/// OK. No need for [block_in_place](https://docs.rs/tokio/latest/tokio/task/fn.block_in_place.html)
fn spawn_tokio_task_c() {
    let handle = Handle::current();
    thread::spawn(move || handle.block_on(loopy("C")));
}

enum Case {
    A,
    B,
    C,
}

#[tokio::main]
async fn main() {
    let case = args().nth(1).and_then(|case| {
        Some(match &*case {
            "a" => Case::A,
            "b" => Case::B,
            "c" => Case::C,
            _ => return None,
        })
    });
    let case = match case {
        Some(case) => case,
        None => {
            eprintln!("Usage: ./run.sh case");
            return;
        }
    };

    let main_task = tokio::spawn(loopy("main"));
    sleep(Duration::from_millis(3100)).await;

    match case {
        Case::A => spawn_tokio_task_a(),
        Case::B => spawn_tokio_task_b(),
        Case::C => spawn_tokio_task_c(),
    }

    main_task.await.expect("Failed running main task")
}
