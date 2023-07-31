pub fn main() {
    let mut post = blog::Post::new();

    let mut ext_content = "External content: ".to_string();

    post.add_text("I ate a salad for lunch today", &mut ext_content);
    assert_eq!("", post.content());
    assert_eq!("External content: I ate a salad for lunch today", ext_content);

    post.request_review();
    post.approve();
    assert_eq!("I ate a salad for lunch today", post.content());
}

mod blog {

    pub struct Post {
        state: State,
        content: String,
    }

    methods_enum::impl_match! {
    impl Post {
        pub fn add_text(&mut self, text: &str, ex_content: &mut String) ~{ match self.state {} }
        pub fn request_review(&mut self)                                ~{ match self.state {} }
        pub fn approve(&mut self)                                       ~{ match self.state {} }
        pub fn content(&mut self) -> &str                          ~{  match self.state { "" } }

        pub fn new() -> Post {
            Post { state: State::Draft, content: String::new() }
        }
    }

    enum State {
        Draft:
            add_text(text, ex_cont) {
                self.content.push_str(text);
                ex_content.push_str(text);
            }
            request_review() { self.state = State::PendingReview }
            ,
        PendingReview: approve() { self.state = State::Published },
        Published: content() { &self.content }
    }
    } // impl_match!
}
