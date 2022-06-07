//! By signatures from methods without bodies, are formed:
//! an enum with options named as methods tuples, corresponding for them arguments,
//! and bodies for that methods calls for handler method this enum of tuples with parameters.

// // region: debug
// #[allow(unused)]
// // output is shorter than dbg!()
// fn print_incoming_ts(attr_ts: &TokenStream, item_ts: &TokenStream) {
//     println!("attr_ts: \"{}\"", attr_ts.to_string());
//     unvrap_ts(attr_ts.clone(), 0);
//     println!("item_ts: \"{}\"", item_ts.to_string());
//     unvrap_ts(item_ts.clone(), 0);
// }
// #[allow(unused)]
// fn unvrap_ts(ts: TokenStream, lvl: usize) {
//     for tt in ts {
//         let indent = "    ".repeat(lvl);
//         match tt {
//             Group(gr) => {
//                 println!("{indent}Group({:?}):", gr.delimiter());
//                 unvrap_ts(gr.stream(), lvl + 1);
//             }
//             Ident(id) => println!("{indent}Ident:{id}"),
//             TokenTree::Literal(l) => println!("{indent}Literal:'{l}'"),
//             Punct(p) => println!(
//                 "{indent}Punct({}):'{p}'",
//                 match p.spacing() {
//                     Spacing::Alone => "Alone",
//                     Spacing::Joint => "Joint",
//                 }
//             ),
//         }
//     }
// }
// // endregion: debug

use core::str::FromStr;

use proc_macro::token_stream::IntoIter;
use proc_macro::TokenTree::{Group, Ident, Punct};
use proc_macro::{
    Delimiter, Group as SGroup, Ident as SIdent, Punct as SPunct, Spacing, Span, TokenStream,
};

#[derive(Debug, Default)]
struct Attr {
    enum_name: String,
    enum_ident: Option<SIdent>,
    run_method: String,
    out_ident: Option<SIdent>,
    strict_types: bool,
}
impl Attr {
    fn new(attr_ts: TokenStream) -> Attr {
        let mut attr_it = attr_ts.into_iter();
        let (enum_id, run_method) = match [attr_it.next(), attr_it.next(), attr_it.next()] {
            [Some(Ident(enum_id)), Some(Punct(p)), Some(Ident(run_method_id))]
                if p.to_string() == ":" =>
            {
                (enum_id, run_method_id.to_string())
            }
            _ => panic!("syntax error in attribute #[methods_enum::gen(?? "),
        };
        let attr = Attr {
            enum_name: enum_id.to_string(),
            enum_ident: Some(enum_id.clone()),
            run_method,
            ..Default::default()
        };
        match [attr_it.next(), attr_it.next()] {
            [None, None] => attr,
            [Some(Punct(p)), None] if p.to_string() == "," => attr,
            [Some(Punct(p)), Some(Ident(out_id))] if p.to_string() == "=" => Attr {
                out_ident: Some(out_id.clone()),
                strict_types: match attr_it.next() {
                    Some(Punct(p)) if p.to_string() == "!" => true,
                    _ => false,
                },
                ..attr
            },
            _ => panic!(
                "syntax error in attribute #[methods_enum::gen({}:{}??..",
                attr.enum_name, attr.run_method
            ),
        }
    }
}

#[derive(Clone, Copy)]
enum ParseStates {
    Stop,
    Start,
    Name,
    Args,
    Minus,
    Gt,
    Out,
}
use ParseStates::*;

#[derive(Debug, Default)]
struct Meth {
    ident: Option<SIdent>,
    ts: TokenStream,
    out: TokenStream,
    default: TokenStream,
    params: String,
    typs: String,
}
impl Meth {
    fn args(&mut self, gr: SGroup) -> bool {
        let mut args_it = gr.stream().into_iter();
        self.ts.extend([Group(gr)]);
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
                        [Some(_tt), _] => break false,
                        [None, _] => break true,
                    }
                }
                Some(Punct(p)) if "<>".contains(&p.to_string()) => {
                    lg = lg + if p.to_string() == "<" { 1 } else { -1 };
                    self.typs += &p.to_string();
                }
                Some(Ident(id)) if id.to_string() == "impl" => break false,
                Some(Ident(id)) if !first && id.to_string() == "mut" => {
                    self.typs += "mut ";
                }
                Some(tt) if !first => self.typs += &tt.to_string(),
                None => break true,
                _ => (),
            };
        }
    }

    fn filling_vec(iit: &mut IntoIter, attr: &mut Attr) -> Vec<Meth> {
        let mut methods: Vec<Meth> = Vec::new();
        loop {
            match iit.try_fold((Start, Meth::default()), |(state, mut m), tt| {
                match (state, tt) {
                    (Start, Ident(id)) if id.to_string() == "fn" => {
                        m.ts.extend([Ident(id)]);
                        Ok((Name, m))
                    }
                    (Name, Ident(id)) => {
                        m.ts.extend([Ident(id.clone())]);
                        if id.to_string() == attr.run_method {
                            Err((Stop, m))
                        } else {
                            m.ident = Some(id.clone());
                            Ok((Args, m))
                        }
                    }
                    (Args, Group(gr)) if gr.delimiter() == Delimiter::Parenthesis => {
                        if m.args(gr) {
                            Ok((Minus, m))
                        } else {
                            Ok((Start, m))
                        }
                    }
                    (Minus, Punct(p)) if p.to_string() == "-" => {
                        m.ts.extend([Punct(p)]);
                        Ok((Gt, m))
                    }
                    (Gt, Punct(p)) if p.to_string() == ">" => {
                        m.ts.extend([Punct(p)]);
                        Ok((Out, m))
                    }
                    (Minus, Group(gr)) if gr.delimiter() == Delimiter::Brace => {
                        // skip fn with body
                        m.ts.extend([Group(gr)]);
                        Ok((Start, m))
                    }
                    (Out, Group(gr))
                        if gr.delimiter() == Delimiter::Brace && attr.out_ident.is_none() =>
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
                    (_, tt) => {
                        m.ts.extend([tt]);
                        Ok((Start, m))
                    }
                }
            }) {
                Ok((_, mut m)) | Err((Stop, mut m)) => {
                    m.ident = None;
                    methods.push(m);
                    break methods;
                }
                Err((_, m)) => methods.push(m),
            };
        }
    }
}

