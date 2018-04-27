use heck::CamelCase;
use proc_macro;
use proc_macro2::Span;
use quote::ToTokens;
use syn::spanned::Spanned;
use syn::{self, Ident, Item, ItemConst, ItemEnum, ItemImpl, ItemMod, ItemStruct, Variant};

use args::AttrArgs;
use common::{build_invariants, filter_structs, push_into_module};

// Macro imports
use super::quote;

pub fn value_from_type_impl(
    args: proc_macro::TokenStream,
    input: proc_macro::TokenStream,
) -> Result<proc_macro::TokenStream, proc_macro::Diagnostic> {
    // Parse macro arguments.
    let args: AttrArgs = syn::parse2(args.into()).map_err(|e| {
        let msg = format!("Failed parsing arguments: {:}", e);
        Span::call_site().unstable().error(msg)
    })?;

    // Parse module code
    // IMPORTANT: Our own macro attribute is automatically stripped!
    let mut module_def: ItemMod = syn::parse2(input.into()).map_err(|e| {
        // Emit warning for context
        let msg = format!("You have syntax errors: {:}", e);
        Span::call_site().unstable().warning(msg).emit();
        // This is the fail-hard message
        let msg = format!("This macro can ONLY be applied to valid modules");
        Span::call_site().unstable().error(msg)
    })?;

    // TODO; Find out which attributes to implement for each variant.
    let enum_invariants = build_invariants(filter_structs(&module_def))?;
    if enum_invariants.len() < 1 {
        let msg = "You have no structs defined in your module";
        return Err(module_def.span().unstable().error(msg));
    }

    // Build and insert the enum item into the subject module.
    let enum_ident = args.enum_name();
    let enum_span = enum_ident.span().resolved_at(Span::call_site());
    let enum_ident = Ident::new(&enum_ident.as_ref().to_camel_case(), enum_span);
    push_into_module(
        &mut module_def,
        build_enum(enum_ident, enum_invariants.iter())?,
    )?;

    // Build implementations for each structure.
    let impl_data: Vec<Item> = {
        let sv_combo = filter_structs(&module_def).zip(enum_invariants.iter());
        build_implementations(sv_combo, enum_ident)?
            .into_iter()
            .map(Into::into)
            .collect()
    };

    // Package all generated implementations and push them into the subject module.
    let impl_dummy = Ident::new(
        &format!("__IMPL_FOR_{}", enum_ident.as_ref()),
        Span::call_site(),
    );
    let implementation_tokens = quote!{
        #[allow(non_upper_case_globals)]
        #[doc(hidden)]
        const #impl_dummy: () = {
            extern crate value_from_type_traits as _vftt;
            #( #impl_data )*
        };
    };
    let impl_item: ItemConst = syn::parse2(implementation_tokens.into()).map_err(|e| {
        let msg = format!("Issue packing implementations: {:}", e);
        Span::call_site().unstable().error(msg)
    })?;
    push_into_module(&mut module_def, impl_item)?;

    return Ok(module_def.into_tokens().into());
}

/// Build an enumeration from the provided invariants.
fn build_enum<'a, I>(name: Ident, variants: I) -> Result<ItemEnum, proc_macro::Diagnostic>
where
    I: IntoIterator<Item = &'a Variant>,
{
    let variants = variants.into_iter();
    let derives = vec![
        Ident::from("Debug"),
        Ident::from("Clone"),
        Ident::from("Copy"),
        Ident::from("PartialEq"),
        Ident::from("Eq"),
        Ident::from("Hash"),
    ];
    //
    let enum_tokens = quote!{
        #[doc = "Autogenerated enum."]
        #[derive( #( #derives ),* )]
        pub enum #name {
            #( #variants ),*
        }
    };
    //
    syn::parse2(enum_tokens.into()).map_err(|e| {
        let msg = format!("Issue generating the enum `{:}`: {:}", name.as_ref(), e);
        name.span().unstable().error(msg)
    })
}

/// Build implementations to transition between structure types and enum invariants.
fn build_implementations<'a, 'b, I>(
    struct_variant_combo: I,
    enum_ident: Ident,
) -> Result<Vec<ItemImpl>, proc_macro::Diagnostic>
where
    I: IntoIterator<Item = (&'a ItemStruct, &'b Variant)>,
{
    struct_variant_combo
        .into_iter()
        .map(|(s, v)| internal_build_impls(s, v, enum_ident))
        .collect()
}

fn internal_build_impls<'a, 'b>(
    struct_item: &'a ItemStruct,
    variant: &'b Variant,
    enum_ident: Ident,
) -> Result<ItemImpl, proc_macro::Diagnostic> {
    let variant_ident = variant.ident;
    let struct_ident = struct_item.ident;
    let (impl_generics, ty_generics, where_clause) = struct_item.generics.split_for_impl();

    // _vftt is the traits crate implemented by the packaging code.
    let impl_tokens = quote!{
        impl #impl_generics _vftt::FromType<#struct_ident #ty_generics> for #enum_ident
        #where_clause
        {
            fn from_type() -> Self {
                #enum_ident::#variant_ident
            }
        }
    };
    //
    syn::parse2(impl_tokens.into()).map_err(|e| {
        let msg = format!("Issue building `From` implementation: {:}", e);
        struct_ident.span().unstable().error(msg)
    })
}
