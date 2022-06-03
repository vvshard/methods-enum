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

#[derive(Debug)]
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
    fn def_arm(&self, out: &str) -> String {
        if out.starts_with("Result") {
            match self.def_type {
                Some(_) => format!("{}(x) => Err(x),", self.def_name),
                None => format!("{} => Err(()),", self.def_name),
            }
        } else {
            match self.def_type {
                Some(_) => format!("{}(_) => <{out}>::default(),", self.def_name),
                None => format!("{} => <{out}>::default(),", self.def_name),
            }
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
    span: Option<proc_macro::Span>,
    ts: TokenStream,
    out: TokenStream,
    body: TokenStream,
    params: String,
    typs: String,
}
impl Meth {
    fn args(&mut self, gr: SctGroup) -> Result<(), String> {
        let mut args_it = gr.stream().into_iter();
        let mut lg = 0;
        let mut first = true;
        self.params = String::new();
        self.typs = String::new();
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
                        [Some(tt), _] => return Err(tt.to_string()),
                        [None, _] => break,
                    }
                }
                Some(Punct(p)) if "<>".contains(&p.to_string()) => {
                    lg = lg + if p.to_string() == "<" { 1 } else { -1 };
                    self.typs += &p.to_string();
                }
                Some(Ident(id)) if id.to_string() == "impl" => {
                    return Err("generalized arg: 'impl'".to_string())
                }
                Some(tt) if !first => self.typs += &tt.to_string(),
                None => break,
                _ => (),
            };
        }
        self.ts.extend([Group(gr)]);
        Ok(())
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
    // print_ts(&attr_ts, &item_ts);

    let mut attr = Attr::new(attr_ts);
    // dbg!(&attr);

    let mut item_it = item_ts.into_iter();

    let mut result_ts = TokenStream::from_iter(item_it.by_ref().take_while(|tt| match tt {
        Ident(id) if id.to_string() == "impl" => false,
        _ => true,
    }));
    result_ts.extend([Ident(proc_macro::Ident::new(
        "impl",
        proc_macro::Span::call_site(),
    ))]);

    let (item_name, mut impl_it, impl_span) = match [item_it.next(), item_it.next(), item_it.next()]
    {
        [Some(Ident(item_n)), Some(Group(gr)), None] if gr.delimiter() == Delimiter::Brace => {
            let item_name = item_n.to_string();
            result_ts.extend([Ident(item_n)]);
            (item_name, gr.stream().into_iter(), gr.span())
        }
        m => panic!("SYNTAX ERROR: 'this attribute must be set on block impl without treyds and generics': {m:?}"),
    };

    let mut methods: Vec<Meth> = Vec::new();

    #[derive(Clone, Copy)]
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
    // filling for methods
    let tail = loop {
        match impl_it.try_fold((Start, Meth::default()), |(state, mut m), tt| {
            match (state, tt) {
                (Start, Ident(id)) if id.to_string() == "fn" => {
                    m.ts.extend([Ident(id)]);
                    Ok((Name, m))
                }
                (Name, Ident(id)) => {
                    m.name = id.to_string();
                    m.span = Some(id.span());
                    m.ts.extend([Ident(id)]);
                    if m.name == attr.run_method {
                        Err((Stop, m))
                    } else {
                        Ok((Args, m))
                    }
                }
                (Args, Group(gr)) if gr.delimiter() == Delimiter::Parenthesis => match m.args(gr) {
                    Ok(_) => Ok((Minus, m)),
                    Err(mess) => {
                        attr.diagn(Report, format!("skip fn {}: args: {}", m.name, mess));
                        Ok((Start, m))
                    }
                },
                (Minus, Punct(p)) if p.to_string() == "-" => {
                    m.ts.extend([Punct(p)]);
                    Ok((Lg, m))
                }
                (Lg, Punct(p)) if p.to_string() == ">" => {
                    m.ts.extend([Punct(p)]);
                    Ok((Out, m))
                }
                (Out, Ident(id)) if id.to_string() == "where" => {
                    // skip the generalized fn
                    m.ts.extend([Ident(id)]);
                    m.out = TokenStream::new();
                    Ok((Start, m))
                }
                (Minus | Out, Punct(p)) if p.to_string() == ";" => Err((state, m)),
                (Minus | Out, Group(gr))
                    if gr.delimiter() == Delimiter::Brace && attr.def_name.is_empty() =>
                {
                    // skip fn with body
                    m.ts.extend([Group(gr)]);
                    m.out = TokenStream::new();
                    Ok((Start, m))
                }
                (Minus | Out, Group(gr)) if gr.delimiter() == Delimiter::Brace => {
                    let mut gr_it = gr.stream().into_iter();
                    let a = [gr_it.next(), gr_it.next(), gr_it.next()];
                    match a {
                        [Some(Ident(ref def_n)), Some(Group(_)), Some(Punct(ref p))]
                        | [Some(Ident(ref def_n)), Some(Punct(ref p)), Some(_)]
                            if def_n.to_string() == attr.def_name
                                && p.to_string() == "="
                                && p.spacing() == Spacing::Alone =>
                        {
                            // replacing '=' with '=>' in the body
                            let mut body =
                                TokenStream::from_iter(a.into_iter().flat_map(|optt| match optt {
                                    Some(Punct(p)) if p.to_string() == "=" => vec![
                                        Punct(SctPunct::new('=', Spacing::Joint)),
                                        Punct(SctPunct::new('>', Spacing::Alone)),
                                    ],
                                    _ => vec![optt.unwrap()],
                                }));
                            body.extend(gr_it);
                            m.body = body;
                            Err((state, m))
                        }

                        _ => {
                            // skips fn with a body starting not with the default option
                            m.ts.extend([Group(gr)]);
                            m.out = TokenStream::new();
                            Ok((Start, m))
                        }
                    }
                }
                (Out, tt) => {
                    m.out.extend((TokenStream::from(tt.clone())));
                    m.ts.extend([tt]);
                    Ok((state, m))
                }
                (_, tt) => {
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
            Err((_, m)) => methods.push(m),
        };
    }; // filling for methods

    // println!("methods: {:#?}", methods);


    let mut options = String::new();
    //                  (name, out)
    let mut outs: Vec<(String, String)> = Vec::new();
    let mut no_out = "";
    for m in methods.iter() {
        options = options + &m.name + "(" + &m.typs + "), ";
        if !attr.out_name.is_empty() {
            if m.out.is_empty() {
                no_out = "__(()), ";
            } else {
                outs.push((m.name.clone(), m.out.to_string()));
            }
        }
    }

    let self_run_enum = format!("self.{}({}::", attr.run_method, attr.enum_name);

    let mut metods_ts = TokenStream::new();
    for m in methods {
        metods_ts.extend(m.ts);
        let call_run = format!("{self_run_enum}{}({}))", m.name, m.params);
        let mut body_ts = match m.out.is_empty() {
            true => TokenStream::from_str("#![allow(unused_must_use)]").unwrap(),
            false => TokenStream::new(),
        };
        if attr.out_name.is_empty() || m.out.is_empty() {
            body_ts.extend(TokenStream::from_str(&call_run).unwrap());
            if m.out.is_empty() {
                body_ts.extend([Punct(SctPunct::new(';', Spacing::Alone))]);
            }
        } else {
            let out = m.out.to_string();
            let lside = (outs.iter())
                .filter_map(|(n, o)| (o == &out).then(|| n.clone() + "(x)"))
                .reduce(|s, n| s + " | " + &n)
                .unwrap();
            let mut match_ts = TokenStream::from_str(&format!("{lside} => x,")).unwrap();
            if m.body.is_empty() {
                match_ts.extend(TokenStream::from_str(&attr.def_arm(&out)).unwrap());
            } else {
                match_ts.extend(m.body);
            }
            match_ts.extend(TokenStream::from_str("_ => panic!(\"type mismatch\")").unwrap());

            body_ts.extend(TokenStream::from_str(&format!("match {call_run}")).unwrap());
            body_ts.extend([Group(SctGroup::new(Delimiter::Brace, match_ts))]);
        }
        metods_ts.extend([Group(SctGroup::new(Delimiter::Brace, body_ts))]);
    }

    metods_ts.extend(tail);
    metods_ts.extend(impl_it);

    result_ts.extend([Group(SctGroup::new(Delimiter::Brace, metods_ts))]);

    // enum / enums
    result_ts.extend(TokenStream::from_str("#[derive(Debug)]").unwrap());
    add_enum(&mut result_ts, &attr.enum_name, options);

    if !attr.out_name.is_empty() {
        options = (outs.iter()).fold(String::new(), |s, (name, out)| s + name + "(" + out + "), ")
            + no_out
            + &attr.def_opt();
        add_enum(&mut result_ts, &attr.out_name, options);
    }

    if attr.dbg > "" {
        println!("diagnostics: \"{:?}\"", attr.diagnostics);
        println!("result_ts: \"{}\"", result_ts);
    }

    result_ts
}

fn add_enum(ts: &mut TokenStream, name: &str, mut options: String) {
    ts.extend(
        TokenStream::from_str(&("#[allow(non_camel_case_types)] enum ".to_string() + name))
            .unwrap(),
    );
    if options.contains('&') {
        options = options.replace('&', "&'a ");
        ts.extend(TokenStream::from_str("<'a>").unwrap());
    }
    ts.extend([Group(proc_macro::Group::new(
        Delimiter::Brace,
        TokenStream::from_str(&options).unwrap(),
    ))]);
}
