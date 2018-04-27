//! Procedural macro attribute to match structure types with an enum variant.
//!
//! This macro can be applied on a module to make a connection between each defined struct
//! and a newly created enum type. This enum is built into the same module as
//! the macro is invocated upon.
//! The macro will also implement [`value_from_type_traits::FromType`] on the enum
//! for each struct (within the module) as generic argument.
//!
//! # Examples
//!
//! ```
//! # #![feature(proc_macro, proc_macro_mod)]
//! # extern crate value_from_type_macros;
//! # extern crate value_from_type_traits;
//! // Attribute macro must be imported through a use statement.
//! use value_from_type_macros::value_from_type;
//! // Implemented trait on `EnumName`
//! use value_from_type_traits::IntoEnum;
//!
//! mod temp {
//!     // The parameter indicates the enum identifier.
//!     #![value_from_type(EnumName)]
//!
//!     #[derive(Debug)]
//!     pub struct X();
//! }
//!
//! // Explicit import for sake of example.
//! use self::temp::{EnumName, X};
//! // use self::temp::*;
//!
//! # fn main() {
//! assert_eq!(EnumName::X, X::into_enum());
//! # }
//! ```
//!

#![doc(html_root_url = "https://docs.rs/value_from_type_macros")]
#![feature(proc_macro, proc_macro_path_invoc)]

extern crate proc_macro;
extern crate proc_macro2;
#[macro_use]
extern crate quote;
#[macro_use]
extern crate syn;
extern crate heck;

// Quote's default spanning of syntax items is at the definition site.
// This macro will overwrite the default behaviour to use the call site.
// Spanning specialises resolving of syntax semantics. Call site indicates
// symantics are resolved at the place (context) where the macro is called.
// Def site indicates symantics are resolved within the context of the macro crate.
//
// The convincing argument is that call site is an intuitive default for most situations.
// As long as procedural macros are not stabilized changes to rustc may break this crate
// in non-intuitive ways.
//
// The difference between both sites is important because it constraints the operations
// you can perform. See 'https://github.com/rust-lang/rust/issues/45934' for precise
// details.
macro_rules! quote {
    ($($tt:tt)*) => {
        $crate::quote::quote_spanned!($crate::proc_macro2::Span::call_site()=> $($tt)*)
    }
}

/// Execute the provided statement if the runtime encountered debug modus.
/// 
/// Currently debug modus will activate when the `RUST_BACKTRACE` environment
/// variable is set to `1` (String).
macro_rules! debug {
    ($($tt:tt)*) => {
        match $crate::std::env::var("RUST_BACKTRACE") {
            Ok(ref val) if val == "1" => { $($tt)* },
            _ => {},
        };
    }
}

mod args;
mod attr;
mod common;

use attr::value_from_type_impl;

/// The procedural macro attribute implementing a new enum and conversion methods.
/// See the crate documentation for an usage example.
#[proc_macro_attribute]
pub fn value_from_type(
    args: proc_macro::TokenStream,
    input: proc_macro::TokenStream,
) -> proc_macro::TokenStream {
    println!("[BUILD] Running proc macro: value_from_type");


    match value_from_type_impl(args, input) {
        Ok(v) => {
            debug!(println!("Outputted tokens\n{:}\n", v.clone().to_string()));
            v
        },
        Err(e) => {
            e.emit();
            panic!("See emitted errors");
        }
    }
}
