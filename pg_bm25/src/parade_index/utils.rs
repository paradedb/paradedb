use pgrx::Spi;

pub fn with_notice_suppressed<F: FnOnce()>(f: F) {
    Spi::run("SET client_min_messages TO WARNING")
        .expect("failed to set client_min_messages to WARNING");
    f();
    Spi::run("SET client_min_messages TO NOTICE")
        .expect("failed to set client_min_messages to NOTICE");
}
