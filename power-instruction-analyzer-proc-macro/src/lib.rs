// SPDX-License-Identifier: LGPL-2.1-or-later
// See Notices.txt for copyright information

#[macro_use]
mod inline_assembly;
mod instructions;

use instructions::Instructions;
use proc_macro::TokenStream;
use syn::parse_macro_input;

#[proc_macro]
pub fn instructions(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as Instructions);
    match input.to_tokens() {
        Ok(retval) => retval,
        Err(err) => err.to_compile_error(),
    }
    .into()
}
