use blog::{Post, State};


pub fn main() {
    let mut post = Post::new();

    assert_eq!(post.add_text("I ate a salad for lunch today"), Ok("I ate a salad for lunch today"));
    assert_eq!(post.content(), "");

    assert_eq!(post.request_review(), Ok(&State::PendingReview { number_approvals: 0 }));
    assert_eq!(post.content(), "");

    assert_eq!(
        post.add_text("\nI'm hungry"),
        Err("For State::PendingReview { number_approvals: 0 } \
        method 'add_text()' is not possible"
            .to_string())
    );
    post.reject();
    assert_eq!(
        post.add_text("\nSecond time: I'm hungry!!"),
        Ok("I ate a salad for lunch today\nSecond time: I'm hungry!!")
    );
    assert_eq!(
        post.approve(),
        Err("For State::Draft method 'approve()' is not possible".to_string())
    );
    assert_eq!(post.request_review(), Ok(&State::PendingReview { number_approvals: 0 }));
    assert_eq!(post.approve(), Ok(&State::PendingReview { number_approvals: 1 }));
    assert_eq!(post.content(), "");
    assert_eq!(post.approve(), Ok(&State::Published));
    assert_eq!(post.content(), "I ate a salad for lunch today\nSecond time: I'm hungry!!");
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

        pub fn add_text(&mut self, text: &str) -> Result<&str, String>  ~{ match self.state }
        pub fn request_review(&mut self) -> Result<&State, String>      ~{ match self.state }
        pub fn reject(&mut self)                                        ~{ match self.state }
        pub fn approve(&mut self) -> Result<&State, String>             ~{ match self.state }
        pub fn content(&mut self) -> &str         ~{ let mut x = ""; match self.state {}; x }

        fn method_not_possible(&self, act: &str) -> String {
            format!("For State::{:?} method '{act}' is not possible", self.state)
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
                self.state = State::PendingReview { number_approvals: 0 };
                Ok(&self.state)
            }

            approve() { Err(self.method_not_possible("approve()")) }
        ,
        PendingReview { number_approvals: u32 }: { number_approvals: _approvals }
            approve() {
                if _approvals == 1 {
                    self.state = State::Published;
                    Ok(&self.state)
                } else {
                    self.state =
                        State::PendingReview { number_approvals: _approvals + 1 };
                    Ok(&self.state)
                }
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
    }
}
