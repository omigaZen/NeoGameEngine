use std::time::Instant;

#[derive(Debug, Clone, Copy)]
pub enum RunMode {
    Poll,
    Wait,
    WaitUntil(Instant),
}
