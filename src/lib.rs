#![doc = include_str!("../README.md")]
//! [crate documentation](crate)

use core::str::FromStr;
use proc_macro::Group as Gr;
use proc_macro::TokenTree::{Group, Ident, Punct};
use proc_macro::{token_stream::IntoIter, Delimiter, Delimiter::Brace, Spacing, Span, TokenStream};

enum ParseStates {
    Start,
    Vis,
    Name,
    Args,
    Minus,
    Gt,
    Out,
}
use ParseStates::{Args, Gt, Minus, Name, Out, Start, Vis};

// region: region gen

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

#[derive(Default)]
struct Meth {
    ident: Option<proc_macro::Ident>,
    prev_ts: TokenStream,
    vis: TokenStream,
    args: TokenStream,
    params: String,
    typs: String,
    out_span: Option<Span>,
    out: TokenStream,
    body: TokenStream,
}

impl Meth {
    /// on successful parsing of the arguments returns `Minus`, otherwise - `Start`
    fn args_parsing(&mut self, args_gr: Gr) -> ParseStates {
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

    fn vec(iit: &mut IntoIter, attr: &Attr) -> Vec<Meth> {
        let mut methods: Vec<Meth> = Vec::new();
        let mut m = Meth::default();
        let mut state = Start;
        for tt in iit {
            state = match (state, tt) {
                (Start, Ident(id)) if id.to_string() == "pub" => {
                    m.vis.extend([Ident(id.clone())]);
                    m.prev_extend(Ident(id), Vis)
                }
                (Vis, Group(gr)) if gr.delimiter() == Delimiter::Parenthesis => {
                    m.vis.extend([Group(gr.clone())]);
                    m.prev_extend(Group(gr), Vis)
                }
                (st @ (Start | Vis), Ident(id)) if id.to_string() == "fn" => {
                    if let Start = st {
                        m.vis = TokenStream::new()
                    };
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
                    m.body = gr.stream();
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
    // std::fs::write("target/debug/item_ts.log", format!("{}\n\n{0:#?}", item_ts)).unwrap();

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

    let methods = Meth::vec(&mut block_it, &attr);

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
    let mut enum_doc = " {".to_string();
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
            enum_doc.push_str(&format!(
                "\n{}fn {ident}({})",
                (ts_to_doc(&m.vis) + " ").trim_start(),
                ts_to_doc(&m.args)
            ));
            let mut body_ts = TokenStream::new();
            let out = if m.out.is_empty() {
                enum_doc.push_str(" {");
                if is_result {
                    enum_doc.push_str("\n    #![allow(unused_must_use)]");
                    body_ts.extend(TokenStream::from_str("#![allow(unused_must_use)]").unwrap());
                }
                String::new()
            } else {
                let name = ident.to_string();
                let find_out = outs.iter().find(|t| t.0 == name).unwrap().1.clone();
                enum_doc.push_str(&format!(" -> {find_out} {{"));
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
                if m.body.is_empty() {
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
                        &ts_to_doc(&m.body)
                            .replace(" {", " {\n            ")
                            .replace(", _ =>", ",\n            _ =>"),
                    );
                    match_ts.extend(m.body);
                }
                enum_doc.push_str("\n    }");
                body_ts.extend([Group(Gr::new(Brace, match_ts))]);
            }
            enum_doc.push_str("\n}");
            methods_ts.extend([Group(Gr::new(Brace, body_ts))]);
        }
    }
    methods_ts.extend(block_it);
    item_ts.extend([Group(Gr::new(Brace, methods_ts))]);

    let mut res_ts = TokenStream::from_str(&format!(
        "{}{}{lftm}{}\"] enum ",
        if attr.drv_dbg { head } else { &head_w_o_dbg },
        attr.enum_name,
        (enum_doc + "\n```").escape_debug().to_string()
    ))
    .unwrap();
    res_ts.extend([Ident(attr.enum_ident.unwrap())]);
    res_ts.extend(TokenStream::from_str(lftm).unwrap());
    res_ts.extend([Group(Gr::new(Brace, enum_ts))]);

    res_ts.extend(item_ts);

    if let Some(out_ident) = &attr.out_ident {
        enum_doc = " {\n    Unit,".to_string();
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
        enum_doc = (enum_doc + "\n}\n\n" + &stype + "\n```").escape_debug().to_string();

        res_ts.extend(
            TokenStream::from_str(&format!(
                "{}{out_ident}{lftm}{enum_doc}\"] enum ",
                if attr.out_dbg { head } else { &head_w_o_dbg }
            ))
            .unwrap(),
        );
        res_ts.extend([Ident(out_ident.clone())]);
        res_ts.extend(TokenStream::from_str(lftm).unwrap());
        res_ts.extend([Group(Gr::new(Brace, enum_ts))]);
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

// endregion: gen

//     #####     #####     #####     #####     #####     #####     #####     #####

// region: region impl_match

use std::collections::{HashMap, HashSet};

#[derive(Default)]
struct Item {
    ident: String,
    prev_ts: TokenStream,
    group: TokenStream,
}
impl Item {
    fn prev_extend(&mut self, tt: proc_macro::TokenTree, new_st: ParseStates) -> ParseStates {
        self.prev_ts.extend([tt]);
        new_st
    }

    fn vec(iit: &mut IntoIter) -> (Vec<Item>, HashMap<&'static str, usize>) {
        let mut items = Vec::new();
        let mut i: HashMap<&'static str, usize> = HashMap::from([("impl", 9999), ("enum", 9999)]);
        let mut kind = "";
        let mut item = Item::default();
        let mut state = Start;
        for tt in iit {
            state = match (state, tt) {
                (Start, Ident(id)) => match (&id.to_string()[..], i["impl"], i["enum"]) {
                    ("impl", 9999, _) => {
                        kind = "impl";
                        item.prev_extend(Ident(id), Name)
                    }
                    ("enum", _, 9999) => {
                        kind = "enum";
                        item.prev_extend(Ident(id), Name)
                    }
                    _ => item.prev_extend(Ident(id), Start),
                },
                (Name | Out, Ident(id)) => {
                    item.ident = id.to_string();
                    item.prev_extend(Ident(id), Out)
                }
                (Out, Group(gr)) if gr.delimiter() == Brace => {
                    item.group = gr.stream();
                    i.insert(kind, items.len());
                    items.push(item);
                    if items.len() == 2 {
                        return (items, i);
                    }
                    item = Item::default();
                    Start
                }
                (_, Punct(p)) if ";#".contains(p.as_char()) => item.prev_extend(Punct(p), Start),
                (st, tt) => item.prev_extend(tt, st),
            }
        }
        item.ident = String::new();
        items.push(item);
        for kind in ["impl", "enum"] {
            if i[kind] == 9999 {
                i.insert(kind, items.len());
                items.push(Item::default());
            }
        }
        (items, i)
    }
}

#[derive(Default)]
struct MethIM {
    ident: String,
    prev_ts: TokenStream,
    body: TokenStream,
    tail: TokenStream,
}
impl MethIM {
    fn prev_extend(&mut self, tt: proc_macro::TokenTree, new_st: ParseStates) -> ParseStates {
        self.prev_ts.extend([tt]);
        new_st
    }

    fn found_match(&mut self, body: &Gr) -> bool {
        self.body = TokenStream::new();
        let mut iit = body.stream().into_iter();
        let mut found = false;
        while let Some(tt) = iit.next() {
            match (found, tt) {
                (false, Ident(id)) if id.to_string() == "match" => {
                    self.body.extend([Ident(id)]);
                    found = true;
                }
                (true, Group(gr)) if gr.delimiter() == Brace => {
                    if gr.stream().is_empty() {
                        self.tail.extend(iit);
                        return true;
                    } else {
                        self.body.extend([Group(gr)]);
                        found = false;
                    }
                }
                (_, tt) => self.body.extend([tt]),
            }
        }
        found
    }

    fn vec(ts: TokenStream) -> (Vec<MethIM>, HashSet<String>) {
        let mut methods = Vec::new();
        let mut mset: HashSet<String> = HashSet::new();
        let mut m = MethIM::default();
        let mut state = Start;
        for tt in ts {
            state = match (state, tt) {
                (Start, Ident(id)) if id.to_string() == "fn" => m.prev_extend(Ident(id), Name),
                (Name, Ident(id)) => {
                    m.ident = id.to_string();
                    m.prev_extend(Ident(id), Out)
                }
                (Out, Group(gr)) if gr.delimiter() == Brace => {
                    if m.found_match(&gr) {
                        mset.insert(m.ident.clone());
                        methods.push(m);
                        m = MethIM::default();
                    } else {
                        m.prev_ts.extend([Group(gr)])
                    }
                    Start
                }
                (Out, Punct(p)) if ";#".contains(p.as_char()) => m.prev_extend(Punct(p), Start),
                (Out, tt) => m.prev_extend(tt, Out),
                (_, tt) => m.prev_extend(tt, Start),
            }
        }
        m.ident = String::new();
        methods.push(m);
        (methods, mset)
    }
}

#[derive(Default)]
struct Var {
    name: String,
    fields: Option<Gr>,
    methods: HashMap<String, Gr>,
}
impl Var {
    fn vec(item: &mut Item, mset: &HashSet<String>, enm_n: &String, imp_n: &String) -> Vec<Var> {
        let mut iit = std::mem::take(&mut item.group).into_iter();
        let mut enm: Vec<Var> = Vec::new();
        let mut var = Var::default();
        while let Some(tt) = iit.next() {
            match tt {
                Punct(p) if p.as_char() == '#' && var.name == "" => match iit.next() {
                    Some(Group(gr)) if gr.delimiter() == Delimiter::Bracket => {
                        item.group.extend([Punct(p), Group(gr)]);
                    }
                    Some(Punct(p1)) if p.as_char() == '!' => match iit.next() {
                        Some(Group(gr)) if gr.delimiter() == Delimiter::Bracket => {
                            item.group.extend([Punct(p), Punct(p1), Group(gr)]);
                        }
                        _ => (),
                    },
                    _ => (),
                },
                Ident(id) => {
                    let name = id.to_string();
                    if var.name.is_empty() {
                        item.group.extend([Ident(id)]);
                        var.name = name;
                    } else {
                        // method name
                        if !mset.contains(&name) {
                            if imp_n.is_empty() {
                                panic!(
                                    "impl_match!: invalid method `{name}` in `enum {enm_n}::{}`:
there is no `impl` block in the macro scope",
                                    var.name
                                )
                            }
                            let mut free_m: Vec<String> = mset
                                .difference(&HashSet::from_iter(var.methods.keys().cloned()))
                                .cloned()
                                .collect();
                            free_m.sort();
                            if free_m.is_empty() {
                                panic!(
                                    "impl_match!: invalid method `{name}` in `enum {enm_n}::{}`:
`impl {imp_n}` contains no freely methods to implement `match{{...}}` from `enum {enm_n}::{0}`",
                                    var.name
                                )
                            } else {
                                panic!(
                                    "impl_match!: invalid method name `{name}` in `enum {enm_n}::{}`
- expected{}: `{}`",
                                    var.name,
                                    if free_m.len() == 1 { "" } else { " one of" },
                                    free_m.join("`|`")
                                )
                            }
                        };
                        match (iit.next(), iit.next()) {
                            (Some(Group(g_arg)), Some(Group(g_block)))
                                if g_arg.delimiter() == Delimiter::Parenthesis
                                    && g_block.delimiter() == Brace =>
                            {
                                if var.methods.insert(name.clone(), g_block).is_some() {
                                    panic!(
                                        "impl_match!: {} `{name}` in `enum {enm_n}::{}`",
                                        "repetition of method name", var.name
                                    )
                                }
                            }
                            _ => panic!(
                                "impl_match!: invalid syntax in method `{}` in `enum {enm_n}::{}`
- expected: `(...) {{...}}`",
                                name, var.name
                            ),
                        }
                    }
                }
                Group(gr) if gr.delimiter() != Delimiter::Bracket && var.methods.is_empty() => {
                    if var.fields.is_none() {
                        item.group.extend([Group(gr)]);
                        var.fields = Some(Gr::new(Brace, TokenStream::new()));
                    } else {
                        var.fields = Some(gr);
                    }
                }
                Punct(p) if p.as_char() == ',' => {
                    if !var.name.is_empty() {
                        item.group.extend([Punct(p)]);
                        enm.push(var);
                        var = Var::default();
                    }
                }
                _ => (),
            }
        }
        if !var.name.is_empty() {
            enm.push(var)
        }
        enm
    }
}

#[proc_macro]
pub fn impl_match(input_ts: TokenStream) -> TokenStream {
    // std::fs::write("target/debug/input_ts.log", format!("{}\n\n{0:#?}", input_ts)).unwrap();

    let mut input_it = input_ts.into_iter();

    let (mut items, i) = Item::vec(&mut input_it);
    let (methods, mset) = MethIM::vec(std::mem::take(&mut items[i["impl"]].group));
    let enm_n = items[i["enum"]].ident.clone();
    let impl_n = items[i["impl"]].ident.clone();
    let mut enm = Var::vec(&mut items[i["enum"]], &mset, &enm_n, &impl_n);
    let mut impl_group = TokenStream::new();
    for mut m in methods {
        impl_group.extend(m.prev_ts);
        if !m.ident.is_empty() {
            let mut block_ts = TokenStream::new();
            for var in enm.iter_mut() {
                block_ts.extend(TokenStream::from_str(&format!("{enm_n}::{}", var.name)));
                if var.fields.is_some() {
                    block_ts.extend([Group(var.fields.clone().unwrap())]);
                }
                block_ts.extend(TokenStream::from_str("=>"));
                block_ts.extend([Group(match var.methods.get_mut(&m.ident) {
                    Some(gr) => std::mem::replace(gr, Gr::new(Brace, TokenStream::new())),
                    None => Gr::new(Brace, TokenStream::new()),
                })]);
            }
            m.body.extend([Group(Gr::new(Brace, block_ts))]);
            m.body.extend(m.tail);
            impl_group.extend([Group(Gr::new(Brace, m.body))]);
        }
    }
    items[i["impl"]].group = impl_group;

    let mut res_ts = TokenStream::new();
    for item in items {
        res_ts.extend(item.prev_ts);
        if !item.ident.is_empty() {
            res_ts.extend([Group(Gr::new(Brace, item.group))]);
        }
    }
    res_ts.extend(input_it);

    res_ts
}

// endregion: impl_match
