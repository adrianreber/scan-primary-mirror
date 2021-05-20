use console::style;
use std::sync::atomic::{AtomicUsize, Ordering};

pub static DEBUG: AtomicUsize = AtomicUsize::new(0);
pub static STEPS: AtomicUsize = AtomicUsize::new(6);

static CALL_COUNT: AtomicUsize = AtomicUsize::new(1);

pub fn print_step(msg: String) {
    let d = DEBUG.load(Ordering::SeqCst);
    let s = CALL_COUNT.load(Ordering::SeqCst);
    CALL_COUNT.fetch_add(1, Ordering::SeqCst);
    if d == 0 {
        return;
    }
    println!(
        " Step {}: {}",
        style(format!("[{}/{}]", s, STEPS.load(Ordering::SeqCst)))
            .bold()
            .dim(),
        msg
    );
}
