#![allow(unused)]

// region: debug
#[allow(unused)]
fn print_incoming_ts(attr_ts: &TokenStream, item_ts: &TokenStream) {
    println!("attr_ts: \"{}\"", attr_ts.to_string());
    unvrap_ts(attr_ts.clone(), 0);
    println!("item_ts: \"{}\"", item_ts.to_string());
    unvrap_ts(item_ts.clone(), 0);
}
#[allow(unused)]
fn unvrap_ts(ts: TokenStream, lvl: usize) {
    for tt in ts {
        let indent = "    ".repeat(lvl);
        match tt {
            Group(gr) => {
                println!("{indent}Group({:?}):", gr.delimiter());
                unvrap_ts(gr.stream(), lvl + 1);
            }
            Ident(id) => println!("{indent}Ident:{id}"),
            TokenTree::Literal(l) => println!("{indent}Literal:'{l}'"),
            Punct(p) => println!(
                "{indent}Punct({}):'{p}'",
                match p.spacing() {
                    Spacing::Alone => "Alone",
                    Spacing::Joint => "Joint",
                }
            ),
        }
    }
}
// endregion: debug

use core::str::FromStr;

use proc_macro::TokenTree::{self, Group, Ident, Punct};
use proc_macro::{
    Delimiter, Group as SGroup, Ident as SIdent, Punct as SPunct, Spacing, Span, TokenStream,
};

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
    strict_types: bool,
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
            [Some(Punct(p)), Some(Ident(out_n))] if p.to_string() == "=" => Attr {
                out_name: out_n.to_string(),
                strict_types: match attr_it.next() {
                    Some(Punct(p)) if p.to_string() == "!" => true,
                    _ => false,
                },
                ..attr
            },
            _ => panic!("{}??.. ", attr.err_in()),
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
    ident: Option<SIdent>,
    ts: TokenStream,
    out: TokenStream,
    default: TokenStream,
    params: String,
    typs: String,
}
impl Meth {
    fn args(&mut self, gr: SGroup) -> Result<(), String> {
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
impl ParseStates {
    fn expect(&self) -> &'static str {
        match self {
            Name => "function name",
            Args => "'('",
            Minus => "'-' or '{' or ';'",
            Lg => "'>'",
            _ => "",
        }
    }
}

//
//
//
//
//
//

#[proc_macro_attribute]
pub fn gen(attr_ts: TokenStream, item_ts: TokenStream) -> TokenStream {
    // print_ts(&attr_ts, &item_ts);

    let mut attr = Attr::new(attr_ts);

    let mut item_it = item_ts.into_iter();

    let mut block_ts = TokenStream::from_iter(item_it.by_ref().take_while(|tt| match tt {
        Ident(id) if id.to_string() == "impl" => false,
        _ => true,
    }));
    block_ts.extend([Ident(proc_macro::Ident::new(
        "impl",
        proc_macro::Span::call_site(),
    ))]);

    let mut item_it = match [item_it.next(), item_it.next(), item_it.next()]
    {
        [Some(Ident(item_n)), Some(Group(gr)), None] if gr.delimiter() == Delimiter::Brace => {
            block_ts.extend([Ident(item_n)]);
            gr.stream().into_iter()
        }
        m => panic!("SYNTAX ERROR: 'this attribute must be set on block impl without treyds and generics': {m:?}"),
    };

    let mut methods: Vec<Meth> = Vec::new();

    // filling for methods
    let tail = loop {
        match item_it.try_fold((Start, Meth::default()), |(state, mut m), tt| {
            match (state, tt) {
                (Start, Ident(id)) if id.to_string() == "fn" => {
                    m.ts.extend([Ident(id)]);
                    Ok((Name, m))
                }
                (Name, Ident(id)) => {
                    m.name = id.to_string();
                    m.ident = Some(id.clone());
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
                (Minus, Group(gr)) if gr.delimiter() == Delimiter::Brace => {
                    // skip fn with body
                    m.ts.extend([Group(gr)]);
                    Ok((Start, m))
                }
                (Out, Group(gr))
                    if gr.delimiter() == Delimiter::Brace && attr.out_name.is_empty() =>
                {
                    // skip fn with body
                    m.ts.extend([Group(gr)]);
                    m.out = TokenStream::new();
                    Ok((Start, m))
                }
                (Out, Ident(id)) if id.to_string() == "where" => {
                    // skip the generalized fn
                    m.ts.extend([Ident(id)]);
                    m.out = TokenStream::new();
                    Ok((Start, m))
                }
                (Minus | Out, Punct(p)) if p.to_string() == ";" => Err((state, m)),
                (Out, Group(gr)) if gr.delimiter() == Delimiter::Brace => {
                    m.default.extend(gr.stream());
                    Err((state, m))
                }
                (Out, tt) => {
                    m.out.extend(TokenStream::from(tt.clone()));
                    m.ts.extend([tt]);
                    Ok((state, m))
                }
                (st, tt) => {
                    if let Start = state {
                    } else {
                        attr.diagn(
                            Report,
                            format!("skip fn {}: expected- {}, found- {tt}", m.name, st.expect()),
                        );
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

    let out_ts =
        TokenStream::from_str("#[derive(Debug)] #[allow(non_camel_case_types)] enum ").unwrap();
    let mut result_ts: TokenStream = out_ts.clone();
    result_ts.extend([Ident(SIdent::new(&attr.enum_name, Span::call_site()))]);
    let live_ts = TokenStream::from_str("<'a>").unwrap();
    //                  (name.0, out.1, span.2)
    let mut outs: Vec<(String, String, Span)> = Vec::new();
    let mut enum_ts = TokenStream::new();
    let mut refs = false;
    for m in methods.iter() {
        enum_ts.extend([Ident(m.ident.as_ref().unwrap().clone())]);
        enum_ts.extend(TokenStream::from_str(&format!(
            "({}), ",
            if m.typs.contains('&') {
                refs = true;
                m.typs.replace('&', "&'a ")
            } else {
                m.typs.clone()
            }
        )));
        if !m.out.is_empty() {
            outs.push((
                m.name.clone(),
                m.out.to_string(),
                m.out.clone().into_iter().next().unwrap().span(),
            ));
        }
    }
    if refs {
        result_ts.extend(live_ts.clone());
    }
    result_ts.extend([Group(proc_macro::Group::new(Delimiter::Brace, enum_ts))]);

    if !attr.out_name.is_empty() {
        result_ts.extend(out_ts);
        result_ts.extend([Ident(SIdent::new(&attr.out_name, Span::call_site()))]);
        enum_ts = TokenStream::from_str("Unit, ").unwrap();
        refs = false;
        for (name, out, span) in outs.iter() {
            enum_ts.extend([Ident(SIdent::new(name, *span))]);
            enum_ts.extend(TokenStream::from_str(&format!(
                "({}), ",
                if out.contains('&') {
                    refs = true;
                    out.replace('&', "&'a ")
                } else {
                    out.clone()
                }
            )));
        }
        if refs {
            result_ts.extend(live_ts.clone());
        }
        result_ts.extend([Group(proc_macro::Group::new(Delimiter::Brace, enum_ts))]);
    }

    let self_run_enum = format!("self.{}({}::", attr.run_method, attr.enum_name);
    let varname = format!("_{}", attr.out_name.to_lowercase());
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
                body_ts.extend([Punct(SPunct::new(';', Spacing::Alone))]);
            }
        } else {
            body_ts.extend(TokenStream::from_str(&format!("match {call_run}")).unwrap());
            let out_enum = attr.out_name.clone() + "::";
            let out = m.out.to_string();
            let lside = if attr.strict_types {
                out_enum + &m.name + "(x)"
            } else {
                (outs.iter())
                    .filter_map(|(n, o, _)| (o == &out).then(|| out_enum.clone() + n + "(x)"))
                    .reduce(|s, n| s + " | " + &n)
                    .unwrap()
            };
            let mut match_ts =
                TokenStream::from_str(&format!("{lside} => x, {varname} => ")).unwrap();
            if m.default.is_empty() {
                match_ts.extend(
                    TokenStream::from_str(&format!(
                        "panic!(\"type mismatch: expected- {}, found- {{:?}}\", {varname})",
                        lside.replace("(x)", &format!("({out})"))
                    ))
                    .unwrap(),
                );
            } else {
                match_ts.extend(m.default);
            }
            body_ts.extend([Group(SGroup::new(Delimiter::Brace, match_ts))]);
        }
        metods_ts.extend([Group(SGroup::new(Delimiter::Brace, body_ts))]);
    }

    metods_ts.extend(tail);
    metods_ts.extend(item_it);

    block_ts.extend([Group(SGroup::new(Delimiter::Brace, metods_ts))]);

    result_ts.extend(block_ts);

    if attr.dbg > "" {
        println!("diagnostics: \n{:#?}", attr.diagnostics);
        println!("result_ts: \n{}", result_ts);
    }

    result_ts
}
