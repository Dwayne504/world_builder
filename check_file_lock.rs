use std::fs::File;
fn _check(f: &File) {
    let _ = f.try_lock();
    let _ = f.lock();
    let _ = f.unlock();
    let _ = f.try_lock_shared();
    let _ = f.lock_shared();
}
fn main(){}
