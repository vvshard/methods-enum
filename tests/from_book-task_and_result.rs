use blog::Post;

#[test]
fn main() {
    let mut post = Post::new();

    assert_eq!(
        post.add_text("I ate a salad for lunch today"),
        Ok("I ate a salad for lunch today")
    );
    assert_eq!(post.content(), "");

    assert!(post.request_review().is_ok());
    assert_eq!(post.content(), "");

    assert_eq!(
        post.add_text("\nI'm hungry"),
        Err("For State::PendingReview { number_approvals: 0 } method 'add_text(\"\\nI'm hungry\")' is not possible".to_string())
    );
    assert!(post.reject().is_ok());
    assert_eq!(
        post.add_text("\nI'm hungry - 2!!"),
        Ok("I ate a salad for lunch today\nI'm hungry - 2!!")
    );
    assert_eq!(
        post.approve(),
        Err("For State::Draft method 'approve' is not possible".to_string())
    );
    assert!(post.request_review().is_ok());
    assert_eq!(
        post.approve(),
        Ok("State::PendingReview { number_approvals: 1 }")
    );
    assert_eq!(post.content(), "");
    assert_eq!(post.approve(), Ok("State::Published"));
    assert_eq!(
        post.content(),
        "I ate a salad for lunch today\nI'm hungry - 2!!"
    );

    // assert_eq!(Ok("I ate a salad for lunch today"), post.content()");
}

mod blog {

    #[derive(Debug, PartialEq, Clone, Copy)]
    pub enum State {
        Draft,
        PendingReview { number_approvals: u32 },
        Published,
    }

    pub struct Post {
        state: State,
        content: String,
    }

    #[methods_enum::gen(Meth: run_methods = Out)]
    impl Post {
        pub fn add_text(&mut self, text: &str) -> Result<&str, String>;
        pub fn request_review(&mut self) -> Result<&str, String>;
        pub fn reject(&mut self) -> Result<&str, String>;
        pub fn approve(&mut self) -> Result<&str, String>;
        pub fn content(&mut self) -> &str {
            ""
        }
    }

    impl Post {
        // #[rustfmt::skip]
        fn run_methods(&mut self, method: Meth) -> Out {
            match self.state {
                State::Draft => match method {
                    Meth::add_text(text) => {
                        self.content.push_str(text);
                        Out::add_text(Ok(&self.content))
                    }
                    Meth::request_review() => {
                        self.state = State::PendingReview {
                            number_approvals: 0,
                        };
                        Out::request_review(Ok(""))
                    }
                    m => self.method_not_possible(m),
                },

                State::PendingReview { number_approvals } => match method {
                    Meth::approve() => {
                        if number_approvals == 1 {
                            self.state = State::Published;
                            Out::approve(Ok("State::Published"))
                        } else {
                            self.state = State::PendingReview {
                                number_approvals: 1,
                            };
                            Out::approve(Ok("State::PendingReview { number_approvals: 1 }"))
                        }
                    }
                    Meth::reject() => {
                        self.state = State::Draft;
                        Out::reject(Ok(""))
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
            Out::add_text(Err(format!(
                "For State::{:?} method '{act:?}' is not possible",
                self.state
            )))
        }

        pub fn new() -> Post {
            Post {
                state: State::Draft,
                content: String::new(),
            }
        }
    }
}