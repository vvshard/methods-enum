#![allow(unused)]

use core::str::FromStr;

use proc_macro::TokenTree::{self, Group, Ident, Punct};
use proc_macro::{Delimiter, Group as SctGroup, Punct as SctPunct, Spacing, TokenStream};

#[derive(Debug)]
enum Message {
    Warning,
    Error,
    Report,
}
use Message::{Error, Report, Warning};
impl Default for Message {
    fn default() -> Self {
        Report
    }
}

#[derive(Debug, Default)]
struct Messg {
    typ: Message,
    info: String,
}

#[derive(Debug, Default)]
struct Attr {
    enum_name: String,
    run_method: String,
    /// "" - none; "?" - conclusion of messages; "!" - panic! and conclusion of messages
    dbg: &'static str,
    out_name: String,
    def_name: String,
    def_type: Option<String>,
    diagnostics: Vec<Messg>,
}
impl Attr {
    fn new(attr_ts: TokenStream) -> Attr {
        let mut attr_it = attr_ts.into_iter();
        let (enum_name, dbg) = match attr_it.next() {
            Some(Ident(enum_n)) => (enum_n.to_string(), ""),
            Some(Punct(p)) if "?!".contains(&p.to_string()) => match attr_it.next() {
                Some(Ident(enum_n)) => (
                    enum_n.to_string(),
                    if p.to_string() == "?" { "?" } else { "!" },
                ),
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
    fn diagn(&mut self, typ: Message, info: String) {
        if self.dbg > "" {
            self.diagnostics.push(Messg { typ, info });
        }
    }
}

#[derive(Debug, Default)]
struct Meth {
    name: String,
    ts: TokenStream,
    out: TokenStream,
    body: Option<TokenStream>,
    params: String,
    typs: String,
}
impl Meth {
    fn args(&mut self, tt: TokenTree) -> Result<(), String> {
        match tt {
            Group(ref gr) => {
                let mut args_it = gr.stream().into_iter();
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
                                        self.params += ", ";
                                        self.typs += ", ";
                                    }
                                    self.params += &id.to_string();
                                }
                                _ => break,
                            }
                        }
                        Some(Punct(p)) if "<>".contains(&p.to_string()) => {
                            lg = lg + if p.to_string() == "<" { 1 } else { -1 };
                            self.typs += &p.to_string();
                        }
                        None => break,
                        Some(tt) => self.typs += &tt.to_string(),
                    };
                } // args loop
                self.ts.extend([tt]);
                Ok(())
            }
            _ => Err("(Group) must be transferred".to_string()),
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
            TokenTree::Literal(id) => println!("{indent}Literal:'{id}'"),
            Punct(id) => println!("{indent}Punct:'{id}'"),
        }
    }
}

