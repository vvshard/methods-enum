[![crates.io](https://img.shields.io/crates/v/methods-enum.svg)](https://crates.io/crates/methods-enum) [![Docs.rs](https://img.shields.io/docsrs/methods-enum)](https://docs.rs/methods-enum)

*State* design pattern and other dynamic polymorphism are often solved with dyn Trait objects.

**enum-matching** is simpler and more efficient than Trait objects, but using it directly in this situation will "smear" the state abstraction over interface methods.

The proposed macros [**`impl_match!{...}`**](#impl_match-macro) and [**`#[gen(...)]`**](#gen-macro) provide two different ways of enum-matching with a visual grouping of methods by `enum` variants, which makes it convenient to use enum-matching in *state* / dynamic polymorphism problems.
___
# impl_match! macro
This is an item-like macro that wraps a state `enum` declaration and one or more `impl` blocks, allowing you to write match-expressions without match-arms in the method bodies of these `impl`, writing the match-arms into the corresponding `enum` variants.

## Usage example 

[Chapter 17.3 "Implementing an Object-Oriented Design Pattern" of the rust-book](https://doc.rust-lang.org/book/ch17-03-oo-design-patterns.html) shows the implementation of the *state pattern* in Rust, which provides the following behavior:
```rust ignore
pub fn main() {
    let mut post = blog::Post::new();

    post.add_text("I ate a salad for lunch today");
    assert_eq!("", post.content());
    post.request_review(); // without request_review() - approve() should not work
    post.approve();  
    assert_eq!("I ate a salad for lunch today", post.content());
}
```
By setting in Cargo.toml:
```toml
[dependencies]
methods-enum = "0.3.0"
```
this can be solved, for example, like this:
```rust
mod blog {
    pub struct Post {
        state: State,
        content: String,
    }

    methods_enum::impl_match! {

    impl Post {
        pub fn add_text(&mut self, text: &str)  ~{ match self.state {} }
        pub fn request_review(&mut self)        ~{ match self.state {} }
        pub fn approve(&mut self)               ~{ match self.state {} }
        pub fn content(&mut self) -> &str       ~{ match self.state { "" } }

        pub fn new() -> Post {
            Post { state: State::Draft, content: String::new() }
        }
    }

    pub enum State {
        Draft:          add_text(text)   { self.content.push_str(text) }
                        request_review() { self.state = State::PendingReview },
        PendingReview:  approve()        { self.state = State::Published },
        Published:      content()        { &self.content }
    }

    } // <-- impl_match!
}
```
All the macro does is complete the unfinished match-expressions in method bodies marked with `~` for all `enum` variants branches in the form  
`(EnumName)::(Variant) => { match-arm block from enum declaration }`.  
Instead of the methods omitted in the `enum` declaration, in resulting `match` will be:  
`(EnumName)::(Variant) => {}`.  
Thus, you see all the code that the compiler will receive, but in a form structured according to the design pattern.

**rust-analyzer**[^rust_analyzer] sees perfectly in this example that `x` is the same variable in both blocks. All hints, auto-completions and replacements in the IDE are processed in match-arm displayed in `enum` as if they were in their own match-block. Plus, the "inline macro" command works in the IDE, displaying the resulting code.

[^rust_analyzer]: *rust-analyzer may not expand proc-macro when running under nightly or old rust edition.* In this case it is recommended to set in its settings: [`"rust-analyzer.server.extraEnv": { "RUSTUP_TOOLCHAIN": "stable" }`](https://rust-analyzer.github.io/manual.html#toolchain)

## Other features

- You can also include `impl (Trait) for ...` blocks in a macro. Example with `Display` - below.

- If you are using `enum` with fields, then before the name of the method that uses them, specify the template for decomposing fields into variables (the IDE[^rust_analyzer] works completely correctly with such variables). Example:
```rust
methods_enum::impl_match! {

enum Shape {
    Circle(f64): (r)
        zoom(scale)    { Shape::Circle(r * scale) }
        to_rect()      { *self = Shape::Rectangle { width: r * 2., height: r * 2. } }
        fmt(f) Display { write!(f, "Circle(R: {r:.1})") }
    ,
    Rectangle { width: f64, height: f64 }: { width: w, height }
        zoom(scale)    { Shape::Rectangle { width: w * scale, height: height * scale } }
        fmt(f) Display { write!(f, "Rectangle(W: {w:.1}, H: {height:.1})") }
}
impl Shape {
    fn zoom(self, scale: f64) -> Shape              ~{ match self }
    fn to_rect(&mut self)                           ~{ match *self {} }
}

use std::fmt::{Display, Formatter, Result};
impl Display for Shape {
    fn fmt(&self, f: &mut Formatter<'_>) -> Result  ~{ match self }
}
} // <-- impl_match!

pub fn main() {
    let rect = Shape::Rectangle { width: 10., height: 10. };
    assert_eq!(format!("{rect}"), "Rectangle(W: 10.0, H: 10.0)");

    let mut circle = Shape::Circle(15.);
    assert_eq!(circle.to_string(), "Circle(R: 15.0)");
    circle.to_rect();
    assert_eq!(circle.to_string(), rect.zoom(3.).to_string()); // "Rectangle(W: 30.0, H: 30.0)" 
}
```

- `@` - character before the `enum` declaration, eg: `@enum State {...` disables passing to the `enum` compiler: only match-arms will be processed. This may be required if this `enum` is already declared elsewhere in the code.

## Links

- [A detailed description of the `impl_match!` macro - in the documentation]().

- [Code examples with `impl_match!`]().
___
# gen() macro

The macro attribute is set before an individual (non-Trait) impl block. Based on the method signatures of the impl block, it generates: `enum` with parameters from argument tuples and generates `{}` bodies of these methods with calling the argument handler method from this `enum`.  
This allows the handler method to control the behavior of methods depending on the context, including structuring enum-matching by state.

## Usage example

Let me remind you of the condition from [chapter 17.3 "Implementing an Object-Oriented Design Pattern" of the rust-book](https://doc.rust-lang.org/book/ch17-03-oo-design-patterns.html). The following behavior is required:
```rust ignore
pub fn main() {
    let mut post = blog::Post::new();

    post.add_text("I ate a salad for lunch today");
    assert_eq!("", post.content());
    post.request_review(); // without request_review() - approve() should not work
    post.approve();  
    assert_eq!("I ate a salad for lunch today", post.content());
}
```
with macro #[gen()] this is solved like this:
```rust ignore
mod blog {
    enum State {
        Draft,
        PendingReview,
        Published,
    }

    pub struct Post {
        state: State,
        content: String,
    }

    #[methods_enum::gen(Meth, run_methods)]
    impl Post {
        pub fn add_text(&mut self, text: &str);
        pub fn request_review(&mut self);
        pub fn approve(&mut self);
        pub fn content(&mut self) -> &str;
        #[rustfmt::skip]
        fn run_methods(&mut self, method: Meth) -> &str {
            match self.state {
                State::Draft => match method {
                    Meth::add_text(text) => { self.content.push_str(text); "" }
                    Meth::request_review() => { self.state = State::PendingReview; "" }
                    _ => "",
                },
                State::PendingReview => match method {
                    Meth::approve() => { self.state = State::Published; "" }
                    _ => "",
                },
                State::Published => match method {
                    Meth::content() => &self.content,
                    _ => "",
                },
            }
        }

        pub fn new() -> Post {
            Post { state: State::Draft, content: String::new() }
        }
    }
}
```
In the handler method (in this case, `run_methods`), simply write for each state which methods should work and how.

The macro duplicates the output for the compiler in the doc-comments. Therefore, in the IDE[^rust_analyzer], you can always see the declaration of the generated `enum` and the generated method bodies, in the popup hint above the enum name:

![enum popup hint](https://github.com/vvshard/methods-enum/raw/master/doc/img_0_2/UsageExample_1.png)

![enum popup: bodies](https://github.com/vvshard/methods-enum/raw/master/doc/img_0_2/UsageExample_2.png)

## Syntax for calling a macro
### For at most one return type from methods
**`#[methods_enum::gen(`*EnumName* `, ` *handler_name*]`**

where:
- ***EnumName***: The name of the automatically generated enum.
- ***handler_name***: Handler method name
### For more than one return type from methods

**`#[methods_enum::gen(`*EnumName* `, ` *handler_name* `, ` *OutName*`]`**

where:
- ***OutName***: The name of an automatically generated enum with variants from the return types.

## Links

- [Detailed description of macro `#[gen(....)]` - in documentation]().

- [Code examples with `#[gen(....)]`]().
___
The advantage of `gen()` over `impl_match!` is that it allows you to see the entire `match` expression and process more complex logic, including those with non-trivial incoming `match` expressions, `match guard` and nested `match` from substate enums.
But `gen()` loses out to `impl_match!` in terms of [limitations]() and ease of working with methods and their output values.
___
# License
MIT or Apache-2.0 license of your choice.
