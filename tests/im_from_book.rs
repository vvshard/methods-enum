#[test]
fn test_main() {
    main();
}

fn main() {
    let mut post = blog::Post::new();

    post.add_text("I ate a salad for lunch today");
    assert_eq!("", post.content());

    post.request_review();
    assert_eq!("", post.content());

    post.approve();
    assert_eq!("I ate a salad for lunch today", post.content());

    #[allow(unused)]
    let check_doc = blog::State::PendingReview;
}

mod blog {
    pub struct Post {
        state: State,
        content: String,
    }

    methods_enum::impl_match! {
    impl Post {
        pub fn add_text(&mut self, text: &str)    { match self.state }
        pub fn request_review(&mut self)          { match self.state }
        /// doc approve
        pub fn approve(&mut self)                 { match self.state }
        pub fn content(&mut self) -> &str         { match self.state }

        /// doc Post::new()
        pub fn new() -> Post {
            Post {
                state: State::Draft,
                content: String::new(),
            }
        }
    }

    /// doc State
    pub enum State {
        Draft
            -add_text(text) { self.content.push_str(text) }
            -request_review() { self.state = State::PendingReview }
            -content() { "" }
        ,
        /// doc State::PendingReview
        PendingReview
            -approve() { self.state = State::Published }
            -content() { "" }
        ,
        Published
            -content() { &self.content }
    }
    } //impl_match!
}
