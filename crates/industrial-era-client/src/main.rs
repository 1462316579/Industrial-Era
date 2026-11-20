//! Industrial Era: native Rust client for Minecraft Java Edition 26.3 gameplay.
#![forbid(unsafe_code)]

#[cfg(not(any(feature = "system-allocator", feature = "alloc-census")))]
#[global_allocator]
static ALLOCATOR: industrial_era_threads::Allocator = industrial_era_threads::Allocator;
#[cfg(all(feature = "alloc-census", not(feature = "system-allocator")))]
#[global_allocator]
static ALLOCATOR: industrial_era_threads::CensusAllocator = industrial_era_threads::CensusAllocator;

/// The mod API level this game serves, kept in the program for the launcher
/// to read without running it (`industrial_era_mods::api_level::MARK`). `main`
/// reads it, so the linker keeps it.
static MOD_API_LEVEL: [u8; 27] = industrial_era_mods::api_level::MARK;

fn main() {
    std::hint::black_box(&MOD_API_LEVEL);
    industrial_era_client::entry::main();
}
