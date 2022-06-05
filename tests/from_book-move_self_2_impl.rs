use blog::Post;

#[test]
fn main() {
    let mut post = Post::new();

    post.add_text("I ate a salad for lunch today");

    post = post.request_review().approve();

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

    #[methods_enum::gen(Move: run_move)]
    impl Post {
        pub fn request_review(self) -> Post;
        pub fn approve(self) -> Post;
    }

    #[methods_enum::gen(Meth: run_methods)]
    impl Post {
        pub fn add_text(&mut self, text: &str);
        pub fn content(&mut self) -> &str;

        #[rustfmt::skip]
        fn run_methods(&mut self, method: Meth) -> &str {
            match self.state {
                State::Draft => match method {
                    Meth::add_text(text) => { self.content.push_str(text); "" }
                    _ => "",
                },
                State::PendingReview => "",
                State::Published => match method {
                    Meth::content() => &self.content,
                    _ => "",
                },
            }
        }

        #[rustfmt::skip]
        fn run_move(mut self, method: Move) -> Post {
            match self.state {
                State::Draft => match method {
                    Move::request_review() => { self.state = State::PendingReview; self }
                    _ => self,
                },
                State::PendingReview => match method {
                    Move::approve() => { self.state = State::Published; self }
                    _ => self,
                },
                State::Published => self,
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
