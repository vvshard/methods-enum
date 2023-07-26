use blog::{Post, State};

pub fn main() {
    let mut post = Post::new();

    assert_eq!(post.add_text("I ate a salad for lunch today"), Ok(&State::Draft));

    assert_eq!(
        post.approve(),
        Err("For State::Draft method 'approve()' is not possible".to_string())
    );

    assert_eq!(post.request_review(), Ok(&State::PendingReview));
    assert_eq!(
        post.content(),
        Err("For State::PendingReview method 'content()' is not possible".to_string())
    );

    assert_eq!(post.approve(), Ok(&State::Published));
    assert_eq!(post.content(), Ok("I ate a salad for lunch today"));
}

mod blog {

    pub struct Post {
        state: State,
        content: String,
    }

    methods_enum::impl_match! {
    impl Post {
        /// Checking associated fn before signatures + doc comment
        pub fn new() -> Post {
            Post { state: State::Draft, content: String::new() }
        }

        pub fn add_text(&mut self, text: &str) -> Result<&State, String>    ~{ match self.state }
        pub fn request_review(&mut self) -> Result<&State, String>          ~{ match self.state }
        pub fn reject(&mut self)                                         ~{ match self.state {} }
        pub fn approve(&mut self) -> Result<&State, String>                 ~{ match self.state }
        pub fn content(&mut self) -> Result<&str, String>                   ~{ match self.state }

        fn method_not_possible(&self, act: &str) -> String {
            format!("For State::{:?} method '{act}' is not possible", self.state)
        }
    }


    #[derive(Debug, PartialEq, Clone, Copy)]
    pub enum State {
        Draft:
            add_text(text) {
                self.content.push_str(text);
                Ok(&self.state)
            }
            request_review() {
                self.state = State::PendingReview;
                Ok(&self.state)
            }

            approve() { Err(self.method_not_possible("approve()")) }
            content() { Err(self.method_not_possible("approve()")) }
        ,
        PendingReview:
            approve() {
                    self.state = State::Published;
                    Ok(&self.state)}
            reject() {  self.state = State::Draft }

            add_text() { Err(self.method_not_possible("add_text()")) }
            request_review() { Err(self.method_not_possible("request_review()")) }
            content() { Err(self.method_not_possible("content()")) }
            ,
        Published:
            content() { Ok(&self.content) }

            add_text() { Err(self.method_not_possible("add_text()")) }
            approve() { Err(self.method_not_possible("approve()")) }
            request_review() { Err(self.method_not_possible("request_review()")) }
    }
    }
}
