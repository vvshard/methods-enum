pub fn main() {
    let mut post = blog::Post::new();

    post.add_text("I ate a salad for lunch today");
    assert_eq!("", post.content());

    assert_eq!("I ate a salad for lunch today", post.request_review().approve().content());
}

mod blog {

    pub struct Post {
        state: State,
        content: String,
    }

    methods_enum::impl_match!{
    impl Post {
        pub fn request_review(mut self) -> Post     { match self.state {}; self }
        pub fn approve(mut self) -> Post            { match self.state {}; self }
    }
    enum State {
        Draft: request_review() { self.state = State::PendingReview },
        PendingReview: approve() { self.state = State::Published },
        Published
    }
    } // impl_match!

    methods_enum::impl_match!{
    impl Post {
        pub fn add_text(&mut self, text: &str)      { match self.state }
        pub fn content(&mut self) -> &str           { let mut x = ""; match self.state {}; x }

        pub fn new() -> Post {
            Post { state: State::Draft, content: String::new() }
        }
    }

    @enum State {
        Draft: add_text(text) { self.content.push_str(text) },
        PendingReview,
        Published: content() { x = &self.content }
    }
    } // impl_match!
}
