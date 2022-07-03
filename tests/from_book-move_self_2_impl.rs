#[test]
fn test_main(){
    main();
}

fn main() {
    let mut post = blog::Post::new();

    post.add_text("I ate a salad for lunch today");
    assert_eq!("", post.content());

    assert_eq!("I ate a salad for lunch today", post.request_review().approve().content());
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
