#![doc = include_str!("../README.md")]
//! see description in [crate documentation](crate)
#![doc(html_playground_url = "https://play.rust-lang.org/")]

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
            [Some(Ident(enum_id)), Some(Punct(p)), Some(Ident(run_id))] if p.to_string() == ":" => {
                (enum_id, run_id.to_string())
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
    Pub,
    Name,
    Args,
    Minus,
    Gt,
    Out,
}
use ParseStates::*;

#[derive(Default)]
struct Meth {
    ident: Option<SIdent>,
    ts: TokenStream,
    pub_s: &'static str,
    args: String,
    params: String,
    typs: String,
    gt_span: Option<Span>,
    out: TokenStream,
    default: TokenStream,
}
impl Meth {
    /// on successful parsing of the arguments returns `Minus`, otherwise - `Start`
    fn args_parsing(&mut self, args_gr: SGroup) -> ParseStates {
        self.args = (args_gr.stream().to_string())
            .replace("& ", "&")
            .replace(" :", ":")
            .replace(" ,", ",")
            .replace(" <", "<")
            .replace(" >", ">");
        let mut args_it = args_gr.stream().into_iter();
        self.ts.extend([Group(args_gr)]);
        self.gt_span = None;
        self.out = TokenStream::new();
        let mut lg = 0;
        let mut first = true;
        let mut is_self = false;
        self.params = String::new();
        self.typs = String::new();
        loop {
            match args_it.next() {
                Some(Punct(p)) if p.to_string() == "," && lg == 0 => {
                    match [args_it.next(), args_it.next()] {
                        [Some(Ident(id)), Some(Punct(p))] if p.to_string() == ":" => {
                            if first {
                                if !is_self {
                                    break Start;
                                }
                                first = false;
                            } else {
                                self.params.push_str(", ");
                                self.typs.push_str(", ");
                            }
                            self.params += &id.to_string();
                        }
                        [Some(_tt), _] => break Start,
                        [None, _] => break Minus,
                    }
                }
                Some(Punct(p)) if "<>".contains(&p.to_string()) => {
                    lg = lg + if p.to_string() == "<" { 1 } else { -1 };
                    self.typs += &p.to_string();
                }
                Some(Ident(id)) if id.to_string() == "impl" => break Start,
                Some(Ident(id)) if first && id.to_string() == "self" => is_self = true,
                Some(Ident(id)) if !first && id.to_string() == "mut" => {
                    self.typs.push_str("mut ");
                }
                Some(tt) if !first => self.typs.push_str(&tt.to_string()),
                None => break Minus,
                _ => (),
            }
        }
    }

