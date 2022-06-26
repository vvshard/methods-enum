#![doc = include_str!("../README.md")]
//! [crate documentation](crate)

use core::str::FromStr;
use proc_macro::TokenTree::{Group, Ident, Punct};
use proc_macro::{token_stream::IntoIter, Delimiter, Delimiter::Brace, Spacing, Span, TokenStream};

#[derive(Default)]
struct Attr {
    enum_name: String,
    enum_ident: Option<proc_macro::Ident>,
    run_method: String,
    drv_dbg: bool,
    out_ident: Option<proc_macro::Ident>,
    out_dbg: bool,
    strict_types: bool,
}
impl Attr {
    fn new(attr_ts: TokenStream) -> Attr {
        let mut attr_it = attr_ts.into_iter();
        let attr = match [attr_it.next(), attr_it.next(), attr_it.next()] {
            [Some(Ident(id)), Some(Punct(p)), Some(Ident(r_id))] if ",:".contains(p.as_char()) => {
                Attr {
                    enum_name: id.to_string(),
                    enum_ident: Some(id),
                    run_method: r_id.to_string(),
                    drv_dbg: p.as_char() == ':',
                    ..Default::default()
                }
            }
            _ => panic!("Syntax error in attribute #[methods_enum::gen(?? "),
        };
        match [attr_it.next(), attr_it.next()] {
            [None, None] => attr,
            [Some(Punct(p)), Some(Ident(out_id))] if ",=".contains(p.as_char()) => Attr {
                out_ident: Some(out_id),
                out_dbg: p.as_char() == '=',
                strict_types: matches!(attr_it.next(), Some(Punct(p)) if p.as_char() == '!'),
                ..attr
            },
            _ => panic!(
                "Syntax error in attribute #[methods_enum::gen({}:{}??..",
                attr.enum_name, attr.run_method
            ),
        }
    }
}

enum ParseStates {
    Start,
    Pub,
    Name,
    Args,
    Minus,
    Gt,
    Out,
}
use ParseStates::{Args, Gt, Minus, Name, Out, Pub, Start};

#[derive(Default)]
struct Meth {
    ident: Option<proc_macro::Ident>,
    prev_ts: TokenStream,
    pub_s: &'static str,
    args: TokenStream,
    params: String,
    typs: String,
    out_span: Option<Span>,
    out: TokenStream,
    default: TokenStream,
}
impl Meth {
    /// on successful parsing of the arguments returns `Minus`, otherwise - `Start`
    fn args_parsing(&mut self, args_gr: proc_macro::Group) -> ParseStates {
        let mut args_it = args_gr.stream().into_iter();
        let mut lg = 0;
        let mut first = true;
        let mut is_self = false;
        self.params = String::new();
        self.typs = String::new();
        let st = loop {
            match args_it.next() {
                Some(Punct(p)) if p.as_char() == ',' && lg == 0 => {
                    match [args_it.next(), args_it.next()] {
                        [Some(Ident(id)), Some(Punct(p))] if p.as_char() == ':' => {
                            if first {
                                if !is_self {
                                    break Start;
                                }
                                first = false;
                            } else {
                                self.params.push_str(", ");
                                self.typs.push_str(", ");
                            }
                            self.params.push_str(&id.to_string());
                        }
                        [Some(_), _] => break Start,
                        [None, _] => break if is_self { Minus } else { Start },
                    }
                }
                Some(Punct(p)) if "<>".contains(p.as_char()) => {
                    lg = lg + if p.as_char() == '<' { 1 } else { -1 };
                    self.typs.push(p.as_char());
                }
                Some(Ident(id)) if id.to_string() == "impl" => break Start,
                Some(Ident(id)) if first && id.to_string() == "self" => is_self = true,
                Some(Ident(id)) if !first && id.to_string() == "mut" => self.typs.push_str("mut "),
                Some(tt) if !first => self.typs.push_str(&tt.to_string()),
                None => break if is_self { Minus } else { Start },
                _ => (),
            }
        };
        if let Minus = st {
            self.args = args_gr.stream();
            self.out_span = None;
            self.out = TokenStream::new();
        }
        self.prev_ts.extend([Group(args_gr)]);
        st
    }

    fn prev_extend(&mut self, tt: proc_macro::TokenTree, new_st: ParseStates) -> ParseStates {
        self.prev_ts.extend([tt]);
        new_st
    }