/// By signatures from methods without bodies, are formed:
/// an enum with options named as methods tuples, corresponding for them arguments,
/// and bodies for that methods calls for handler method this enum of tuples with parameters.
///
/// This allows the handler method to manipulate the behavior of the methods depending on the context.
///
/// There are two options syntaxes:
///
/// 1- For case when methods that return a value have the same return type:
///
/// `#[methods_enum::gen(`*EnumName*`: `*handler_name*`)]`
///
/// where:
/// - *EnumName*: The name of the automatically accepted enumeration.
/// - *handler_name*: name of the handler method
///
/// 2- For the case of more than one meaningful return type:
///
/// `#[methods_enum::gen(`*EnumName*`: `*handler_name*` = `*OutName*`)]`
///
/// where - *OutName*: the name of the automatically retrieved enum
/// with method-named options single-tuples of the return type.
///
/// In this case, you can also specify default return value expressions in the method signature.
///
/// For more details, see the [module documentation](self)
#[proc_macro_attribute]
pub fn gen(attr_ts: TokenStream, item_ts: TokenStream) -> TokenStream {
    // print_incoming_ts(&attr_ts, &item_ts);

    let mut attr = Attr::new(attr_ts);

    let mut item_it = item_ts.into_iter();

    let mut item_ts = TokenStream::from_iter(item_it.by_ref().take_while(|tt| match tt {
        Ident(id) if id.to_string() == "impl" => false,
        _ => true,
    }));
    item_ts.extend([Ident(proc_macro::Ident::new(
        "impl",
        proc_macro::Span::call_site(),
    ))]);

    let mut block_it = match [item_it.next(), item_it.next(), item_it.next()]
    {
        [Some(Ident(item_n)), Some(Group(gr)), None] if gr.delimiter() == Delimiter::Brace => {
            item_ts.extend([Ident(item_n)]);
            gr.stream().into_iter()
        }
        m => panic!("SYNTAX ERROR: 'this attribute must be set on block impl without treyds and generics': {m:?}"),
    };

    let head_enum = TokenStream::from_str(
        r#"
        #[derive(Debug)] 
        #[allow(non_camel_case_types)]
        #[doc = "formed by macro `#[methods_enum::gen(...)]`"]
        enum "#,
    )
    .unwrap();

    let methods = Meth::filling_vec(&mut block_it, &mut attr);

    let mut result_ts: TokenStream = head_enum.clone();
    result_ts.extend([Ident(attr.enum_ident.unwrap())]);

    let live_ts = TokenStream::from_str("<'a>").unwrap();
    //                  (name.0, out.1, span.2)
    let mut outs: Vec<(String, String, Span)> = Vec::new();
    let mut enum_ts = TokenStream::new();
    let mut refs = false;
    for m in methods.iter() {
        if let Some(ident) = &m.ident {
            enum_ts.extend([Ident(ident.clone())]);
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
                    ident.to_string(),
                    m.out.to_string(),
                    m.out.clone().into_iter().next().unwrap().span(),
                ));
            }
        }
    }
    if refs {
        result_ts.extend(live_ts.clone());
    }
    result_ts.extend([Group(proc_macro::Group::new(Delimiter::Brace, enum_ts))]);

    if let Some(out_ident) = &attr.out_ident {
        result_ts.extend(head_enum);
        result_ts.extend([Ident(out_ident.clone())]);
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
            result_ts.extend(live_ts);
        }
        result_ts.extend([Group(proc_macro::Group::new(Delimiter::Brace, enum_ts))]);
    }

    let self_run_enum = format!("self.{}({}::", attr.run_method, attr.enum_name);
    let varname = match &attr.out_ident {
        Some(out_ident) => format!("_{}", out_ident).to_lowercase(),
        None => String::new(),
    };
    let mut metods_ts = TokenStream::new();
    for m in methods {
        metods_ts.extend(m.ts);
        if let Some(ident) = m.ident {
            let call_run = format!("{self_run_enum}{ident}({}))", m.params);
            let mut body_ts = match m.out.is_empty() {
                true => TokenStream::from_str("#![allow(unused_must_use)]").unwrap(),
                false => TokenStream::new(),
            };
            if attr.out_ident.is_none() || m.out.is_empty() {
                body_ts.extend(TokenStream::from_str(&call_run).unwrap());
                if m.out.is_empty() {
                    body_ts.extend([Punct(SPunct::new(';', Spacing::Alone))]);
                }
            } else if let Some(out_ident) = &attr.out_ident {
                body_ts.extend(TokenStream::from_str(&format!("match {call_run}")).unwrap());
                let out_enum = out_ident.to_string() + "::";
                let out = m.out.to_string();
                let lside = if attr.strict_types {
                    format!("{out_enum}{ident}(x)")
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
                            "panic!(\"type mismatch in {ident}() metod: expected- {}, \
                            found- {out_enum}{{:?}}\", {varname})",
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
    }
    metods_ts.extend(block_it);
    item_ts.extend([Group(SGroup::new(Delimiter::Brace, metods_ts))]);

    result_ts.extend(item_ts);

    // println!("result_ts: \n{}\n", result_ts);

    result_ts
}