    fn filling_vec(iit: &mut IntoIter, attr: &Attr) -> Vec<Meth> {
        let mut methods: Vec<Meth> = Vec::new();
        loop {
            match iit.try_fold((Start, Meth::default()), |(state, mut m), tt| {
                match (state, tt) {
                    (Start, Ident(id)) if id.to_string() == "pub" => {
                        m.ts.extend([Ident(id)]);
                        Ok((Pub, m))
                    }
                    (st @ (Start | Pub), Ident(id)) if id.to_string() == "fn" => {
                        m.pub_s = if let Start = st { "" } else { "pub " };
                        m.ts.extend([Ident(id)]);
                        Ok((Name, m))
                    }
                    (Name, Ident(id)) => {
                        m.ts.extend([Ident(id.clone())]);
                        if id.to_string() == attr.run_method {
                            Err((Stop, m))
                        } else {
                            m.ident = Some(id);
                            Ok((Args, m))
                        }
                    }
                    (Args, Group(gr)) if gr.delimiter() == Delimiter::Parenthesis => {
                        Ok((m.args_parsing(gr), m))
                    }
                    (Minus, Punct(p)) if p.to_string() == "-" => {
                        m.ts.extend([Punct(p)]);
                        Ok((Gt, m))
                    }
                    (Gt, Punct(p)) if p.to_string() == ">" => {
                        m.gt_span = Some(p.span());
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
                        Ok((Start, m))
                    }
                    (Out, Ident(id)) if id.to_string() == "where" => {
                        // skip the generalized fn
                        m.ts.extend([Ident(id)]);
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

/// Based on the signatures of the methods of the `impl` block, the are formed:
/// `enum` with options from the tuples of arguments, and
/// `{}` bodies of these methods with a call to the argument handler method from this `enum`.
///
/// This allows the handler method to control the behavior of the methods depending on the context.
///
/// There are two syntax options:
///
/// 1- For the case where methods returning a value have the same return type:
///
/// **`#[methods_enum::gen(`*EnumName*`: `*handler_name*`)]`**
///
/// where:
/// - *EnumName*: the name of the automatically generated enum.
/// - *handler_name*: handler method name
///
/// 2- In case of more than one meaningful return type:
///
/// **`#[methods_enum::gen(`*EnumName*`: `*handler_name*` = `*OutName*`)]`**
///
/// where:
///  - *OutName*: the name of the automatically generated enum with options from single tuples of return types.
///
/// In this case, you can also specify default return value expressions.
///
/// See the [module documentation](crate) for details.
#[proc_macro_attribute]
pub fn gen(attr_ts: TokenStream, item_ts: TokenStream) -> TokenStream {
    // std::fs::write("target/debug/item_ts.txt", format!("{}\n\n{0:#?}", item_ts)).unwrap();

    let attr = Attr::new(attr_ts);

    let mut item_it = item_ts.into_iter();

    let mut item_ts = TokenStream::from_iter(item_it.by_ref().take_while(|tt| match tt {
        Ident(id) if id.to_string() == "impl" => false,
        _ => true,
    }));
    item_ts.extend([Ident(SIdent::new("impl", Span::call_site()))]);

    let mut block_it = match [item_it.next(), item_it.next(), item_it.next()] {
        [Some(Ident(item_n)), Some(Group(gr)), None] if gr.delimiter() == Delimiter::Brace => {
            item_ts.extend([Ident(item_n)]);
            gr.stream().into_iter()
        }
        m => panic!(
            "SYNTAX ERROR: 
        'this attribute must be set on block impl without treyds and generics': {m:?}"
        ),
    };

    let methods = Meth::filling_vec(&mut block_it, &attr);

    let head_enum = r##"
        #[allow(non_camel_case_types)]
        #[derive(Debug)] 
        /// formed by macro `#[methods_enum::gen(...)]`:
        /// ```
        /// #[allow(non_camel_case_types)]
        /// #[derive(Debug)]
        #[doc = "enum "##;
    //                 (name.0, out.1, span.2)
    let mut outs: Vec<(String, String, Span)> = Vec::new();
    let mut enum_ts = TokenStream::new();
    let mut enum_doc = String::new();
    let mut refs = "";
    for m in methods.iter() {
        if let Some(ident) = &m.ident {
            enum_ts.extend([Ident(ident.clone())]);
            let typs = if m.typs.contains('&') {
                refs = "<'a>";
                m.typs.replace('&', "&'a ")
            } else {
                m.typs.clone()
            };
            enum_ts.extend(TokenStream::from_str(&format!("({typs}), ")));
            enum_doc.push_str(&format!("\n    {ident}({typs}), "));
            if let Some(gt_span) = m.gt_span {
                outs.push((
                    ident.to_string(),
                    (m.out.to_string())
                        .replace("& ", "&")
                        .replace(" ,", ",")
                        .replace(" <", "<")
                        .replace(" >", ">"),
                    gt_span,
                ));
            }
        }
    }
    enum_doc.push_str("\n}\n\n");
    enum_doc.push_str(
        r#"//--- fn bodies generated by the `#[methods_enum::gen(...)]` macro: ---
    "#,
    );

    let is_result = attr.out_ident.is_none() && outs.iter().any(|t| t.1.contains("Result<"));
    let self_run_enum = format!("self.{}({}::", attr.run_method, attr.enum_name);
    let mut metods_ts = TokenStream::new();
    for m in methods {
        metods_ts.extend(m.ts);
        if let Some(ident) = m.ident {
            enum_doc.push_str(&format!("\n{}fn {ident}({})", m.pub_s, m.args));
            let mut body_ts = TokenStream::new();
            let out = if m.out.is_empty() {
                enum_doc.push_str("{");
                if is_result {
                    enum_doc.push_str("\n    #![allow(unused_must_use)]");
                    body_ts.extend(TokenStream::from_str("#![allow(unused_must_use)]").unwrap());
                }
                String::new()
            } else {
                let name = ident.to_string();
                let find_out = outs.iter().find(|t| t.0 == name).unwrap().1.clone();
                enum_doc.push_str(&format!(" -> {find_out}{{"));
                find_out
            };
            let call_run = format!("{self_run_enum}{ident}({}))", m.params);
            if attr.out_ident.is_none() || m.out.is_empty() {
                enum_doc.push_str(&format!("\n    {call_run}"));
                body_ts.extend(TokenStream::from_str(&call_run).unwrap());
                if m.out.is_empty() {
                    enum_doc.push_str(";");
                    body_ts.extend([Punct(SPunct::new(';', Spacing::Alone))]);
                }
            } else if let Some(out_ident) = &attr.out_ident {
                enum_doc.push_str(&format!("\n    match {call_run} {{"));
                body_ts.extend(TokenStream::from_str(&format!("match {call_run}")).unwrap());
                let out_enum = out_ident.to_string() + "::";
                let varname = format!("_{}", out_ident).to_lowercase();
                let lside = if attr.strict_types {
                    format!("{out_enum}{ident}(x)")
                } else {
                    (outs.iter())
                        .filter_map(|(n, o, _)| (o == &out).then(|| out_enum.clone() + n + "(x)"))
                        .reduce(|s, n| s + " | " + &n)
                        .unwrap()
                };
                enum_doc.push_str(&format!("\n        {lside} => x,\n        {varname} => "));
                let mut match_ts =
                    TokenStream::from_str(&format!("{lside} => x, {varname} => ")).unwrap();
                if m.default.is_empty() {
                    let panic_s = format!(
                        "panic!(
                \"type mismatch in {ident}() metod:
                    expected- {},
                    found- {out_enum}{{:?}}\", 
                {varname})",
                        lside
                            .replace("(x)", &format!("({out})"))
                            .replace(" | ", "\n                            | ")
                    );
                    enum_doc.push_str(&panic_s.replace('"', "\\\""));
                    match_ts.extend(TokenStream::from_str(&panic_s).unwrap());
                } else {
                    enum_doc.push_str(
                        &(m.default.to_string())
                            .replace('"', "\\\"")
                            .replace(" {", " {\n            ")
                            .replace(", _ =>", ",\n            _ =>")
                            .replace(" !", "!")
                            .replace(" (", "(")
                            .replace("& ", "&")
                            .replace(" :", ":")
                            .replace(" ,", ",")
                            .replace(" <", "<")
                            .replace(" >", ">"),
                    );
                    match_ts.extend(m.default);
                }
                enum_doc.push_str("\n    }");
                body_ts.extend([Group(SGroup::new(Delimiter::Brace, match_ts))]);
            }
            enum_doc.push_str("\n}");
            metods_ts.extend([Group(SGroup::new(Delimiter::Brace, body_ts))]);
        }
    }
    metods_ts.extend(block_it);
    item_ts.extend([Group(SGroup::new(Delimiter::Brace, metods_ts))]);

    let mut result_ts = TokenStream::from_str(&format!(
        "{head_enum}{}{refs}{{{enum_doc}\n```\"] enum ",
        attr.enum_ident.as_ref().unwrap()
    ))
    .unwrap();
    result_ts.extend([Ident(attr.enum_ident.unwrap())]);
    result_ts.extend(TokenStream::from_str(refs).unwrap());
    result_ts.extend([Group(proc_macro::Group::new(Delimiter::Brace, enum_ts))]);

    result_ts.extend(item_ts);

    if let Some(out_ident) = &attr.out_ident {
        enum_ts = TokenStream::from_str("Unit, ").unwrap();
        enum_doc = "\n    Unit,".to_string();
        refs = "";
        for (name, out, span) in outs.iter() {
            enum_ts.extend([Ident(SIdent::new(name, *span))]);
            let typs = if out.contains('&') {
                refs = "<'a>";
                out.replace('&', "&'a ").replace("'a  ", "'a ")
            } else {
                out.clone()
            };
            enum_ts.extend(TokenStream::from_str(&format!("({typs}), ")));
            enum_doc.push_str(&format!("\n    {name}({typs}), "));
        }
        result_ts.extend(
            TokenStream::from_str(&format!(
                "{head_enum}{out_ident}{refs}{{{enum_doc}\n}}\n```\"] enum "
            ))
            .unwrap(),
        );
        result_ts.extend([Ident(out_ident.clone())]);
        result_ts.extend(TokenStream::from_str(refs).unwrap());
        result_ts.extend([Group(proc_macro::Group::new(Delimiter::Brace, enum_ts))]);
    }

    if std::env::var("M_ENUM_DBG").map_or(false, |v| &v != "0") {
        println!(
            "\nM_ENUM_DBG - output to compiler input for enum {}:\n{}\n",
            attr.enum_name, result_ts
        );
    }

    result_ts
}