    fn filling_vec(iit: &mut IntoIter, attr: &Attr) -> Vec<Meth> {
        let mut methods: Vec<Meth> = Vec::new();
        let mut m = Meth::default();
        let mut state = Start;
        for tt in iit {
            state = match (state, tt) {
                (Start, Ident(id)) if id.to_string() == "pub" => m.prev_extend(Ident(id), Pub),
                (st @ (Start | Pub), Ident(id)) if id.to_string() == "fn" => {
                    m.pub_s = if let Pub = st { "pub " } else { "" };
                    m.prev_extend(Ident(id), Name)
                }
                (Name, Ident(id)) => {
                    m.prev_ts.extend([Ident(id.clone())]);
                    if id.to_string() == attr.run_method {
                        break;
                    }
                    m.ident = Some(id);
                    Args
                }
                (Args, Group(gr)) if gr.delimiter() == Delimiter::Parenthesis => m.args_parsing(gr),
                (Minus, Punct(p)) if p.as_char() == '-' => m.prev_extend(Punct(p), Gt),
                (Gt, Punct(p)) if p.as_char() == '>' => {
                    m.out_span = Some(p.span());
                    m.prev_extend(Punct(p), Out)
                }
                (Out, Group(gr)) if gr.delimiter() == Brace && attr.out_ident.is_none() => {
                    m.prev_extend(Group(gr), Start) // skip fn with body
                }
                (Minus, Group(gr)) if gr.delimiter() == Brace => m.prev_extend(Group(gr), Start),
                (Out, Ident(id)) if id.to_string() == "where" => m.prev_extend(Ident(id), Start),
                (Minus | Out, Punct(p)) if p.as_char() == ';' => {
                    methods.push(m);
                    m = Meth::default();
                    Start
                }
                (Out, Group(gr)) if gr.delimiter() == Brace => {
                    m.default = gr.stream();
                    methods.push(m);
                    m = Meth::default();
                    Start
                }
                (Out, tt) => {
                    m.out.extend(TokenStream::from(tt.clone()));
                    m.prev_extend(tt, Out)
                }
                (_, tt) => m.prev_extend(tt, Start),
            }
        }
        m.ident = None;
        methods.push(m);
        methods
    }
}

fn ts_to_doc(ts: &TokenStream) -> String {
    let s = ts.to_string().replace("& ", "&").replace(":: ", "::");
    let inds: Vec<_> = s.match_indices(&['!', '(', ',', ':', '<', '>']).map(|t| t.0).collect();
    ([0].iter().chain(inds.iter()))
        .zip(inds.iter().chain(&[s.len()]))
        .map(|(&a, &b)| s[a..b].trim_end())
        .collect()
}

