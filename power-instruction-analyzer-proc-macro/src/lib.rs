// SPDX-License-Identifier: LGPL-2.1-or-later
// See Notices.txt for copyright information

#[macro_use]
mod inline_assembly;
mod instructions;

use instructions::Instructions;
use proc_macro::TokenStream;
use quote::quote;
use std::{env, fs, path::Path};
use syn::parse_macro_input;

#[proc_macro]
pub fn instructions(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as Instructions);
    match input.to_tokens() {
        Ok(retval) => {
            if cfg!(feature = "debug-proc-macro") {
                fs::write(
                    Path::new(&env::var_os("CARGO_MANIFEST_DIR").unwrap())
                        .join("proc-macro-out.rs"),
                    retval.to_string(),
                )
                .unwrap();
                quote! {
                    include!(concat!(env!("CARGO_MANIFEST_DIR"), "/proc-macro-out.rs"));
                }
            } else {
                retval
            }
        }
        Err(err) => err.to_compile_error(),
    }
    .into()
}
