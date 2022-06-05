use blog::Post;

use crate::blog::State;

#[test]
fn main() {
    let mut post = Post::new();

    assert_eq!(
        post.add_text("I ate a salad for lunch today"),
        Ok("I ate a salad for lunch today")
    );
    assert_eq!(post.content(), "");

    assert_eq!(
        post.request_review(),
        Ok(&State::PendingReview {
            number_approvals: 0
        })
    );
    assert_eq!(post.content(), "");

    assert_eq!(
        post.add_text("\nI'm hungry"),
        Err("For State::PendingReview { number_approvals: 0 } \
        method 'add_text(\"\\nI'm hungry\")' is not possible"
            .to_string())
    );
    post.reject();
    assert_eq!(
        post.add_text("\nSecond time: I'm hungry!!"),
        Ok("I ate a salad for lunch today\nSecond time: I'm hungry!!")
    );
    assert_eq!(
        post.approve(),
        Err("For State::Draft method 'approve' is not possible".to_string())
    );
    assert_eq!(
        post.request_review(),
        Ok(&State::PendingReview {
            number_approvals: 0
        })
    );
    assert_eq!(
        post.approve(),
        Ok(&State::PendingReview {
            number_approvals: 1
        })
    );
    assert_eq!(post.content(), "");
    assert_eq!(post.approve(), Ok(&State::Published));
    assert_eq!(
        post.content(),
        "I ate a salad for lunch today\nSecond time: I'm hungry!!"
    );

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
        pub fn request_review(&mut self) -> Result<&State, String>;
        pub fn reject(&mut self);
        pub fn approve(&mut self) -> Result<&State, String>;
        pub fn content(&mut self) -> &str {
            ""
        }

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
                        Out::request_review(Ok(&self.state))
                    }
                    m => self.method_not_possible(m),
                },

                State::PendingReview { number_approvals } => match method {
                    Meth::approve() => {
                        if number_approvals == 1 {
                            self.state = State::Published;
                            Out::approve(Ok(&self.state))
                        } else {
                            self.state = State::PendingReview {
                                number_approvals: number_approvals + 1,
                            };
                            Out::approve(Ok(&self.state))
                        }
                    }
                    Meth::reject() => {
                        self.state = State::Draft;
                        Out::Unit
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
            let err_s = format!(
                "For State::{:?} method '{act:?}' is not possible",
                self.state
            );
            match act {
                Meth::add_text(_) => Out::add_text(Err(err_s)),
                _ => Out::request_review(Err(err_s)),
            }
        }

        pub fn new() -> Post {
            Post {
                state: State::Draft,
                content: String::new(),
            }
        }
    }
}
