use blog::{Post, State};

pub fn main() {
    let mut post = Post::new();

    assert_eq!(post.add_text("I ate a salad for lunch today"), Ok("I ate a salad for lunch today"));

    assert_eq!(
        post.approve(),
        Err("For State::Draft method 'approve()' is not possible".to_string())
    );

    assert_eq!(post.request_review(), Ok(&State::PendingReview));
    assert_eq!(post.content(), "");

    assert_eq!(post.approve(), Ok(&State::Published));
    assert_eq!(post.content(), "I ate a salad for lunch today");
}

mod blog {


    pub struct Post {
        state: State,
        content: String,
    }

    methods_enum::impl_match!{
    impl Post {
        pub fn add_text(&mut self, text: &str) -> Result<&str, String>  ~{ match self.state }
        pub fn request_review(&mut self) -> Result<&State, String>      ~{ match self.state }
        pub fn reject(&mut self)                                        ~{ match self.state }
        pub fn approve(&mut self) -> Result<&State, String>             ~{ match self.state }
        pub fn content(&mut self) -> &str            ~{ let mut x = ""; match self.state; x }

        fn method_not_possible(&self, act: &str) -> String {
            format!("For State::{:?} method '{act}' is not possible", self.state)
        }

        pub fn new() -> Post {
            Post { state: State::Draft, content: String::new() }
        }
    }

    #[derive(Debug, PartialEq, Clone, Copy)]
    pub enum State {
        Draft:
            add_text(text) {
                self.content.push_str(text);
                Ok(&self.content)
            }
            request_review() {
                self.state = State::PendingReview;
                Ok(&self.state)
            }

            approve() { Err(self.method_not_possible("approve()")) }
        ,
        PendingReview:
            approve() {
                    self.state = State::Published;
                    Ok(&self.state)
            }
            reject() {
                self.state = State::Draft;
            }

            add_text() { Err(self.method_not_possible("add_text()")) }
            request_review() { Err(self.method_not_possible("request_review()")) }
            ,
        Published:
            content() { x = &self.content }

            add_text() { Err(self.method_not_possible("add_text()")) }
            approve() { Err(self.method_not_possible("approve()")) }
            request_review() { Err(self.method_not_possible("request_review()")) }
    }
    } // impl_match!
}
