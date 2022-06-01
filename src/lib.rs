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
        let attr = Attr {
            enum_name,
            run_method,
            dbg,
            ..Default::default()
        };
        match [attr_it.next(), attr_it.next()] {
            [None, None] => attr,
            [Some(Punct(p)), Some(Ident(out_n))] if p.to_string() == "=" => {
                let attr = Attr {
                    out_name: out_n.to_string(),
                    ..attr
                };
                match [attr_it.next(), attr_it.next()] {
                    [None, None] => attr,
                    [Some(Punct(p)), Some(Ident(def_n))] if p.to_string() == "/" => Attr {
                        def_name: def_n.to_string(),
                        def_type: attr_it.next().map(|tt| match tt {
                            Group(type_gr) if type_gr.delimiter() == Delimiter::Parenthesis => {
                                type_gr.stream().to_string()
                            }
                            _ => panic!("{} = {out_n} / {def_n}??.. ", attr.err_in()),
                        }),
                        ..attr
                    },
                    _ => panic!("{} = {out_n}??.. ", attr.err_in()),
                }
            }
            _ => panic!("{}??.. ", attr.err_in()),
        }
    }

    fn def_opt(&self) -> String {
        match &self.def_type {
            Some(typ) => format!("{}({})", self.def_name, typ),
            None => self.def_name.clone(),
        }
    }
    fn err_in(&self) -> String {
        format!(
            "syntax error in attribute #[methods_enum::gen({}:{}",
            self.enum_name, self.run_method
        )
    }
}

#[derive(Debug, PartialEq, Default)]
struct Meth {
    enum_name: String,
    run_method: String,
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

    let mut item_it = item_ts.into_iter();

    let mut result_ts = TokenStream::from_iter(item_it.by_ref().take_while(|tt| match tt {
        Ident(id) if id.to_string() == "impl" => false,
        _ => true,
    }));
    // if no_impl {
    //     panic!("syntax error: 'this attribute must be set on block impl'")
    // }

    let (item_name, mut impl_it, impl_span) = match [item_it.next(), item_it.next(), item_it.next()]
    {
        [Some(Ident(item_n)), Some(Group(gr)), None] if gr.delimiter() == Delimiter::Brace => {
            let item_name = item_n.to_string();
            result_ts.extend(TokenStream::from(Ident(item_n)));
            (item_name, gr.stream().into_iter(), gr.span())
        }
        m => panic!("SYNTAX ERROR: 'this attribute must be set on block impl': {m:?}"),
    };

    let mut enum_s = String::new();
    let mut out_s = String::new();
    let mut diagnostics = String::new();
    let mut impl_ts = TokenStream::new();

    let self_run_enum = format!("self.{}({}::", attr.run_method, attr.enum_name);

    

    // metods loop
    loop {
        let (meth, mut sign_it, body_ts) = match impl_it.try_fold(
            (TokenStream::new(), TokenStream::new(), String::new()),
            |(mut ts, mut sign_ts, nm), tt| match tt {
                Ident(ref id) if nm == "" && id.to_string() == "fn" => {
                    ts.extend(TokenStream::from(tt));
                    Ok((ts, sign_ts, "***".to_string()))
                }
                Ident(ref id) if nm == "***" => {
                    let nm = id.to_string();
                    ts.extend(TokenStream::from(tt));
                    if nm == attr.run_method {
                        Err((ts, sign_ts, nm, None))
                    } else {
                        Ok((ts, sign_ts, nm))
                    }
                }
                _ if nm == "***" => {
                    if attr.dbg {
                        diagnostics += "\n! SYNTAX ERROR: fn??"
                    }
                    ts.extend(TokenStream::from(tt));
                    Ok((ts, TokenStream::new(), String::new()))
                }
                Punct(p) if nm != "" && p.to_string() == ";" => Err((ts, sign_ts, nm, None)),
                Ident(ref id) if nm != "" && id.to_string() == "where" => {
                    if attr.dbg {
                        diagnostics += &format!("\nskip fn {nm}: \"where\" - generic method '{p}' ");
                    }
                    ts.extend(TokenStream::from(tt));
                    Ok((ts, TokenStream::new(), String::new()))
                }
                Group(ref gr)
                    if nm != ""
                        && gr.delimiter() == Delimiter::Brace
                        && attr.def_name.is_empty() =>
                {
                    if attr.dbg {
                        diagnostics += &format!("\nskip fn {nm}: no default option specified");
                    }
                    ts.extend(TokenStream::from(tt));
                    Ok((ts, TokenStream::new(), String::new()))
                }
                Group(ref gr) if nm != "" && gr.delimiter() == Delimiter::Brace => {
                    let mut it3 = gr.stream().into_iter().take(3);
                    match [it3.next(), it3.next(), it3.next()] {
                        [Some(Ident(def_n)), Some(Group(_)), Some(Punct(p))]
                        | [Some(Ident(def_n)), Some(Punct(p)), Some(_)]
                            if def_n.to_string() == attr.def_name && p.to_string() == "=" =>
                        {
                            Err((ts, sign_ts, nm, Some(gr.stream())))
                        }
                        _ => {
                            if attr.dbg {
                                diagnostics +=
                                    &format!("\nskip fn {nm}: no default option in body");
                            }
                            ts.extend(TokenStream::from(tt));
                            Ok((ts, TokenStream::new(), String::new()))
                        }
                    }
                }
                _ => {
                    if nm != "" {
                        sign_ts.extend(TokenStream::from(tt.clone()))
                    }
                    ts.extend(TokenStream::from(tt));
                    Ok((ts, sign_ts, nm))
                }
            },
        ) {
            Ok((ts, _, _)) => {
                impl_ts.extend(ts);
                break;
            }
            Err((ts, _, nm, None)) if nm == attr.run_method => {
                impl_ts.extend(ts);
                impl_ts.extend(impl_it);
                break;
            }
            Err((ts, sign_ts, nm, body_ts)) => {
                impl_ts.extend(ts);
                (nm, sign_ts.into_iter(), body_ts)
            }
        };
        // let _span = sign_it.next().unwrap().span();
        let args_gr = sign_it.next();
        let args_gr = match args_gr {
            Some(Group(gr)) if gr.delimiter() == Delimiter::Parenthesis => gr,
            _ => {
                if attr.dbg {
                    match args_gr {
                        Some(Punct(p)) if p.to_string() == "<" => {
                            diagnostics +=  &format!("\n!SKIP fn {meth}: generic methods in macro '#[methods_enum::gen(...' are not supported");
                        }
                        _ => diagnostics += &format!("\n!SYNTAX ERROR in fn {meth}"),
                    }
                }
                match body_ts {
                    Some(ts) => result_ts.extend(TokenStream::from(Group(SctGroup::new(
                        Delimiter::Brace,
                        ts,
                    )))),
                    None => result_ts.extend(TokenStream::from_str(";")),
                }
                continue;
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
            call_run = "#![allow(unused_must_use)] ".to_string() + &call_run + ";";
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
        println!("diagnostics: \"{}\"", diagnostics);
        println!("result_ts: \"{}\"", result_ts);
    }

    result_ts
}
