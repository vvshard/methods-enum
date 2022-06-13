#[test]
fn main() {
    let mut post = blog::Post::new();

    post.add_text("I ate a salad for lunch today");
    assert_eq!("", post.content());

    post.request_review();
    assert_eq!("", post.content());

    post.approve();
    assert_eq!("I ate a salad for lunch today", post.content());
}

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

    #[methods_enum::gen(Meth: run_methods)]
    impl Post {
        pub fn add_text(&mut self, text: &str);
        pub fn request_review(&mut self);
        pub fn approve(&mut self);

        fn run_methods(&mut self, method: Meth){
            match self.state {
                State::Draft => match method {
                    Meth::add_text(text) => self.content.push_str(text),
                    Meth::request_review() =>  self.state = State::PendingReview,
                    _ => (),
                },
                State::PendingReview => match method {
                    Meth::approve() =>  self.state = State::Published,
                    _ => (),
                },
                State::Published => match method {
                    _ => (),
                },
            }
        }

        pub fn content(&self) -> &str{
            match self.state {
                State::Published => &self.content,
                _ => ""
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
