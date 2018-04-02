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
//! # #![feature(proc_macro)]
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

#![feature(proc_macro)]

extern crate proc_macro;
extern crate proc_macro2;
#[macro_use]
extern crate quote;
#[macro_use]
extern crate syn;
extern crate heck;

use proc_macro::Diagnostic;
use proc_macro2::{Span, TokenStream};
use syn::{Ident, Item, ItemEnum, ItemImpl, ItemMod, ItemStruct, Path, Attribute, Variant};
use syn::synom::Synom;
use syn::spanned::Spanned;
use quote::ToTokens;
use heck::SnakeCase;

struct AttrArgs {
    enum_name: Ident,
}

impl Synom for AttrArgs {
    named!(parse -> Self, do_parse!(
		all: parens!(
			do_parse!(
				enum_name: syn!(Ident) >>
				({
					AttrArgs { enum_name }
				})
		)) >>
		({
			let (_parens, args) = all;
			args
		})
	));
}

/// The procedural macro attribute implementing a new enum and conversion methods.
/// See the crate documentation for an usage example.
#[proc_macro_attribute]
pub fn value_from_type(
    args: proc_macro::TokenStream,
    input: proc_macro::TokenStream,
) -> proc_macro::TokenStream {
    println!("[BUILD] Running proc macro: value_from_type");

    match value_from_type_impl(args, input) {
        Ok(v) => v,
        Err(e) => {
            e.emit();
            panic!("See emitted errors");
        }
    }
}

fn value_from_type_impl(
    args: proc_macro::TokenStream,
    input: proc_macro::TokenStream,
) -> Result<proc_macro::TokenStream, Diagnostic> {
    let input: TokenStream = input.into();
    let call_site = Span::call_site();
    let def_site = Span::def_site();

    // println!("ARG TOKENS: {:?}", args.to_string());

    let args: AttrArgs = syn::parse2(args.into()).map_err(|e| {
        let msg = format!("Failed parsing arguments: {:}", e);
        call_site.unstable().error(msg)
    })?;

    // Parse module code
    // IMPORTANT: Our own macro attribute is automatically stripped!
    let mut module_def: ItemMod = syn::parse2(input.into()).map_err(|e| {
        let msg = format!("You have syntax errors in your module: {:}", e);
        call_site.unstable().error(msg)
    })?;

    // Clones all ItemStructs from the ItemModule AST.
    // Clones are necessary because we inject new Items into the AST item and mutable
    // access is prohibited when we still have lingering immutable borrows.
    let all_structs: Vec<_> = locate_structs(&module_def).cloned().collect();
    if all_structs.len() < 1 {
        let msg = "You have no structs defined in your module";
        return Err(module_def.span().unstable().error(msg));
    }

    // Force resolving at call site, but retain location info.
    // Subsequent usage of enum_access will always correctly resolve to the 
    // auto-generated enum type.
    let enum_site = args.enum_name.span().resolved_at(call_site);
    let enum_access = Ident::new(args.enum_name.as_ref(), enum_site);
    // let super_access: Token![super] = enum_site.into();

    // The collect trick: Transpose an iterator of Results into a Result of 
    // that iterator. The first encountered Result::Err will be bound to the variable.
    let enum_variants: Result<Vec<Variant>, _> = all_structs.iter()
        .map(|s| {
            let variant_attrs = build_transitive_attrs(&s)?;
            let variant_site = s.ident.span().resolved_at(call_site);
            let variant_ident = Ident::new(s.ident.as_ref(), variant_site);
            let variant_tokens = quote_spanned!{variant_site=>
                #( #variant_attrs )*
                #variant_ident
            };
            syn::parse2(variant_tokens.into()).map_err(|e| {
                let msg = format!("Issue creating enum invariant `{:}`: {:}", s.ident.as_ref(), e);
                variant_site.unstable().error(msg)
            })
        })
        .collect();

    // Validate each variant was successfully constructed.
    let enum_variants = match enum_variants {
        Ok(data) => data,
        Err(e) => return Err(e),
    };

    // Prepare submodule which imports the necessary types so we can push implementation details into it.
    let snek_enum_name = args.enum_name.as_ref().to_snake_case();
    let impl_mod_name = format!("_impl_{:}", snek_enum_name);
    let impl_mod_access = Ident::new(&impl_mod_name, call_site);

    let impl_tokens = quote_spanned!{def_site=>
        // Hide this module in docs because crates referring the macro-invocation crate
        // are able to access this module!
        #[doc(hidden)]
        mod #impl_mod_access {
            // self will resolve to the local module
            // super will resolve to the parent module (unhygienic?)
            extern crate value_from_type_traits;
            use self::value_from_type_traits::FromType;
            use super::#enum_access;
        }
    };
    let mut impl_module: ItemMod = syn::parse2(impl_tokens.into()).map_err(|e| {
            let msg = format!("Issue creating implementation submodule: {:}", e);
            def_site.unstable().error(msg)
    })?;

    {
        // Build enum from structs
        let variants = enum_variants.iter();
        let fab_tokens = quote_spanned!{enum_site=>
            #[doc = "Autogenerated."]
            #[derive(Debug, Clone, PartialEq)]
            pub enum #enum_access {
                #( #variants ),*
            }
        };
        let fab_enum: ItemEnum = syn::parse2(fab_tokens.into()).map_err(|e| {
            let msg = format!("Issue generating the enum `{:}`: {:}", enum_access.as_ref(), e);
            enum_site.unstable().error(msg)
        })?;

        push_into_module(&mut module_def, fab_enum.into())?;
    }

    {
        // Build conversion implementations
        for (struct_item, enum_variant) in all_structs.into_iter().zip(enum_variants) {
            let enum_variant_ident = enum_variant.ident;
            let (impl_generics, ty_generics, where_clause) = struct_item.generics.split_for_impl();

            // Resolving at def_site to prevent tampering through type aliases.
            let target_site = struct_item.span().resolved_at(def_site);
            // But the struct reference comes from the call_site!
            let struct_access = struct_item.ident;

            let fab_tokens = quote_spanned!{target_site=>
                // We can directly use #enum_access because it was imported earlier
                // (see impl_tokens)
                impl #impl_generics FromType<super::#struct_access #ty_generics> 
                for #enum_access #where_clause
                {
                    fn from_type() -> Self {
                        #enum_access::#enum_variant_ident
                    }
                }
            };

            let fab_impl: ItemImpl = syn::parse2(fab_tokens.into()).map_err(|e| {
                let msg = format!("Issue building the implementation: {:}", e);
                target_site.unstable().error(msg)
            })?;
            // The implementation is pushed into the IMPLEMENTATION MODULE, which is a 
            // child module of the module this macro is invoked on!
            push_into_module(&mut impl_module, fab_impl.into())?;
        }
    }

    // Push implementations module into original module.
    push_into_module(&mut module_def, impl_module.into())?;
    // Remove all items that were targetted for processing by us. 
    // These items will cause the compiler to panic because it doesn't know what to do
    // when they are left in the AST.
    // purge_transitive_attrs(&mut module_def);

    let module_tokens = module_def.into_tokens();
    return Ok(module_tokens.into());
}

