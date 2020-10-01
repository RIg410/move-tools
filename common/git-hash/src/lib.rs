/*!
    Utility crate for get current git HEAD hash as

    - environment variable `GIT_HASH`
    - constant `Option<String>`

    ## Usage examples

    ### Use env var:

    ```rust
    // build.rs
    extern crate git_hash;

    fn main() { git_hash::env_git_hash_short(); }
    ```

    ```rust,ignore
    // main.rs
    extern crate git_hash;

    fn main() { println!("{}", env!("GIT_HASH")); }
    ```

    ### Use proc macro:

    ```rust
    // main.rs
    extern crate git_hash;

    fn main() {
        println!("{:?}", git_hash::git_hash!());
        println!("{:?}", git_hash::git_hash_short!());
    }
    ```
*/

pub use macros::git_hash;
pub use macros::git_hash_short;
pub mod cmd;

/// Sets the current git (HEAD) commit SHA to env var `GIT_HASH`
/// and makes it available for build & source code via standard `env!` macro.
pub fn env_git_hash() {
    if let Some(git_hash) = cmd::git_hash() {
        println!("cargo:rustc-env=GIT_HASH={}", git_hash);
        println!("cargo:rustc-rerun-if-changed=.git/HEAD");
    }
}

/// Sets the current git (HEAD) commit SHA (as shorten view) to env var `GIT_HASH_SHORT`
/// and makes it available for build & source code via standard `env!` macro.
pub fn env_git_hash_short() {
    if let Some(git_hash) = cmd::git_hash_short() {
        println!("cargo:rustc-env=GIT_HASH_SHORT={}", git_hash);
        println!("cargo:rustc-rerun-if-changed=.git/HEAD");
    }
}