#[proc_macro_attribute]
pub fn gen(attr_ts: TokenStream, item_ts: TokenStream) -> TokenStream {
    print_ts(&attr_ts, &item_ts);

    let mut attr = Attr::new(attr_ts);
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
            result_ts.extend([Ident(item_n)]);
            (item_name, gr.stream().into_iter(), gr.span())
        }
        m => panic!("SYNTAX ERROR: 'this attribute must be set on block impl': {m:?}"),
    };

    let mut diagnostics: Vec<Message> = Vec::new();
    let mut metods: Vec<Meth> = Vec::new();

    enum ParseStates {
        Stop,
        Start,
        Name,
        Args,
        Minus,
        Lg,
        Out,
    }
    use ParseStates::*;
    // metods loop
    let tail = loop {
        match impl_it.try_fold((Start, Meth::default()), |(state, mut m), tt| {
            match (state, tt) {
                (Start, Ident(ref id)) if id.to_string() == "fn" => {
                    m.ts.extend([tt]);
                    Ok((Name, m))
                }
                (Name, Ident(ref id)) => {
                    let nm = id.to_string();
                    m.ts.extend([tt]);
                    if nm == attr.run_method {
                        Err((Stop, m))
                    } else {
                        m.name = nm;
                        Ok((Args, m))
                    }
                }
                (Args, Group(ref gr)) if gr.delimiter() == Delimiter::Parenthesis => {
                    match m.args(tt) {
                        Ok(_) => Ok((Minus, m)),
                        Err(mess) => {
                            attr.diagn(Report, format!("skip fn {}: args: {}", m.name, mess));
                            Ok((Start, m))
                        }
                    }
                }
                (Minus, Punct(ref p)) if p.to_string() == "-" => {
                    m.ts.extend([tt]);
                    Ok((Lg, m))
                }
                (Lg, Punct(ref p)) if p.to_string() == ">" => {
                    m.ts.extend([tt]);
                    Ok((Out, m))
                }
                (Out, Ident(ref id)) if id.to_string() == "where" => {
                    attr.diagn(
                        Report,
                        format!("skip fn {}: \"where\" - generic method", m.name),
                    );
                    m.ts.extend([tt]);
                    m.out = TokenStream::new();
                    Ok((Start, m))
                }
                (Minus | Out, Punct(p)) if p.to_string() == ";" => Err((state, m)),
                (Minus | Out, Group(ref gr))
                    if gr.delimiter() == Delimiter::Brace && attr.def_name.is_empty() =>
                {
                    attr.diagn(
                        Report,
                        format!("skip fn {}: no default option specified", m.name),
                    );
                    m.ts.extend([tt]);
                    m.out = TokenStream::new();
                    Ok((Start, m))
                }
                (Minus | Out, Group(ref gr)) if gr.delimiter() == Delimiter::Brace => {
                    let mut gr_it = gr.stream().into_iter();
                    match [gr_it.next(), gr_it.next(), gr_it.next()] {
                        a @ [Some(Ident(def_n)), Some(Group(_)), Some(Punct(p))]
                        | a @ [Some(Ident(def_n)), Some(Punct(p)), Some(_)]
                            if def_n.to_string() == attr.def_name && p.to_string() == "=" =>
                        // replacing '=' with '=>' in body
                        {
                            let u = match a[1] {
                                Some(Punct(_)) => 1,
                                _ => 2,
                            };
                            let mut body =
                                TokenStream::from_iter(a.into_iter().take(u).map(|ot| ot.unwrap()));
                            body.extend([
                                Punct(SctPunct::new('=', Spacing::Joint)),
                                Punct(SctPunct::new('>', Spacing::Alone)),
                            ]);
                            if u == 2 {
                                body.extend([a[2].unwrap()]);
                            }
                            body.extend(gr_it);
                            m.body = Some(body);
                            Err((state, m))
                        }

                        _ => {
                            attr.diagn(Report,format!(
                                    "skip fn {}: no default option in body",
                                    m.name
                                ));
                            m.ts.extend([tt]);
                            m.out = TokenStream::new();
                            Ok((Start, m))
                        }
                    }
                }

                (Out, _) => {
                    m.out.extend((TokenStream::from(tt.clone())));
                    m.ts.extend([tt]);
                    Ok((state, m))
                }
                _ => {
                    if let Start = state {
                    } else {
                        attr.diagn(Report, format!("skip fn {}: {}", m.name, tt.to_string()));
                    }
                    m.ts.extend([tt]);
                    Ok((Start, m))
                }
            }
        }) {
            Ok((_, m)) | Err((Stop, m)) => break m.ts,
            Err((_, m)) => metods.push(m),
        };
    }; // metods loop

    enum_s = enum_s + &meth + "(";
    let mut params = String::new();
    let mut args_it = args_gr.stream().into_iter().skip_while(|tt| match tt {
        Punct(p) if p.to_string() == "," => false,
        _ => true,
    });

    let self_run_enum = format!("self.{}({}::", attr.run_method, attr.enum_name);

    if enum_s.contains('&') {
        enum_s = enum_s.replace('&', "&'a ");
        result_ts.extend(TokenStream::from_str("<'a>").unwrap());
    }
    result_ts.extend([Group(SctGroup::new(
        Delimiter::Brace,
        TokenStream::from_str(&enum_s).unwrap(),
    ))]);
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
    result_ts.extend([Group(SctGroup::new(Delimiter::Brace, impl_ts))]);

    if attr.dbg {
        println!("diagnostics: \"{}\"", diagnostics);
        println!("result_ts: \"{}\"", result_ts);
    }

    result_ts
}
