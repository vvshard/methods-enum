In the popup hint above the enum name:

![enum popup hint](https://github.com/vvshard/methods-enum/raw/master/doc/img_0_2/UsageExample_1.png)

![enum popup: bodies](https://github.com/vvshard/methods-enum/raw/master/doc/img_0_2/UsageExample_2.png)

Alternatively, the entire result of a macro can be output to the console at compile time by setting the session environment variable M_ENUM_DBG to a value other than "0". PowerShell example:
```PowerShell
PS > $Env:M_ENUM_DBG=1
PS > cargo build
```
This is worth doing when the compiler messages are not clear and referring to the macro line , so that for debugging, replace the impl block along with the attribute with the output of the macro.

The macro reads only its impl block and only up to the name of the handler method. From which it follows that all method signatures for enum must be located before the handler method or in a separate from it impl block.

The handler method always has two arguments: `self` in the form corresponding to the method signatures, and the `enum` declared in the macro (*EnumName*).

The following example demonstrates the use of methods with `self` in the form of a move, in a separate `impl` block from their handler, which also contains the signatures of the `&mut self` methods and both handlers.

Let's say that in the blog::Post task, the state-changing methods require the form `self` move, to work with dot notation, while the rest of the methods need to be left on the form `&mut self`, or:
```rust
fn main() {
    let mut post = blog::Post::new();

    post.add_text("I ate a salad for lunch today");
    assert_eq!("", post.content());

    assert_eq!("I ate a salad for lunch today", post.request_review().approve().content());
}

// In this case, the solution might be:

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

    #[methods_enum::gen(Move, run_move)]
    impl Post {
        pub fn request_review(self) -> Post;
        pub fn approve(self) -> Post;
    }

    #[methods_enum::gen(Meth, run_methods)]
    impl Post {
        pub fn add_text(&mut self, text: &str);
        pub fn content(&mut self) -> &str;

        fn run_methods(&mut self, method: Meth) -> &str {
            match self.state {
                State::Draft => match method {
                    Meth::add_text(text) => {
                        self.content.push_str(text);
                        ""
                    }
                    _ => "",
                },
                State::PendingReview => "",
                State::Published => match method {
                    Meth::content() => &self.content,
                    _ => "",
                },
            }
        }

        fn run_move(mut self, method: Move) -> Post {
            match self.state {
                State::Draft => match method {
                    Move::request_review() => {
                        self.state = State::PendingReview;
                        self
                    }
                    _ => self,
                },
                State::PendingReview => match method {
                    Move::approve() => {
                        self.state = State::Published;
                        self
                    }
                    _ => self,
                },
                State::Published => self,
            }
        }

        pub fn new() -> Post {
            Post { state: State::Draft, content: String::new() }
        }
    }
}
```
Here fn run_move and/or fn run_methods can also be placed at the end of the first `impl` block.

Associated functions (for the syntax without *OutName* also and regular methods) can be in the `impl` block and before the handler method, interspersed with method signatures, but this worsens readability.

The best option for readability is to separate the signatures for `enum` into a separate `impl` block, ending with a handler method.

Methods arguments with &mut types work the same way. For example, to extend the blog::Post task to:
```rust 
fn main() {
    let mut post = blog::Post::new();

    let mut ext_content = "External content: ".to_string();

    post.add_text("I ate a salad for lunch today", &mut ext_content);
    assert_eq!("", post.content());
    assert_eq!("External content: I ate a salad for lunch today", ext_content);

    post.request_review();
    post.approve();
    assert_eq!("I ate a salad for lunch today", post.content());
}

// the solution might look like this:

mod blog {
// . . .                    
#   enum State {
#       Draft,
#       PendingReview,
#       Published,
#   }
#
#   pub struct Post {
#       state: State,
#       content: String,
#   }
#
// . . .                    
    #[methods_enum::gen(Meth, run_methods)]
    impl Post {
        pub fn add_text(&mut self, text: &str, ex_content: &mut String);
        pub fn request_review(&mut self);
        pub fn approve(&mut self);
        pub fn content(&mut self) -> &str;

        fn run_methods(&mut self, method: Meth) -> &str {
            match self.state {
                State::Draft => match method {
                    Meth::add_text(text, ex_cont) => {
                        self.content.push_str(text);
                        ex_cont.push_str(text);
                        ""
                    }
// . . .                    
#                   Meth::request_review() => {
#                       self.state = State::PendingReview;
#                       ""
#                   }
#                   _ => "",
#               },
#               State::PendingReview => match method {
#                   Meth::approve() => {
#                       self.state = State::Published;
#                       ""
#                   }
#                   _ => "",
#               },
#               State::Published => match method {
#                   Meth::content() => &self.content,
#                   _ => "",
#               },
#           }
#       }
#
#       pub fn new() -> Post {
#           Post { state: State::Draft, content: String::new() }
#       }
#   }
// . . .    
}
```
## Syntax variant with *OutName*

**`#[methods_enum::gen(`*EnumName* `, ` | `: ` *handler_name* `, ` | ` = ` *OutName* `!`<sup>?</sup> `)]`**

where:
- ***OutName***: The name of an automatically generated enum with variants from the return types.

Replacing the delimiter **`, `** before *OutName* with **` = `** will automatically add the `#[derive(Debug)]` attribute to the `enum`.

Setting `!` after *OutName* enables checking the returned variant by its name, not by its type.

This variant allows you to go beyond one meaningful return type, but forces the handler method to enclose all return values in `enum` *OutName*. The unwrapping will be done in auto-generated method bodies.

`enum` *OutName* includes only variants with return types named like methods, and one variant named `Unit` for methods without return values or possibly as trigger for default values. In addition, the `.stype()` method of `enum` is generated, which returns a string representation of the enum value type for diagnostic messages.

In the generated method bodies, a variant of `enum` *OutName* that matches the type it contains with the return type in the method signature is unwrapped to the return type value, otherwise the method panics with a type mismatch message. If you want to panic if the `enum` variant *OutName* does not nominally match the method name, set the **`!`** after *OutName* in the macro attribute.

It is possible to replace the type mismatch panic with a default expression by specifying it after the method signature in braces.

As an example, let's make all methods except `content()` of our `blog::Post` output a `Result<&State, String>` type, with `Ok()` reflects the `Post` state after the method and `Err()` - method impossibility message:
```rust
use blog::{Post, State};

fn main() {
    let mut post = Post::new();

    assert_eq!( post.add_text("I ate a salad for lunch today"), Ok(&State::Draft) );

    assert_eq!(
        post.approve(),
        Err("For State::Draft method 'approve' is not possible".to_string())
    );

    assert_eq!(post.request_review(), Ok(&State::PendingReview));
    assert_eq!(post.content(), "");

    assert_eq!(post.approve(), Ok(&State::Published));
    assert_eq!(post.content(), "I ate a salad for lunch today");
}

mod blog {

    #[derive(Debug, PartialEq, Clone, Copy)]
    pub enum State {
        Draft,
        PendingReview,
        Published,
    }

    pub struct Post {
        state: State,
        content: String,
    }

    #[methods_enum::gen(Meth: run_methods, Out)]
    impl Post {
        pub fn add_text(&mut self, text: &str) -> Result<&State, String>;
        pub fn request_review(&mut self) -> Result<&State, String>;
        pub fn approve(&mut self) -> Result<&State, String>;
        #[rustfmt::skip]
        pub fn content(&mut self) -> &str { "" } // default value

        fn run_methods(&mut self, method: Meth) -> Out {
            match self.state {
                State::Draft => match method {
                    Meth::add_text(text) => {
                        self.content.push_str(text);
                        Out::add_text(Ok(&self.state))
                    }
                    Meth::request_review() => {
                        self.state = State::PendingReview;
                        Out::request_review(Ok(&self.state))
                    }
                    m => self.method_not_possible(m),
                },

                State::PendingReview => match method {
                    Meth::approve() => {
                        self.state = State::Published;
                        Out::approve(Ok(&self.state))
                    }
                    m => self.method_not_possible(m),
                },

                State::Published => match method {
                    Meth::content() => Out::content(&self.content),
                    m => self.method_not_possible(m),
                },
            }
        }

        fn method_not_possible(&self, act: Meth) -> Out {
            Out::request_review(Err(format!(
                "For State::{:?} method '{act:?}' is not possible",
                self.state
            )))
        }

        pub fn new() -> Post {
            Post { state: State::Draft, content: String::new() }
        }
    }
}
```
The `enum Out` declaration and the generated method bodies can be seen in the tooltip:

![enum popup hint](https://github.com/vvshard/methods-enum/raw/master/doc/img_0_2/OutName_1.png)

The macro passes the attributes and doc comments of the methods signatures to the compiler unchanged, without displaying them in the doc comment of the `enum`. Regular comments are skipped.
![enum popup: bodies](https://github.com/vvshard/methods-enum/raw/master/doc/img_0_2/OutName_2.png)

As you might guess from the last screenshot, the default value expression can use a return from a handler method in a variable with a name derived from *OutName* by converting it to lower case and preceding it with an underscore.

For example, if in the `content()` method we need to return not `&str`, but `Result<&str, String>`, then in the expression for the default value `content()` we should put the Err conversion from the `Result<&State, String>` to the type `Result<&str, String>`:
```rust
use blog::{Post, State};

fn main() {
// . . .
#    let mut post = Post::new();
#
#    assert_eq!( post.add_text("I ate a salad for lunch today"), Ok(&State::Draft) );
#
#    assert_eq!(
#        post.approve(),
#        Err("For State::Draft method 'approve' is not possible".to_string())
#    );
// . . .
    assert_eq!(post.request_review(), Ok(&State::PendingReview));
    assert_eq!(
        post.content(),
        Err("For State::PendingReview method 'content' is not possible".to_string())
    );

    assert_eq!(post.approve(), Ok(&State::Published));
    assert_eq!(post.content(), Ok("I ate a salad for lunch today"));
}

mod blog {
// . . .
#    #[derive(Debug, PartialEq, Clone, Copy)]
#    pub enum State {
#        Draft,
#        PendingReview,
#        Published,
#    }
#
#    pub struct Post {
#        state: State,
#        content: String,
#    }
// . . .
    #[methods_enum::gen(Meth: run_methods = Out)]
    impl Post {
        pub fn add_text(&mut self, text: &str) -> Result<&State, String>;
        pub fn request_review(&mut self) -> Result<&State, String>;
        pub fn approve(&mut self) -> Result<&State, String>;
        #[rustfmt::skip]
        pub fn content(&mut self) -> Result<&str, String> { match _out {
                    Out::request_review(Err(e)) => Err(e),   // default value
                    _ => panic!("Type mismatch in the content() method"), // never
                }}

        fn run_methods(&mut self, method: Meth) -> Out {
            match self.state {
// . . .
#               State::Draft => match method {
#                   Meth::add_text(text) => {
#                       self.content.push_str(text);
#                       Out::add_text(Ok(&self.state))
#                   }
#                   Meth::request_review() => {
#                       self.state = State::PendingReview;
#                       Out::request_review(Ok(&self.state))
#                   }
#                   m => self.method_not_possible(m),
#               },
#
#               State::PendingReview => match method {
#                   Meth::approve() => {
#                       self.state = State::Published;
#                       Out::approve(Ok(&self.state))
#                   }
#                   m => self.method_not_possible(m),
#               },
// . . .
                State::Published => match method {
                    Meth::content() => Out::content(Ok(&self.content)),
                    m => self.method_not_possible(m),
                },
// . . .
#           }
#       }
#
#       fn method_not_possible(&self, act: Meth) -> Out {
#           Out::request_review(Err(format!(
#               "For State::{:?} method '{act:?}' is not possible",
#               self.state
#           )))
#       }
#
#       pub fn new() -> Post {
#           Post { state: State::Draft, content: String::new() }
#       }
#   }
// . . .
}
```
![enum popup hint](https://github.com/vvshard/methods-enum/raw/master/doc/img_0_2/OutNameRR_1.png)

![enum popup: bodies](https://github.com/vvshard/methods-enum/raw/master/doc/img_0_2/OutNameRR_2.png)

___
All examples as .rs files plus state_machine.rs file and from_book-task_and_2_result.rs file with extension to book task and using `Unit` are located in the directory: <https://github.com/vvshard/methods-enum/tree/master/tests/gen>
