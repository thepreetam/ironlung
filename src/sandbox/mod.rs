//! Containment sandbox for complex libc calls. Isolates getaddrinfo in a seccomp-restricted helper.

pub mod protocol;
pub mod shm;
