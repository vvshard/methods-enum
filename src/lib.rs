#![doc = include_str!("../README.md")]
//!
//! Two macros for easy implementation of 'state' design pattern and other dynamic polymorphism using enum instead of dyn Trait   
//!
//! [crate documentation](crate)

use core::str::FromStr;
use proc_macro::TokenTree::{Group, Ident, Punct};
use proc_macro::{token_stream::IntoIter, Delimiter, Delimiter::Brace, Spacing, Span, TokenStream};
use proc_macro::{Group as Gr, Ident as Idn, Punct as Pn};
use std::iter::once;

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
    enum_ident: Option<Idn>,
    run_method: String,
    drv_dbg: bool,
    out_ident: Option<Idn>,
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
            _ => panic!("#[gen]: Syntax error in attribute #[methods_enum::gen(?? "),
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
                "#[gen]: Syntax error in attribute #[methods_enum::gen({}:{}??..",
                attr.enum_name, attr.run_method
            ),
        }
    }
}

#[derive(Default)]
struct Meth {
    ident: Option<Idn>,
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
        self.prev_ts.extend(once(Group(args_gr)));
        st
    }

    fn prev_extend(&mut self, tt: proc_macro::TokenTree, new_st: ParseStates) -> ParseStates {
        self.prev_ts.extend(once(tt));
        new_st
    }

    fn vec(iit: &mut IntoIter, attr: &Attr) -> Vec<Meth> {
        let mut methods: Vec<Meth> = Vec::new();
        let mut m = Meth::default();
        let mut state = Start;
        for tt in iit {
            state = match (state, tt) {
                (Start, Ident(id)) if id.to_string() == "pub" => {
                    m.vis.extend(once(Ident(id.clone())));
                    m.prev_extend(Ident(id), Vis)
                }
                (Vis, Group(gr)) if gr.delimiter() == Delimiter::Parenthesis => {
                    m.vis.extend(once(Group(gr.clone())));
                    m.prev_extend(Group(gr), Vis)
                }
                (st @ (Start | Vis), Ident(id)) if id.to_string() == "fn" => {
                    if let Start = st {
                        m.vis = TokenStream::new()
                    };
                    m.prev_extend(Ident(id), Name)
                }
                (Name, Ident(id)) => {
                    m.prev_ts.extend(once(Ident(id.clone())));
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
/// The macro attribute is set before an individual (non-Trait) impl block. Based on the method signatures of the impl block, it generates: `enum` with parameters from argument tuples and generates `{}` bodies of these methods with calling the argument handler method from this `enum`.  
/// This allows the handler method to control the behavior of methods depending on the context, including structuring enum-matching by state.
///
/// ## Usage example
///
/// [Chapter 17.3 "Implementing an Object-Oriented Design Pattern" of the rust-book](https://doc.rust-lang.org/book/ch17-03-oo-design-patterns.html) shows the implementation of the *state pattern* in Rust, which provides the following behavior:
/// ```rust ignore
/// pub fn main() {
///     let mut post = blog::Post::new();
///
///     post.add_text("I ate a salad for lunch today");
///     assert_eq!("", post.content());
///     post.request_review(); // without request_review() - approve() should not work
///     post.approve();  
///     assert_eq!("I ate a salad for lunch today", post.content());
/// }
/// ```
/// with macro **`#[gen()]`** this is solved like this:
/// ```rust
/// mod blog {
///     enum State {
///         Draft,
///         PendingReview,
///         Published,
///     }
///
///     pub struct Post {
///         state: State,
///         content: String,
///     }
///
///     #[methods_enum::gen(Meth, run_methods)]
///     impl Post {
///         pub fn add_text(&mut self, text: &str);
///         pub fn request_review(&mut self);
///         pub fn approve(&mut self);
///         pub fn content(&mut self) -> &str;
///
///         #[rustfmt::skip]
///         fn run_methods(&mut self, method: Meth) -> &str {
///             match self.state {
///                 State::Draft => match method {
///                     Meth::add_text(text) => { self.content.push_str(text); "" }
///                     Meth::request_review() => { self.state = State::PendingReview; "" }
///                     _ => "",
///                 },
///                 State::PendingReview => match method {
///                     Meth::approve() => { self.state = State::Published; "" }
///                     _ => "",
///                 },
///                 State::Published => match method {
///                     Meth::content() => &self.content,
///                     _ => "",
///                 },
///             }
///         }
///
///         pub fn new() -> Post {
///             Post { state: State::Draft, content: String::new() }
///         }
///     }
/// }
/// ```
/// In the handler method (in this case, `run_methods`), simply write for each state which methods should work and how.
///
/// The macro duplicates the output for the compiler in the doc-comments.
/// Therefore, in the IDE[^rust_analyzer], you can always see the declaration of the generated `enum` and the generated method bodies.
/// 
/// [^rust_analyzer]: *rust-analyzer may not expand proc-macro when running under nightly or old rust edition.* In this case it is recommended to set in its settings: [`"rust-analyzer.server.extraEnv": { "RUSTUP_TOOLCHAIN": "stable" }`](https://rust-analyzer.github.io/manual.html#toolchain)
///
/// ## Restrictions
/// 
/// - The **`#[gen(...)]`** macro does not work on generic methods (including lifetime generics). As a general rule, methods with <...> before the argument list, with `where` before the body, or `impl` in the argument type declaration will be silently ignored for inclusion in `enum`.
/// - The macro will ignore signatures with destructured arguments.
/// - The macro ignores also methods with a `mut` prefix in front of a method argument name (except  `self`): move such an argument to a mut variable in the body of the handler method.
/// - The `self` form of all methods of the same `enum` must be the same and match the `self` form of the handler method. As a rule, it is either `&mut self` everywhere or `self` in methods + `mut self` in the handler method. However, it is allowed to group method signatures into multiple `impl` blocks with different `enum` and handler methods. See example below.
/// 
/// ## [gen macro details and use cases](attr.gen.html#gen-macro-details-and-use-cases)
///
#[doc = include_str!("gen_details.md")]
#[proc_macro_attribute]
pub fn gen(attr_ts: TokenStream, item_ts: TokenStream) -> TokenStream {
    // std::fs::write("target/debug/item_ts.log", format!("{}\n\n{0:#?}", item_ts)).unwrap();

    let attr = Attr::new(attr_ts);

    let mut item_it = item_ts.into_iter();

    let mut item_ts = TokenStream::from_iter(
        item_it.by_ref().take_while(|tt| !matches!(tt, Ident(id) if id.to_string() == "impl")),
    );
    item_ts.extend(once(Ident(Idn::new("impl", Span::call_site()))));

    let mut block_it = match [item_it.next(), item_it.next(), item_it.next()] {
        [Some(Ident(item_n)), Some(Group(gr)), None] if gr.delimiter() == Brace => {
            item_ts.extend(once(Ident(item_n)));
            gr.stream().into_iter()
        }
        m => panic!(
            "#[gen]: SYNTAX ERROR 
'attribute #[gen] must be set on block impl without treyds and generics': {m:?}"
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
            enum_ts.extend(once(Ident(ident.clone())));
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
                    body_ts.extend(once(Punct(Pn::new(';', Spacing::Alone))));
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
                body_ts.extend(once(Group(Gr::new(Brace, match_ts))));
            }
            enum_doc.push_str("\n}");
            methods_ts.extend(once(Group(Gr::new(Brace, body_ts))));
        }
    }
    methods_ts.extend(block_it);
    item_ts.extend(once(Group(Gr::new(Brace, methods_ts))));

    let mut res_ts = TokenStream::from_str(&format!(
        "{}{}{lftm}{}\"] enum ",
        if attr.drv_dbg { head } else { &head_w_o_dbg },
        attr.enum_name,
        (enum_doc + "\n```").escape_debug().to_string()
    ))
    .unwrap();
    res_ts.extend(once(Ident(attr.enum_ident.unwrap())));
    res_ts.extend(TokenStream::from_str(lftm).unwrap());
    res_ts.extend(once(Group(Gr::new(Brace, enum_ts))));

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
            enum_ts.extend(once(Ident(Idn::new(&name, span))));
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

        res_ts.extend(TokenStream::from_str(&format!(
            "{}{out_ident}{lftm}{enum_doc}\"] enum ",
            if attr.out_dbg { head } else { &head_w_o_dbg }
        )));
        res_ts.extend(once(Ident(out_ident.clone())));
        res_ts.extend(TokenStream::from_str(lftm).unwrap());
        res_ts.extend(once(Group(Gr::new(Brace, enum_ts))));
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

use std::collections::{hash_map::DefaultHasher, HashMap, HashSet};
use std::hash::{Hash, Hasher};
use std::mem;

struct Flags {
    panic: bool,
    no_semnt: bool,
}

#[derive(Default)]
struct Item {
    name: String,
    ident: Option<Idn>, // for enum here - its id, for impl - the id of the trait
    it_enum: bool,
    no_def: bool,
    prev_ts: TokenStream,
    group: TokenStream,
    methods: Vec<MethIM>,
}
impl Item {
    fn prev_extend(&mut self, tt: proc_macro::TokenTree, new_state: ParseStates) -> ParseStates {
        if !self.no_def {
            self.prev_ts.extend(once(tt))
        }
        if let Vis = new_state {
            self.ident = None;
            self.name = String::new();
            Name
        } else {
            new_state
        }
    }

    fn vec(ts: TokenStream) -> (Vec<Item>, HashMap<String, bool>, Flags) {
        let mut items = Vec::new();
        let mut mmap: HashMap<String, bool> = HashMap::new(); // v: bool = there is a generic
        let mut impl_n = String::new();
        let mut item = Item::default();
        let mut lg = 0;
        let mut state = Args;
        let mut flags = Flags { no_semnt: true, panic: true };
        if cfg!(debug_assertions) {
            flags.no_semnt = false;
            flags.panic = false;
        }
        for tt in ts {
            state = match (state, tt, lg) {
                (Args, Group(gr), 0) if gr.delimiter() == Delimiter::Parenthesis => {
                    if cfg!(debug_assertions) {
                        for fl in gr.stream() {
                            match fl {
                                Punct(p) if p.as_char() == '!' => flags.panic = true,
                                Ident(id) => match &id.to_string().to_lowercase()[..] {
                                    "ns" | "sn" => {
                                        flags.no_semnt = true;
                                        flags.panic = true;
                                    }
                                    _ => (),
                                },
                                _ => (),
                            }
                        }
                    }
                    Start
                }
                (Start | Args, Punct(p), 0) if p.as_char() == '@' => {
                    item.it_enum = true;
                    item.no_def = true;
                    item.prev_extend(Punct(p), Vis)
                }
                (Start | Args, Ident(id), 0) => match &id.to_string()[..] {
                    "impl" => item.prev_extend(Ident(id), Vis),
                    "enum" => {
                        item.it_enum = true;
                        item.prev_extend(Ident(id), Vis)
                    }
                    _ => item.prev_extend(Ident(id), Start),
                },
                (Name, Ident(id), 0) if id.to_string() == "for" => item.prev_extend(Ident(id), Out),
                (st @ (Name | Out), Ident(id), 0) => {
                    match st {
                        Name => item.ident = Some(id.clone()),
                        _ => item.name = id.to_string(),
                    }
                    item.prev_extend(Ident(id), st)
                }
                (Name | Out, Group(gr), 0) if gr.delimiter() == Brace => {
                    if item.ident.is_some() {
                        if item.it_enum {
                            item.group = gr.stream();
                            item.name = item.ident.as_ref().unwrap().to_string();
                            items.push(mem::take(&mut item));
                        } else {
                            if item.name.is_empty() {
                                item.name = item.ident.as_ref().unwrap().to_string();
                                item.ident = None;
                            }
                            if impl_n.is_empty() || impl_n == item.name {
                                if impl_n.is_empty() {
                                    impl_n = item.name.clone();
                                }
                                item.fill_methods(gr.stream(), &mut mmap);
                                items.push(mem::take(&mut item));
                            } else {
                                item.prev_ts.extend(once(Group(gr)));
                            }
                        }
                    }
                    Start
                }
                (st, Punct(p), _) if "<>".contains(p.as_char()) => {
                    lg = 0.max(lg + if p.as_char() == '<' { 1 } else { -1 });
                    item.prev_extend(Punct(p), st)
                }
                (Args, tt, _) => item.prev_extend(tt, Start),
                (st, tt, _) => item.prev_extend(tt, st),
            }
        }
        item.name = String::new();
        items.push(item);
        (items, mmap, flags)
    }

    fn fill_methods(&mut self, ts: TokenStream, mmap: &mut HashMap<String, bool>) {
        let mut m = MethIM::default();
        let mut args: Option<TokenStream> = None;
        let mut state = Start;
        for tt in ts {
            state = match (state, tt) {
                (Start, Ident(id)) if id.to_string() == "fn" => m.prev_extend(Ident(id), Name),
                (Name, Ident(id)) => {
                    m.name = self.ident.as_ref().map_or(id.to_string(), |t| format!("{id}() {t}"));
                    args = None;
                    m.prev_extend(Ident(id), Args)
                }
                (Args, Punct(p)) if p.as_char() == '<' => {
                    args = Some(TokenStream::from_iter(once(Ident(Idn::new("impl", p.span())))));
                    m.prev_extend(Punct(p), Gt)
                }
                (Args, Group(gr)) if gr.delimiter() == Delimiter::Parenthesis => {
                    args = Some(gr.stream());
                    m.prev_extend(Group(gr), Gt)
                }
                (Gt, Group(gr)) if gr.delimiter() == Brace => m.prev_extend(Group(gr), Start),
                (Gt, Punct(p)) if p.as_char() == ';' => m.prev_extend(Punct(p), Start),
                (Gt, Punct(p)) if p.as_char() == '~' => Out,
                (Gt | Args, tt) => m.prev_extend(tt, Gt),
                (Out, Group(gr)) if gr.delimiter() == Brace => {
                    if m.found_match(&gr) {
                        mmap.insert(
                            m.name.clone(), // v: bool = there is a generic
                            args.take().map_or(false, |t| {
                                t.into_iter()
                                    .any(|tr| matches!(tr, Ident(id) if id.to_string() == "impl"))
                            }),
                        );
                        self.methods.push(mem::take(&mut m));
                    } else {
                        m.prev_ts.extend(once(Group(gr)))
                    }
                    Start
                }
                (_, tt) => m.prev_extend(tt, Start),
            }
        }
        m.name = String::new();
        self.methods.push(m);
    }
}

#[derive(Default)]
struct MethIM {
    name: String,
    prev_ts: TokenStream,
    body: TokenStream,
    dflt_arm: Option<Gr>,
    tail: TokenStream,
}
impl MethIM {
    fn prev_extend(&mut self, tt: proc_macro::TokenTree, new_st: ParseStates) -> ParseStates {
        self.prev_ts.extend(once(tt));
        new_st
    }

    fn found_match(&mut self, body: &Gr) -> bool {
        self.body = TokenStream::new();
        let mut iit = body.stream().into_iter();
        let mut found = false;
        while let Some(tt) = iit.next() {
            match (found, tt) {
                (false, Ident(id)) if id.to_string() == "match" => {
                    self.body.extend(once(Ident(id)));
                    found = true;
                }
                (true, Punct(p)) if p.as_char() == ';' => {
                    self.tail.extend(once(Punct(p)).chain(iit));
                    return true;
                }
                (true, Group(gr)) if gr.delimiter() == Brace => {
                    let mut isfat_arrow = false;
                    let mut gr_iit = gr.stream().into_iter();
                    while let Some(tt) = gr_iit.next() {
                        if let Punct(p) = tt {
                            if p.as_char() == '=' {
                                if let Some(Punct(gt)) = gr_iit.next() {
                                    if gt.as_char() == '>' {
                                        isfat_arrow = true;
                                        break;
                                    }
                                }
                            }
                        }
                    }
                    if isfat_arrow {
                        self.body.extend(once(Group(gr)));
                        found = false;
                    } else {
                        self.dflt_arm = Some(gr);
                        self.tail.extend(iit);
                        return true;
                    }
                }
                (_, tt) => self.body.extend(once(tt)),
            }
        }
        found
    }
}

struct VarMeth {
    ident: Idn,
    fields: Option<Gr>,
    block: Gr,
    opt_trait: Option<Idn>,
}

#[derive(Default)]
struct Var {
    ident: Option<Idn>,
    fields: Option<Gr>,
    methods: HashMap<String, VarMeth>,
}
impl Var {
    fn vec(item: &mut Item) -> (Vec<Var>, String) {
        let mut iit = mem::take(&mut item.group).into_iter();
        let mut enm: Vec<Var> = Vec::new();
        let mut err = String::new();
        let mut err_state = false;
        let dd = TokenStream::from_str("..").unwrap();
        let mut var = Var::default();
        while let Some(tt) = iit.next() {
            if err_state {
                match tt {
                    Punct(p) if p.as_char() == ',' => {
                        err_state = false;
                        item.group.extend(once(Punct(p)));
                        enm.push(mem::take(&mut var));
                    }
                    _ => (),
                }
            } else {
                match tt {
                    Punct(p) if p.as_char() == '#' && var.ident.is_none() => match iit.next() {
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
                        if var.ident.is_none() {
                            var.ident = Some(id.clone());
                            item.group.extend(once(Ident(id)));
                        // } else if id.to_string() == "fn" {
                        } else {
                            // method
                            let mut opt_tt = iit.next();
                            match opt_tt {
                                Some(Group(ref g)) if g.delimiter() == Delimiter::Parenthesis => {
                                    opt_tt = iit.next()
                                }
                                _ => (),
                            }
                            let opt_trait = match opt_tt {
                                Some(Ident(trait_id)) => {
                                    opt_tt = iit.next();
                                    Some(trait_id)
                                }
                                _ => None,
                            };
                            let in_enum_var =
                                format!("in `enum {}::{}`", item.name, var.ident.as_ref().unwrap());
                            match opt_tt {
                                Some(Group(block)) if block.delimiter() == Brace => {
                                    let name = (opt_trait.as_ref())
                                        .map_or(id.to_string(), |t| format!("{id}() {t}"));
                                    let m = VarMeth {
                                        ident: id,
                                        fields: var.fields.clone(),
                                        block,
                                        opt_trait,
                                    };
                                    if var.methods.insert(name.clone(), m).is_some() {
                                        err += &format!(
                                            "\nRepetition of method name `{name}` \
{in_enum_var} (last arm-block used)"
                                        );
                                    }
                                }
                                Some(tt2) => {
                                    err += &format!(
                                        "\nInvalid syntax in method `{id}` {in_enum_var} \
- expected arm-block: `{{...}}`, found: `{tt2}`"
                                    );
                                    err_state = true;
                                }
                                None => {
                                    err += &format!(
                                        "\nUnexpected end of macro on method`{id}` {in_enum_var}"
                                    );
                                    err_state = true;
                                }
                            };
                        }
                    }
                    Group(gr) if gr.delimiter() != Delimiter::Bracket => {
                        match (var.methods.is_empty(), var.fields.is_none()) {
                            (true, true) => {
                                var.fields = Some(Gr::new(gr.delimiter(), dd.clone()));
                                item.group.extend(once(Group(gr)));
                            }
                            (_, false) => var.fields = Some(gr),
                            _ => (),
                        }
                    }
                    Punct(p) if p.as_char() == ',' => {
                        if var.ident.is_some() {
                            item.group.extend(once(Punct(p)));
                            enm.push(mem::take(&mut var));
                        }
                    }
                    _ => (),
                }
            }
        }

        if var.ident.is_some() {
            enm.push(var)
        }
        (enm, err)
    }
}

/// This is an item-like macro that wraps a state `enum` declaration and one or more `impl` blocks, allowing you to write match-expressions without match-arms in the method bodies of these `impl`, writing the match-arms into the corresponding `enum` variants.
///
/// ## Usage example
///
/// [Chapter 17.3 "Implementing an Object-Oriented Design Pattern" of the rust-book](https://doc.rust-lang.org/book/ch17-03-oo-design-patterns.html) shows the implementation of the *state pattern* in Rust, which provides the following behavior:
/// ```rust ignore
/// pub fn main() {
///     let mut post = blog::Post::new();
///
///     post.add_text("I ate a salad for lunch today");
///     assert_eq!("", post.content());
///     post.request_review(); // without request_review() - approve() should not work
///     post.approve();  
///     assert_eq!("I ate a salad for lunch today", post.content());
/// }
/// ```
/// with the macro **`impl_match!`** this is solved like this:
/// ```rust
/// mod blog {
///     pub struct Post {
///         state: State,
///         content: String,
///     }
///
///     methods_enum::impl_match! {
///
///     impl Post {
///         pub fn add_text(&mut self, text: &str)  ~{ match self.state {} }
///         pub fn request_review(&mut self)        ~{ match self.state {} }
///         pub fn approve(&mut self)               ~{ match self.state {} }
///         pub fn content(&mut self) -> &str       ~{ match self.state { "" } }
///
///         pub fn new() -> Post {
///             Post { state: State::Draft, content: String::new() }
///         }
///     }
///
///     pub enum State {
///         Draft:          add_text(text)   { self.content.push_str(text) }
///                         request_review() { self.state = State::PendingReview },
///         PendingReview:  approve()        { self.state = State::Published },
///         Published:      content()        { &self.content }
///     }
///
///     } // <-- impl_match!
/// }
/// ```
/// All the macro does is complete the unfinished match-expressions in method bodies marked with `~` for all `enum` variants branches in the form:   
/// `(EnumName)::(Variant) => { match-arm block from enum declaration }`.  
/// If a `{}` block (without `=>`) is set at the end of an unfinished match-expressions, it will be placed in all variants branches that do not have this method in `enum`:   
/// `(EnumName)::(Variant) => { default match-arm block }`.  
/// Thus, you see all the code that the compiler will receive, but in a form structured according to the design pattern.
///
/// **rust-analyzer**[^rust_analyzer] perfectly defines identifiers in all blocks. All hints, auto-completions and replacements in the IDE are processed in match-arm displayed in `enum` as if they were in their native match-block. Plus, the "inline macro" command works in the IDE, displaying the resulting code.
///
/// [^rust_analyzer]: *rust-analyzer may not expand proc-macro when running under nightly or old rust edition.* In this case it is recommended to set in its settings: [`"rust-analyzer.server.extraEnv": { "RUSTUP_TOOLCHAIN": "stable" }`](https://rust-analyzer.github.io/manual.html#toolchain)
/// 
/// ## Other features
///
/// - You can also include `impl (Trait) for ...` blocks in a macro. The name of the `Trait` (without the path) is specified in the enum before the corresponding arm-block. Example with `Display` - below.
///
/// - An example of a method with generics is also shown there: `mark_obj<T: Display>()`.   
/// There is an uncritical nuance with generics, described in the [documentation](impl_match!#currently-this-mode-has-the-following-non-critical-restrictions).
///
/// - `@` - character before the `enum` declaration, in the example: `@enum Shape {...` disables passing to the `enum` compiler: only match-arms will be processed. This may be required if this `enum` is already declared elsewhere in the code, including outside the macro.
///
/// - If you are using `enum` with fields, then before the name of the method that uses them, specify the template for decomposing fields into variables (the IDE[^rust_analyzer] works completely correctly with such variables). The template to decompose is accepted by downstream methods of the same enumeration variant and can be reassigned. Example:
/// ```rust
/// methods_enum::impl_match! {
///
/// enum Shape<'a> {
/// //     Circle(f64, &'a str),                  // if you uncomment or remove these 4 lines
/// //     Rectangle { width: f64, height: f64 }, //    it will work the same
/// // }
/// // @enum Shape<'a> {
///     Circle(f64, &'a str): (radius, mark)
///         zoom(scale)    { Shape::Circle(radius * scale, mark) }      // template change
///         fmt(f) Display { write!(f, "{mark}(R: {radius:.1})") };     (_, mark) 
///         mark_obj(obj)  { format!("{} {}", mark, obj) };             (radius, _)
///         to_rect()      { *self = Shape::Rectangle { width: radius * 2., height: radius * 2.,} }
///     ,
///     Rectangle { width: f64, height: f64}: { width: w, height}
///         zoom(scale)    { Shape::Rectangle { width: w * scale, height: height * scale } }
///         fmt(f) Display { write!(f, "Rectangle(W: {w:.1}, H: {height:.1})") }; {..}
///         mark_obj(obj)  { format!("⏹️ {}", obj) }
/// }
/// impl<'a> Shape<'a> {
///     fn zoom(&mut self, scale: f64)                      ~{ *self = match *self }
///     fn to_rect(&mut self) -> &mut Self                  ~{ match *self {}; self }
///     fn mark_obj<T: Display>(&self, obj: &T) -> String   ~{ match self }
/// }
///
/// use std::fmt::{Display, Formatter, Result};
///
/// impl<'a> Display for Shape<'a>{
///     fn fmt(&self, f: &mut Formatter<'_>) -> Result      ~{ match self }
/// }
///
/// } // <--impl_match!
///
/// pub fn main() {
///     let mut rect = Shape::Rectangle { width: 10., height: 10. };
///     assert_eq!(format!("{rect}"), "Rectangle(W: 10.0, H: 10.0)");
///     rect.zoom(3.);
///     let mut circle = Shape::Circle(15., "⭕");
///     assert_eq!(circle.mark_obj(&rect.mark_obj(&circle)), "⭕ ⏹️ ⭕(R: 15.0)");
///     // "Rectangle(W: 30.0, H: 30.0)"
///     assert_eq!(circle.to_rect().to_string(), rect.to_string());
/// }
/// ```
/// - Debug flags. They can be placed through spaces in parentheses at the very beginning of the macro,   
/// eg: `impl_match! { (ns ) `...
///     - flag `ns` or `sn` in any case - replaces the semantic binding of the names of methods and traits in `enum` variants with a compilation error if they are incorrectly specified.
///     - flag `!` - causes a compilation error in the same case, but without removing the semantic binding.
///
/// ## [impl_match macro details](macro.impl_match.html#impl_match-macro-details)
#[doc = include_str!("impl_match_details.md")]
#[proc_macro]
pub fn impl_match(input_ts: TokenStream) -> TokenStream {
    // std::fs::write("target/debug/input_ts.log", format!("{}\n\n{0:#?}", input_ts)).unwrap();

    let (mut items, mmap, flags) = Item::vec(input_ts);
    let opt_enm_idx = (items.iter().enumerate().find_map(|(i, it)| it.no_def.then(|| i)))
        .or_else(|| items.iter().enumerate().find_map(|(i, it)| it.it_enum.then(|| i)));
    let ((mut enm, mut err), enm_i) =
        opt_enm_idx.map_or(((Vec::new(), String::new()), None), |i| {
            let enm_it = items.get_mut(i).unwrap();
            (Var::vec(enm_it), enm_it.ident.take())
        });
    let fat_arrow = TokenStream::from_str("=>").unwrap();
    let empty_gr = Gr::new(Brace, TokenStream::new());
    let dd = TokenStream::from_str("..").unwrap();
    let dd_gr = |g: &Gr| Gr::new(g.delimiter(), dd.clone());

    let mut res_ts = TokenStream::new();
    for item in items.iter_mut() {
        res_ts.extend(mem::take(&mut item.prev_ts));
        if !item.name.is_empty() && !item.no_def {
            let group = if item.it_enum {
                mem::take(&mut item.group)
            } else {
                let mut group = TokenStream::new();
                for mut m in mem::take(&mut item.methods) {
                    group.extend(m.prev_ts);
                    if !m.name.is_empty() {
                        let mut match_block = TokenStream::new();
                        for var in enm.iter_mut() {
                            let (fields, arm_block) = match var.methods.get_mut(&m.name) {
                                Some(VarMeth { fields, block, .. }) => {
                                    (fields.take(), mem::replace(block, empty_gr.clone()))
                                }
                                None => {
                                    if m.dflt_arm.is_none() {
                                        continue;
                                    }
                                    (var.fields.as_ref().map(dd_gr), m.dflt_arm.clone().unwrap())
                                }
                            };
                            match_block.extend(TokenStream::from_iter([
                                Ident(enm_i.as_ref().unwrap().clone()),
                                Punct(Pn::new(':', Spacing::Joint)),
                                Punct(Pn::new(':', Spacing::Alone)),
                                Ident(var.ident.as_ref().unwrap().clone()),
                            ]));
                            match_block.extend(fields.map(Group));
                            match_block.extend(fat_arrow.clone());
                            match_block.extend(once(Group(arm_block)));
                        }
                        m.body.extend(once(Group(Gr::new(Brace, match_block))).chain(m.tail));
                        group.extend(once(Group(Gr::new(Brace, m.body))));
                    }
                }
                group
            };
            res_ts.extend(once(Group(Gr::new(Brace, group))));
        }
    }

    // semantic+highlighting var methods / traits
    if !flags.no_semnt {
        if enm_i.is_some() {
            let item_n = (items.iter())
                .find_map(|it| (!it.it_enum && !it.name.is_empty()).then(|| it.name.clone()))
                .unwrap_or_default();
            let span = Span::call_site();
            let item_ts = TokenStream::from_iter([
                Ident(Idn::new(&item_n, span)),
                Punct(Pn::new(':', Spacing::Joint)),
                Punct(Pn::new(':', Spacing::Alone)),
            ]);
            let sm = Punct(Pn::new(';', Spacing::Alone));
            let mut fn_ts = TokenStream::new();
            if !item_n.is_empty() {
                for var in enm.iter_mut() {
                    for (k, m) in var.methods.iter_mut() {
                        if !mmap.get(k).map_or(false, |&v| v) {
                            fn_ts.extend(if let Some(trait_i) = m.opt_trait.take() {
                                TokenStream::from_iter([
                                    Punct(Pn::new('<', Spacing::Alone)),
                                    Ident(Idn::new(&item_n, span)),
                                    Ident(Idn::new("as", span)),
                                    Ident(trait_i),
                                    Punct(Pn::new('>', Spacing::Alone)),
                                    Punct(Pn::new(':', Spacing::Joint)),
                                    Punct(Pn::new(':', Spacing::Alone)),
                                ])
                            } else {
                                item_ts.clone()
                            });
                            fn_ts.extend([Ident(m.ident.clone()), sm.clone()]);
                        }
                    }
                }
            }
            if !fn_ts.is_empty() {
                let mut hasher = DefaultHasher::new();
                (item_n + "-" + mmap.keys().next().unwrap_or(&String::new())).hash(&mut hasher);
                res_ts.extend(
                    TokenStream::from_str(&format!(
                        r##"#[allow(unused)]
                            #[doc(hidden)]
                            #[doc = " Semantic bindings for impl_match! macro"]
                            mod _{}"##,
                        hasher.finish()
                    ))
                    .unwrap(),
                );
                let mut mod_ts = TokenStream::from_str("use super::*; fn methods()").unwrap();
                mod_ts.extend(once(Group(Gr::new(Brace, fn_ts))));
                res_ts.extend(once(Group(Gr::new(Brace, mod_ts))));
            }
        }
    }

    // errors
    if enm_i.is_some() {
        let mset: HashSet<String> = HashSet::from_iter(mmap.into_keys());
        let enm_n = enm_i.as_ref().unwrap().to_string();
        for var in enm.iter() {
            for name in var.methods.keys() {
                if !mset.contains(name) {
                    let mut free_m: Vec<String> = mset
                        .difference(&HashSet::from_iter(var.methods.keys().cloned()))
                        .cloned()
                        .collect();
                    free_m.sort();
                    let enm_var = format!("`enum {enm_n}::{}`", var.ident.as_ref().unwrap());
                    if free_m.is_empty() {
                        err += &format!(
                            "\nInvalid method `{name}` in {enm_var}:
`impl(-s)` contains no freely methods to implement `match{{...}}` from {enm_var}"
                        )
                    } else {
                        err += &format!(
                            "\nInvalid method name `{name}` in {enm_var} - expected{}: `{}`",
                            if free_m.len() == 1 { "" } else { " one of" },
                            free_m.join("`|`")
                        )
                    }
                };
            }
        }
    }
    if !err.is_empty() {
        eprintln!("\nErr in impl_match! macro:{err}\n");
        if flags.panic {
            panic!("Err in impl_match! macro:{err}");
        }
    }

    res_ts
}

// endregion: impl_match
