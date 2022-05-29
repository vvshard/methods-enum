use std::str::FromStr;

use proc_macro::Group as SctGroup;
use proc_macro::TokenTree::{Group, Ident, Punct};
use proc_macro::{Delimiter, TokenStream};

#[derive(Debug, PartialEq)]
struct Attr {
    enum_name: String,
    run_method: String,
}
impl Attr {
    fn new(attr_ts: TokenStream) -> Attr {
        let mut attr_it = attr_ts.into_iter();
        match [attr_it.next(), attr_it.next(), attr_it.next()] {
            [Some(Ident(enum_n)), Some(Punct(p0)), Some(Ident(run_method))]
                if p0.to_string() == ":" =>
            {
                Attr {
                    enum_name: enum_n.to_string(),
                    run_method: run_method.to_string(),
                }
            }
            _ => panic!("syntax error in attribute #[methods_enum::gen(?? "),
        }
    }
}

fn unvrap_ts(ts: TokenStream, lvl: usize) {
    for tt in ts {
        let indent = "  ".repeat(lvl);
        match tt {
            Group(gr) => {
                println!("{indent}Group({:?})-", gr.delimiter());
                unvrap_ts(gr.stream(), lvl + 1);
            }
            Ident(id) => println!("{indent}Ident:{id}"),
            proc_macro::TokenTree::Literal(id) => println!("{indent}Literal:'{id}'"),
            Punct(id) => println!("{indent}Punct:'{id}'"),
        }
    }
}

#[proc_macro_attribute]
pub fn gen(attr_ts: TokenStream, item_ts: TokenStream) -> TokenStream {
    // println!("attr_ts: \"{}\"", attr_ts.to_string());
    // unvrap_ts(attr_ts.clone(), 0);
    println!("item_ts: \"{}\"", item_ts.to_string());
    unvrap_ts(item_ts.clone(), 0);

    let attr = Attr::new(attr_ts);

    let mut out_ts = TokenStream::from_str(
        &("
        #[derive(Debug)] 
        #[allow(non_camel_case_types)]
        enum "
            .to_string()
            + &attr.enum_name),
    )
    .unwrap();

    let mut item_it = item_ts.into_iter().skip_while(|tt| match tt {
        Ident(id) if id.to_string() == "impl" => false,
        _ => true,
    });
    item_it.next();
    let (item_name, mut impl_it) = match [item_it.next(), item_it.next(), item_it.next()] {
        [Some(Ident(item_n)), Some(Group(gr)), None] if gr.delimiter() == Delimiter::Brace => {
            (item_n.to_string(), gr.stream().into_iter())
        }
        _ => panic!("syntax error: 'this attribute must be set on block impl'"),
    };

    let mut impl_ts = TokenStream::new();
    let mut enum_s = String::new();
    let call_run_pattern =
        "self.".to_string() + &attr.run_method + "(" + &attr.enum_name + "::$meth($params))";

    // metods loop
    loop {
        let first_ts = match impl_it.try_fold(TokenStream::new(), |mut ts, tt| match tt {
            Punct(p) if p.to_string() == ";" => Err((ts, true)),
            Group(ref gr) if gr.delimiter() == Delimiter::Brace => {
                ts.extend(TokenStream::from(tt));
                Err((ts, false))
            }
            _ => {
                ts.extend(TokenStream::from(tt));
                Ok(ts)
            }
        }) {
            Err((ts, true)) => {
                impl_ts.extend(ts.clone());
                ts
            }
            Err((ts, false)) => {
                impl_ts.extend(ts);
                continue;
            }
            Ok(_ts) => break,
        };
        let mut sign_it = first_ts.into_iter().skip_while(|tt| match tt {
            Ident(id) if id.to_string() == "fn" => false,
            _ => true,
        });
        let span = sign_it.next().unwrap().span();
        let (meth, args_gr) = match [sign_it.next(), sign_it.next()] {
            [Some(Ident(n)), Some(Group(gr))] if gr.delimiter() == Delimiter::Parenthesis => {
                (n.to_string(), gr)
            }
            _ => panic!("syntax error: {:?}", span),
        };
        enum_s = enum_s + &meth + "(";
        let mut params = String::new();
        let mut args_it = args_gr.stream().into_iter().skip_while(|tt| match tt {
            Punct(p) if p.to_string() == "," => false,
            _ => true,
        });

        // args loop
        let mut lg = 0;
        let mut first = true;
        loop {
            match args_it.next() {
                Some(Punct(p)) if p.to_string() == "," && lg == 0 => {
                    match [args_it.next(), args_it.next()] {
                        [Some(Ident(id)), Some(Punct(p))] if p.to_string() == ":" => {
                            if first {
                                first = false;
                            } else {
                                enum_s += ", ";
                                params += ", ";
                            }
                            params += &id.to_string();
                        }
                        _ => break,
                    }
                }
                Some(Punct(p)) if "<>".contains(&p.to_string()) => {
                    lg = lg + if p.to_string() == "<" { 1 } else { -1 };
                    enum_s += &p.to_string();
                }
                None => break,
                Some(tt) => enum_s += &tt.to_string(),
            };
        } // args loop

        enum_s += "), ";

        let mut call_run = call_run_pattern.replace("$meth", &meth);
        call_run = call_run.replace("$params", &params);
        if let None = sign_it.next() {
            call_run += ";"
        }

        impl_ts.extend(TokenStream::from(Group(SctGroup::new(
            Delimiter::Brace,
            TokenStream::from_str(&call_run).unwrap(),
        ))));
    } // metods loop

    if enum_s.contains('&') {
        enum_s = enum_s.replace('&', "&'a ");
        out_ts.extend(TokenStream::from_str("<'a>").unwrap());
    }
    out_ts.extend(TokenStream::from(Group(SctGroup::new(
        Delimiter::Brace,
        TokenStream::from_str(&enum_s).unwrap(),
    ))));
    out_ts.extend(
        TokenStream::from_str(
            &("
        #[allow(unused_must_use)]
        impl "
                .to_string()
                + &item_name),
        )
        .unwrap(),
    );
    out_ts.extend(TokenStream::from(Group(SctGroup::new(
        Delimiter::Brace,
        impl_ts,
    ))));

    println!("out_ts: \"{}\"", out_ts.to_string());

    out_ts
}