/// Select all ItemStructs from the provided ItemMod AST.
fn locate_structs(data: &ItemMod) -> impl Iterator<Item = &ItemStruct> {
    return data.content
        .iter()
        .flat_map(|content| content.1.iter())
        .flat_map(|item| {
            if let &Item::Struct(ref s) = item {
                Some(s)
            } else {
                None
            }
        });
}


/// Finds the custom attribute marker `transitive` and builds a new attribute
/// from its parameter.
fn build_transitive_attrs(data: &ItemStruct) -> Result<Vec<Attribute>, Diagnostic> {
    let call_site = Span::call_site();
    // 
    let custom_attrs_ident = quote!{transitive};
    let custom_attr_path: Path = syn::parse2(custom_attrs_ident.into()).unwrap();
    let custom_attrs: Vec<_> = data.attrs.iter().filter(|a| a.path == custom_attr_path).collect();

    custom_attrs.iter()
        .map(|ca| {
            // Returns (span, meta-object)
            ca.interpret_meta()
            .ok_or_else(|| {
                let msg = "You have a syntax error in your custom attribute";
                custom_attrs[0].span().unstable().error(msg)
            })
            .map(|result_meta| {
                // Pushing span information next to the Meta object will allow for better 
                // error messages.
                (ca.span(), result_meta)
            })
        })
        .map(|ca_result| {
            match ca_result {
                Ok( (span, meta,) ) => {
                    let target_site = span.resolved_at(call_site);
                    // Construct an ItemMod because Attribute DOES NOT implement Synom!
                    let tokens = quote_spanned!{target_site=>
                        #[ #meta ] struct tmp;
                    };
                    syn::parse2::<ItemStruct>(tokens.into()).map_err(|e| {
                        let msg = format!("Issue building transitive attribute: {:}", e);
                        span.unstable().error(msg)
                    })
                    // Here we filter our attribute out again.
                    .map(|s| s.attrs[0].clone())
                },
                // Error must be unwrapped here!
                Err(e) => Err(e),
            }
        })
        .collect()
}

/// Removes all custom attributes marked `transitive` from . 
fn purge_transitive_attrs(data: &mut ItemMod) {
    let custom_attrs_ident = quote!{transitive};
    let custom_attr_path: Path = syn::parse2(custom_attrs_ident.into()).unwrap();

    data.content
        .iter_mut()
        .flat_map(|content| content.1.iter_mut())
        .flat_map(|item| {
            if let &mut Item::Struct(ref mut s) = item {
                Some(s)
            } else {
                None
            }
        })
        .for_each(|item_mod| {
            item_mod.attrs.retain(|a| a.path != custom_attr_path);
        });
}

/// Push new contents into the AST representation of a module.
fn push_into_module(module: &mut ItemMod, i: Item) -> Result<(), Diagnostic> {
    let mod_span = module.span();
    match module.content.as_mut() {
        Some(&mut (_, ref mut c)) => {
            c.push(i);
            Ok(())
        }
        None => {
            let msg = "This module doesn't have any contents!";
            Err(mod_span.unstable().error(msg))
        }
    }
}
