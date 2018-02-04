This repository contains two Rust crates. Both crates must be used as dependancy on your application crate!

- [`value_from_type_macros`](value_from_type_macros)  
    This crate contains the macro attribute `value_from_type` which builds an enumeration
    from structures defined within the subject module.
- [`value_from_type_traits`](value_from_type_traits)  
    This crate defines the trait `FromType` which is implemented for each structure returning
    a variant of the generated enum.  
    See `value_from_type_macros` crate for more information!