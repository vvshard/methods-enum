use blog::{Post, State};

#[test]
fn test_main(){
    main();
}

fn main() {
    let mut post = Post::new();

    assert_eq!(post.add_text("I ate a salad for lunch today"), Ok(&State::Draft));

    assert_eq!(
        post.approve(),
        Err("For State::Draft method 'approve' is not possible".to_string())
    );

    assert_eq!(post.request_review(), Ok(&State::PendingReview));
    assert_eq!(
        post.content(),
        Err("For State::PendingReview method 'content' is not possible".to_string())
    );

    assert_eq!(post.approve(), Ok(&State::Published));
    assert_eq!(post.content(), Ok("I ate a salad for lunch today"));
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
        pub fn content(&mut self) -> Result<&str, String> { match _out {
                    Out::request_review(Err(e)) => Err(e),   // default value
                    _ => panic!("Type mismatch in the content() method"), // never
                }}

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
                    Meth::content() => Out::content(Ok(&self.content)),
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
