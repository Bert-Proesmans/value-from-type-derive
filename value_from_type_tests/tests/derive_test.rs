#![feature(proc_macro, proc_macro_mod)]

extern crate value_from_type_macros;
extern crate value_from_type_traits;

// Attribute macro must be imported through a use statement.
use value_from_type_macros::value_from_type;
// Implemented trait on `EnumName`
use value_from_type_traits::IntoEnum;

mod struct_container {
 	// The parameter indicates the enum identifier.
     #![value_from_type(EnumName)]

     #[derive(Debug)]
     pub struct X();
}

#[test]
fn check_conversion() {
	// Explicit import for sake of example.
	use struct_container::{EnumName, X};
    
    assert_eq!(EnumName::X, X::into_enum());
}
