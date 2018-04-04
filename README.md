This repository contains two Rust crates. Both crates must be used as dependancy on your application crate!

- [`value_from_type_macros`](value_from_type_macros)  
    This crate contains the macro attribute `value_from_type` which builds an enumeration
    from structures defined within the subject module.
- [`value_from_type_traits`](value_from_type_traits)  
    This crate defines the trait `FromType` which is implemented for each structure returning
    a variant of the generated enum.  
    See `value_from_type_macros` crate for more information!

# Building

*ANY CRATE* depending on this macro must pass the flag `procmacro2_semver_exempt` to rustc.
When building with Cargo it's possible to create a small configuration file that automatically
passes this flag for you.  
This configuration file must contain this

```toml
[build]
# Necessary to keep unstable Span behaviour of proc-macro2
rustflags = "--cfg procmacro2_semver_exempt"
```

Name and place this file `.cargo/config` next to your crate's `cargo.toml`.  
This repository has this file located at [.cargo/config](.cargo/config).  
For more information about Cargo config, look [here](https://doc.rust-lang.org/cargo/reference/config.html).
