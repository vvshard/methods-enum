#![allow(unused)]

use std::str::FromStr;

use proc_macro::TokenTree::{Group, Ident, Literal, Punct};
use proc_macro::{Delimiter, Span, TokenStream};

// region: debug

fn unvrap_ts(ts: TokenStream, lvl: usize) {
    for tt in ts {
        let indent = "  ".repeat(lvl);
        match tt {
            Group(gr) => {
                println!("{indent}Group({:?})-", gr.delimiter());
                unvrap_ts(gr.stream(), lvl + 1);
            }
            Ident(id) => println!("{indent}Ident:{id}"),
            Literal(id) => println!("{indent}Literal:'{id}'"),
            Punct(id) => println!("{indent}Punct:'{id}'"),
        }
    }
}

fn gen_duppy() -> TokenStream {
    TokenStream::from_str(stringify!(
        #[derive(Debug)]
        #[allow(non_camel_case_types)]
        enum Meth<'a> {
            add_text(&'a str),
            content(),
            request_review(),
            reject(),
            approve(),
        }

        impl Post {
            #[allow(unused_must_use)]
            pub fn add_text(&mut self, text: &str){
                self.maintain_methods(Meth::add_text(text));
            }
            /// content...
            pub fn content(&mut self) -> Result<&str, String> {
                self.maintain_methods(Meth::content())
            }
            #[allow(unused_must_use)]
            pub fn request_review(&mut self) {
                self.maintain_methods(Meth::request_review());
            }
            pub fn reject(&mut self) -> Result<&str, String>{
                self.maintain_methods(Meth::reject())
            }
            pub fn approve(&mut self) -> Result<&str, String> {
                self.maintain_methods(Meth::approve())
            }


        }

    ))
    .unwrap()
}

// endregion: debug

#[proc_macro_attribute]
pub fn gen(attr_ts: TokenStream, item_ts: TokenStream) -> TokenStream {
    println!("attr: \"{}\"", attr_ts.to_string());
    unvrap_ts(attr_ts.clone(), 0);
    println!("item: \"{}\"", item_ts.to_string());
    unvrap_ts(item_ts.clone(), 0);

    let ts = gen_duppy();
    println!("{}", ts.to_string());

    ts
}
