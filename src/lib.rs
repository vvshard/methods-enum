#![allow(unused)]

use std::str::FromStr;

use proc_macro::TokenTree::{Group, Ident, Punct};
use proc_macro::{Delimiter, Group as SctGroup, TokenStream};

#[derive(Debug, PartialEq, Default)]
struct Attr {
    enum_name: String,
    run_method: String,
    dbg: bool,
    out_name: String,
    def_name: String,
    def_type: Option<String>,
}
impl Attr {
    fn new(attr_ts: TokenStream) -> Attr {
        let mut attr_it = attr_ts.into_iter();
        let (enum_name, dbg) = match attr_it.next() {
            Some(Ident(enum_n)) => (enum_n.to_string(), false),
            Some(Punct(p)) if p.to_string() == "?" => match attr_it.next() {
                Some(Ident(enum_n)) => (enum_n.to_string(), true),
                _ => panic!("syntax error in attribute #[methods_enum::gen(?? "),
            },
            _ => panic!("syntax error in attribute #[methods_enum::gen(?? "),
        };
        let run_method = match [attr_it.next(), attr_it.next()] {
            [Some(Punct(p)), Some(Ident(run_method))] if p.to_string() == ":" => {
                run_method.to_string()
            }
            _ => panic!("syntax error in attribute #[methods_enum::gen({enum_name}??.. "),
        };
        match [
            attr_it.next(),
            attr_it.next(),
            attr_it.next(),
            attr_it.next(),
        ] {
            [None, None, None, None] => Attr {
                enum_name,
                run_method,
                dbg,
                ..Default::default()
            },
            [Some(Punct(p1)), Some(Ident(out_n)), Some(Punct(p2)), Some(Ident(def_n))]
                if p1.to_string() == "=" && p2.to_string() == "/" =>
            {
                Attr {
                    enum_name,
                    run_method,
                    dbg,
                    out_name: out_n.to_string(),
                    def_name: def_n.to_string(),
                    def_type: attr_it.next().map(|tt| match tt {
                        Group(type_gr) if type_gr.delimiter() == Delimiter::Parenthesis => {
                            type_gr.stream().to_string()
                        }
                        _ => panic!(
                            "syntax error in attribute \
                            #[methods_enum::gen in type default: ... {out_n} / {def_n}??.. "
                        ),
                    }),
                }
            }
            _ => panic!(
                "syntax error in attribute #[methods_enum::gen({enum_name}:{run_method}??.. "
            ),
        }
    }

    fn def_opt(&self) -> String {
        match &self.def_type {
            Some(typ) => format!("{}({})", self.def_name, typ),
            None => self.def_name.clone(),
        }
    }
}

//for debug
#[allow(unused)]
fn print_ts(attr_ts: &TokenStream, item_ts: &TokenStream) {
    println!("attr_ts: \"{}\"", attr_ts.to_string());
    unvrap_ts(attr_ts.clone(), 0);
    println!("item_ts: \"{}\"", item_ts.to_string());
    unvrap_ts(item_ts.clone(), 0);
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
    print_ts(&attr_ts, &item_ts);

    let attr = Attr::new(attr_ts);
    dbg!(&attr);

    let mut result_ts = TokenStream::from_str(
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

    let mut enum_s = String::new();
    let mut out_s = String::new();
    let mut impl_ts = TokenStream::new();
    let self_run_enum = format!("self.{}({}::", attr.run_method, attr.enum_name);

    // metods loop
    loop {
        let (first_ts, body_ts) =
            match impl_it.try_fold(TokenStream::new(), |mut ts, tt| match tt {
                Punct(p) if p.to_string() == ";" => Err((ts, Ok(None))),
                Group(ref gr) if gr.delimiter() == Delimiter::Brace && attr.def_name.is_empty() => {
                    ts.extend(TokenStream::from(tt));
                    Err((ts, Err(())))
                }
                Group(gr) if gr.delimiter() == Delimiter::Brace =>{
                    let mut gr_it = gr.stream().into_iter();
                    let it3 = gr_it.by_ref().take(3);
                    let mut ts3 = TokenStream::from_iter(it3);
                    let mut gr_ts: TokenStream = ts3.clone();
                    gr_ts.extend(gr_it);
                    let mut it3 = ts3.into_iter();
                    match [it3.next(), it3.next(), it3.next()] {
                        [Some(Ident(def_n)), Some(Group(_)), Some(Punct(p))]
                        | [Some(Ident(def_n)), Some(Punct(p)), Some(_)]
                            if def_n.to_string() == attr.def_name && p.to_string() == "=" =>
                        {
                            Err((ts, Ok(Some(gr_ts))))
                        }
                        _ => {
                            ts.extend(TokenStream::from(Group(SctGroup::new(
                                Delimiter::Brace,
                                gr_ts,
                            ))));
                            Err((ts, Err(())))
                        }
                    }
                }
                _ => {
                    ts.extend(TokenStream::from(tt));
                    Ok(ts)
                }
            }) {
                Err((ts, Ok(body_ts))) => {
                    impl_ts.extend(ts.clone());
                    (ts, body_ts)
                }
                Err((ts, Err(()))) => {
                    impl_ts.extend(ts);
                    continue;
                }
                Ok(_) => break,
            };
        let mut sign_it = first_ts.into_iter().skip_while(|tt| match tt {
            Ident(id) if id.to_string() == "fn" => false,
            _ => true,
        });
        let _span = sign_it.next().unwrap().span();
        let (meth, args_gr) = match [sign_it.next(), sign_it.next()] {
            [Some(Ident(n)), Some(Group(gr))] if gr.delimiter() == Delimiter::Parenthesis => {
                (n.to_string(), gr)
            }
            [Some(Ident(n)), Some(Punct(p))] if p.to_string() == "<" => panic!(
                "Generic types in macro '#[methods_enum::gen()]' are not supported: fn {}()",
                n
            ),
            _ => {
                let enum_options: Vec<_> = enum_s.rsplit(&['(', ')'][..]).collect();
                panic!(
                    "Syntax error in macro '#[methods_enum::gen()]' {}",
                    if enum_options.len() == 1 {
                        "in first fn".to_string()
                    } else {
                        format!("after fn {}()", enum_options[2])
                    }
                )
            }
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

        let mut call_run = format!("{self_run_enum}{meth}({params}))");
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
        result_ts.extend(TokenStream::from_str("<'a>").unwrap());
    }
    result_ts.extend(TokenStream::from(Group(SctGroup::new(
        Delimiter::Brace,
        TokenStream::from_str(&enum_s).unwrap(),
    ))));
    result_ts.extend(
        TokenStream::from_str(
            &("
        #[allow(unused_must_use)]
        impl "
                .to_string()
                + &item_name),
        )
        .unwrap(),
    );
    result_ts.extend(TokenStream::from(Group(SctGroup::new(
        Delimiter::Brace,
        impl_ts,
    ))));

    if attr.dbg {
        println!("result_ts: \"{}\"", result_ts);
    }

    result_ts
}
