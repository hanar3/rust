//@compile-flags: -Zmiri-disable-weak-memory-emulation -Zmiri-preemption-rate=0

use std::sync::atomic::{AtomicUsize, Ordering};
use std::thread::spawn;

#[derive(Copy, Clone)]
struct EvilSend<T>(pub T);

unsafe impl<T> Send for EvilSend<T> {}
unsafe impl<T> Sync for EvilSend<T> {}

static SYNC: AtomicUsize = AtomicUsize::new(0);

pub fn main() {
    let mut a = 0u32;
    let b = &mut a as *mut u32;
    let c = EvilSend(b);

    // Note: this is scheduler-dependent
    // the operations need to occur in
    // order:
    //  1. store release : 1
    //  2. RMW relaxed : 1 -> 2
    //  3. store relaxed : 3
    //  4. load acquire : 3
    unsafe {
        let j1 = spawn(move || {
            *c.0 = 1;
            SYNC.store(1, Ordering::Release);
        });

        let j2 = spawn(move || {
            if SYNC.swap(2, Ordering::Relaxed) == 1 {
                // Blocks the acquire-release sequence
                SYNC.store(3, Ordering::Relaxed);
            }
        });

        let j3 = spawn(move || {
            if SYNC.load(Ordering::Acquire) == 3 {
                *c.0 //~ ERROR: Data race detected between Read on thread `<unnamed>` and Write on thread `<unnamed>`
            } else {
                0
            }
        });

        j1.join().unwrap();
        j2.join().unwrap();
        j3.join().unwrap();
    }
}
