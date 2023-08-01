[![crates.io](https://img.shields.io/crates/v/methods-enum.svg)](https://crates.io/crates/methods-enum) [![Docs.rs](https://img.shields.io/docsrs/methods-enum)](https://docs.rs/methods-enum)

State design pattern and other dynamic polymorphism are often solved with dyn Trait objects.

**enum-matching** is simpler and more efficient than Trait objects, but using it directly in this situation will "smear" the state abstraction over interface methods.

The proposed macros [**`impl_match!{...}`**](#impl_match-macro) and [**`#[gen(...)]`**](#gen-macro) provide two different ways of enum-matching with a visual grouping of methods by `enum` variants, which makes it convenient to use enum-matching in state design pattern and dynamic polymorphism problems.
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
methods-enum = "0.3.1"
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
All the macro does is complete the unfinished match-expressions in method bodies marked with `~` for all `enum` variants branches in the form:   
`(EnumName)::(Variant) => { match-arm block from enum declaration }`.  
If a `{}` block (without `=>`) is set at the end of an unfinished match-expressions, it will be placed in all variants branches that do not have this method in `enum`:   
`(EnumName)::(Variant) => { default match-arm block }`.  
Thus, you see all the code that the compiler will receive, but in a form structured according to the design pattern.

**rust-analyzer**[^rust_analyzer] perfectly defines identifiers in all blocks. All hints, auto-completions and replacements in the IDE are processed in match-arm displayed in `enum` as if they were in their native match-block. Plus, the "inline macro" command works in the IDE, displaying the resulting code.

[^rust_analyzer]: *rust-analyzer may not expand proc-macro when running under nightly or old rust edition.* In this case it is recommended to set in its settings: [`"rust-analyzer.server.extraEnv": { "RUSTUP_TOOLCHAIN": "stable" }`](https://rust-analyzer.github.io/manual.html#toolchain)

## Other features

- You can also include `impl (Trait) for ...` blocks in a macro. The name of the `Trait` (without the path) is specified in the enum before the corresponding arm-block. Example with `Display` - below.

- An example of a method with generics is also shown there: `mark_obj<T: Display>()`.   
There is an uncritical nuance with generics, described in the [documentation](https://docs.rs/methods-enum/latest/methods_enum/macro.impl_match.html#currently-this-mode-has-the-following-non-critical-restrictions).

- `@` - character before the `enum` declaration, in the example: `@enum Shape {...` disables passing to the `enum` compiler: only match-arms will be processed. This may be required if this `enum` is already declared elsewhere in the code, including outside the macro.

- If you are using `enum` with fields, then before the name of the method that uses them, specify the template for decomposing fields into variables (the IDE[^rust_analyzer] works completely correctly with such variables). The template to decompose is accepted by downstream methods of the same enumeration variant and can be reassigned. Example:
```rust
methods_enum::impl_match! {

enum Shape<'a> {
//     Circle(f64, &'a str),                  // if you uncomment or remove these 4 lines 
//     Rectangle { width: f64, height: f64 }, //    it will work the same
// }
// @enum Shape<'a> {
    Circle(f64, &'a str): (radius, mark)
        zoom(scale)    { Shape::Circle(radius * scale, mark) }      // template change
        fmt(f) Display { write!(f, "{mark}(R: {radius:.1})") };     (_, mark)
        mark_obj(obj)  { format!("{} {}", mark, obj) };             (radius, _)
        to_rect()      { *self = Shape::Rectangle { width: radius * 2., height: radius * 2.,} }
    ,
    Rectangle { width: f64, height: f64}: { width: w, height}
        zoom(scale)    { Shape::Rectangle { width: w * scale, height: height * scale } }
        fmt(f) Display { write!(f, "Rectangle(W: {w:.1}, H: {height:.1})") }; {..}
        mark_obj(obj)  { format!("⏹️ {}", obj) }
}
impl<'a> Shape<'a> {
    fn zoom(&mut self, scale: f64)                      ~{ *self = match *self }
    fn to_rect(&mut self) -> &mut Self                  ~{ match *self {}; self }
    fn mark_obj<T: Display>(&self, obj: &T) -> String   ~{ match self }
}

use std::fmt::{Display, Formatter, Result};

impl<'a> Display for Shape<'a>{
    fn fmt(&self, f: &mut Formatter<'_>) -> Result      ~{ match self }
}

} // <--impl_match!

pub fn main() {
    let mut rect = Shape::Rectangle { width: 10., height: 10. };
    assert_eq!(format!("{rect}"), "Rectangle(W: 10.0, H: 10.0)");
    rect.zoom(3.);
    let mut circle = Shape::Circle(15., "⭕");
    assert_eq!(circle.mark_obj(&rect.mark_obj(&circle)), "⭕ ⏹️ ⭕(R: 15.0)");
    // "Rectangle(W: 30.0, H: 30.0)"
    assert_eq!(circle.to_rect().to_string(), rect.to_string());
}
```
- Debug flags. They can be placed through spaces in parentheses at the very beginning of the macro,   
eg: `impl_match! { (ns ) `...
    - flag `ns` or `sn` in any case - replaces the semantic binding of the names of methods and traits in `enum` variants with a compilation error if they are incorrectly specified.
    - flag `!` - causes a compilation error in the same case, but without removing the semantic binding.


## Links

- [A detailed description of the `impl_match!` macro - in the documentation](https://docs.rs/methods-enum/latest/methods_enum/macro.impl_match.html#impl_match-macro-details).

- [Code examples with `impl_match!`](https://github.com/vvshard/methods-enum/tree/master/tests/impl_match).
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
with macro `#[gen()]` this is solved like this:
```rust
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

- [Detailed description of macro `#[gen(....)]` - in documentation](https://docs.rs/methods-enum/latest/methods_enum/attr.gen.html#gen-macro-details-and-use-cases).

- [Code examples with `#[gen(....)]`](https://github.com/vvshard/methods-enum/tree/master/tests/impl_match).
___
The gen() macro loses out to impl_match! in terms of [restrictions](https://docs.rs/methods-enum/latest/methods_enum/attr.gen.html#restrictions) and ease of working with methods and their output values.
The benefit of gen() is that it allows you to see the full match-expression and handle more complex logic, including those with non-trivial incoming expressions, match guards, and nested matches from substate enums.
___
# License
MIT or Apache-2.0 license of your choice.
___
