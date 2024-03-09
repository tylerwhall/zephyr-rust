# A no_std example for zephyr-rust

This is essentially the same as `samples/rust-app`, but with the following changes to make it
`#![no_std]`:
- Replaced `zephyr::` with `zephyr_core::`
- Replaced `println!()` with `zephyr_core::any::k_str_out(format!())` (except where println was 
  to be called explicitly)
- Used `#[cfg(feature = "have_std")]` to 'comment out' code that requires std. This makes it easy to see what parts of Rust (`println`, `std::time`, etc) and zephyr-rust (`zephyr` vs `zephyr_core`) require std. You can of course run those lines by enabling the `have_std` feature.
