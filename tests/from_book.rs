use blog::Post;

#[test]
fn main() {
    let mut post = Post::new();

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
            Post {
                state: State::Draft,
                content: String::new(),
            }
        }
    }
}