use dreamcast::{self};

fn main() {
    let dc = Box::into_raw(Box::new(dreamcast::Dreamcast::default()));
    dreamcast::init_dreamcast(dc);

    loop {
        dreamcast::run_slice_dreamcast(dc);
    }
}