/// Based on the method signatures of the `impl` block, it generates: `enum` with parameters
/// from argument tuples and generates `{}` bodies of these methods with calling the argument
/// handler method from this `enum`.
///
/// This allows the handler method to control the behavior of the methods depending on the context.
///
/// #### Macro call syntax
/// **`#[methods_enum::gen(`*EnumName* `, ` | `: ` *handler_name* ( `, ` | ` = ` *OutName* `!`<sup>?</sup> )<sup>?</sup> `)]`**
///
/// where:
/// - ***EnumName***: The name of the automatically generated enum.
/// - ***handler_name***: Handler method name
/// - ***OutName*** (in case of more than one return type and/or to specify a default return values)
/// : The name of an automatically generated enum with variants from the return types.
///
/// Replacing the delimiter **`, `** after *EnumName* with **`: `** or before *OutName* with **` = `**
/// will automatically add the `#[derive(Debug)]` attribute to the corresponding enum.
///
/// Setting `!` after *OutName* enables checking the returned variant by its name, not by its type.
///
/// See the [crate documentation](crate) for details.
#[proc_macro_attribute]
pub fn gen(attr_ts: TokenStream, item_ts: TokenStream) -> TokenStream {
    // std::fs::write("target/debug/item_ts.txt", format!("{}\n\n{0:#?}", item_ts)).unwrap();

    let attr = Attr::new(attr_ts);

    let mut item_it = item_ts.into_iter();

    let mut item_ts = TokenStream::from_iter(
        item_it.by_ref().take_while(|tt| !matches!(tt, Ident(id) if id.to_string() == "impl")),
    );
    item_ts.extend([Ident(proc_macro::Ident::new("impl", Span::call_site()))]);

    let mut block_it = match [item_it.next(), item_it.next(), item_it.next()] {
        [Some(Ident(item_n)), Some(Group(gr)), None] if gr.delimiter() == Brace => {
            item_ts.extend([Ident(item_n)]);
            gr.stream().into_iter()
        }
        m => panic!(
            "SYNTAX ERROR: 
        'this attribute must be set on block impl without treyds and generics': {m:?}"
        ),
    };

    let methods = Meth::filling_vec(&mut block_it, &attr);

    let head = r##"
        #[derive(Debug)]
        #[allow(non_camel_case_types)]
        /// Formed by macro [`#[methods_enum::gen(...)]`](https://docs.rs/methods-enum):
        /// ```
        /// #[derive(Debug)]
        /// #[allow(non_camel_case_types)]
        #[doc = "enum "##;
    let head_w_o_dbg = head.lines().filter(|s| !s.ends_with("g)]")).collect::<Vec<_>>().join("\n");
    //                 (name.0, out.1, span.2)
    let mut outs: Vec<(String, String, Span)> = Vec::new();
    let mut enum_doc = String::new();
    let mut enum_ts = TokenStream::new();
    for m in methods.iter() {
        if let Some(ident) = &m.ident {
            enum_ts.extend([Ident(ident.clone())]);
            let typs = m.typs.replace('&', "&'a ");
            enum_ts.extend(TokenStream::from_str(&format!("({typs}), ")));
            enum_doc.push_str(&format!("\n    {ident}({typs}), "));
            if let Some(out_span) = m.out_span {
                outs.push((ident.to_string(), ts_to_doc(&m.out), out_span));
            }
        }
    }
    let lftm = if enum_doc.contains('&') { "<'a>" } else { "" };
    enum_doc.push_str("\n}\n```\n---\nMethod bodies generated by the same macro:\n```");

    let is_result = attr.out_ident.is_none() && outs.iter().any(|t| t.1.contains("Result<"));
    let self_run_enum = format!("self.{}({}::", attr.run_method, attr.enum_name);
    let mut methods_ts = TokenStream::new();
    for m in methods {
        methods_ts.extend(m.prev_ts);
        if let Some(ident) = m.ident {
            enum_doc.push_str(&format!("\n{}fn {ident}({})", m.pub_s, ts_to_doc(&m.args)));
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
                    body_ts.extend([Punct(proc_macro::Punct::new(';', Spacing::Alone))]);
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
                        "panic!(\"Type mismatch in the {ident}() method:
                    expected- {},
                    found- {out_enum}{{}}\", {varname}.stype())",
                        lside
                            .replace("(x)", &format!("({out})"))
                            .replace(" | ", "\n                            | ")
                    );
                    enum_doc.push_str(&panic_s);
                    match_ts.extend(TokenStream::from_str(&panic_s).unwrap());
                } else {
                    enum_doc.push_str(
                        &ts_to_doc(&m.default)
                            .replace(" {", " {\n            ")
                            .replace(", _ =>", ",\n            _ =>"),
                    );
                    match_ts.extend(m.default);
                }
                enum_doc.push_str("\n    }");
                body_ts.extend([Group(proc_macro::Group::new(Brace, match_ts))]);
            }
            enum_doc.push_str("\n}");
            methods_ts.extend([Group(proc_macro::Group::new(Brace, body_ts))]);
        }
    }
    methods_ts.extend(block_it);
    item_ts.extend([Group(proc_macro::Group::new(Brace, methods_ts))]);

    let mut res_ts = TokenStream::from_str(&format!(
        "{}{}{lftm}{{{}\n```\"] enum ",
        if attr.drv_dbg { head } else { &head_w_o_dbg },
        attr.enum_name,
        enum_doc.escape_debug().to_string()
    ))
    .unwrap();
    res_ts.extend([Ident(attr.enum_ident.unwrap())]);
    res_ts.extend(TokenStream::from_str(lftm).unwrap());
    res_ts.extend([Group(proc_macro::Group::new(Brace, enum_ts))]);

    res_ts.extend(item_ts);

    if let Some(out_ident) = &attr.out_ident {
        enum_doc = "\n    Unit,".to_string();
        enum_ts = TokenStream::from_str("Unit, ").unwrap();
        let indent = "\n            ";
        let mut stype = format!(
            "    fn stype(&self) -> &'static str {{
        match self {{{indent}{out_ident}::Unit => \"Unit\","
        );
        let mut lftm = "";
        for (name, mut out, span) in outs {
            enum_ts.extend([Ident(proc_macro::Ident::new(&name, span))]);
            stype.push_str(&format!("{indent}{out_ident}::{name}(..) => \"{name}({out})\","));
            if out.contains('&') {
                lftm = "<'a>";
                out = out.replace('&', "&'a ");
            }
            enum_ts.extend(TokenStream::from_str(&format!("({out}), ")));
            enum_doc.push_str(&format!("\n    {name}({out}), "));
        }
        stype = format!("impl{lftm} {out_ident}{lftm} {{\n{stype}\n        }}\n    }}\n}}");
        enum_doc = (enum_doc + "\n}\n\n" + &stype).escape_debug().to_string();

        res_ts.extend(
            TokenStream::from_str(&format!(
                "{}{out_ident}{lftm}{{{enum_doc}\n```\"] enum ",
                if attr.out_dbg { head } else { &head_w_o_dbg }
            ))
            .unwrap(),
        );
        res_ts.extend([Ident(out_ident.clone())]);
        res_ts.extend(TokenStream::from_str(lftm).unwrap());
        res_ts.extend([Group(proc_macro::Group::new(Brace, enum_ts))]);
        res_ts.extend(TokenStream::from_str(&stype).unwrap());
    }

    if std::env::var("M_ENUM_DBG").map_or(false, |v| &v != "0") {
        println!(
            "\nM_ENUM_DBG - output to compiler input for enum {}:\n{}\n",
            attr.enum_name, res_ts
        );
    }

    res_ts
}
