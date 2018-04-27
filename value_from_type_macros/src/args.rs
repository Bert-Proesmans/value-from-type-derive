use syn::Ident;
use syn::punctuated::Punctuated;
use syn::synom::Synom;

/// Structure used as knowledge base for macro arguments.
pub struct AttrArgs {
    args: Vec<Ident>,
}

impl Synom for AttrArgs {
    named!(parse -> Self, map!(
		Punctuated::<Ident, Token![,]>::parse_terminated_nonempty,
		|args| AttrArgs {
			args: args.into_iter().collect(),
		}
	));
}

impl AttrArgs {
    /// Returns the name chosen by the user.
    pub fn enum_name(&self) -> Ident {
        assert!(self.args.len() > 0);
        // Ident implements copy
        self.args[0]
    }
}
